use std::alloc::{alloc, Layout};
use std::collections::HashMap;
use std::future::Future;
use std::os::raw::c_void;
use std::pin::Pin;
use std::sync::Arc;
use std::{mem, os::windows::io::FromRawHandle, ptr};

use tokio::sync::Mutex;
use tokio::task;
use windows::core::HRESULT;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Console::ClosePseudoConsole;
use windows::Win32::System::Threading::CreateProcessW;
use windows::Win32::System::Threading::EXTENDED_STARTUPINFO_PRESENT;
use windows::Win32::System::Threading::LPPROC_THREAD_ATTRIBUTE_LIST;
use windows::Win32::System::Threading::PROCESS_INFORMATION;
use windows::Win32::System::Threading::PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE;
use windows::Win32::System::Threading::STARTUPINFOEXW;
use windows::Win32::System::Threading::STARTUPINFOW;
use windows::Win32::System::Threading::{
    InitializeProcThreadAttributeList, UpdateProcThreadAttribute, WaitForSingleObject, INFINITE,
};
use windows::Win32::{
    self,
    Foundation::HANDLE,
    Security::SECURITY_ATTRIBUTES,
    System::{
        Console::{CreatePseudoConsole, HPCON},
        Pipes::CreatePipe,
    },
};

use crate::process::{ProcessData, TerminalError, TerminalLike};
use crate::Vector2;

impl From<windows::core::Error> for TerminalError {
    fn from(error: windows::core::Error) -> Self {
        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(error);
        TerminalError::from(err)
    }
}

fn close(
    hpcon: HPCON,
    input_read: HANDLE,
    input_write: HANDLE,
    output_read: HANDLE,
    output_write: HANDLE,
    proc_info: PROCESS_INFORMATION,
) -> Result<(), TerminalError> {
    unsafe {
        ClosePseudoConsole(hpcon);
        CloseHandle(input_read)?;
        CloseHandle(output_write)?;
        CloseHandle(input_write)?;
        CloseHandle(output_read)?;
        CloseHandle(proc_info.hProcess)?;
        CloseHandle(proc_info.hThread)?;
    }

    Ok(())
}

struct ProcHandle {
    handle: HANDLE,
}
impl ProcHandle {
    fn handle(&self) -> HANDLE {
        self.handle
    }
}
unsafe impl Send for ProcHandle {}
unsafe impl Sync for ProcHandle {}

pub async fn spawn_interactive_process(
    program: &str,
    env: HashMap<String, String>,
    size: Vector2,
) -> windows::core::Result<ProcessData> {
    unsafe {
        let mut input_read: HANDLE = HANDLE::default();
        let mut input_write: HANDLE = HANDLE::default();
        let mut output_read: HANDLE = HANDLE::default();
        let mut output_write: HANDLE = HANDLE::default();

        let mut security_attributes = SECURITY_ATTRIBUTES {
            nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: ptr::null_mut(),
            bInheritHandle: false.into(),
        };

        CreatePipe(
            &mut input_read,
            &mut input_write,
            Some(&mut security_attributes),
            0,
        )?;
        CreatePipe(
            &mut output_read,
            &mut output_write,
            Some(&mut security_attributes),
            0,
        )?;

        let mut tty_size = Win32::System::Console::COORD::default();
        tty_size.X = size.x as i16;
        tty_size.Y = size.y as i16;
        let hpcon = CreatePseudoConsole(tty_size, input_read, output_write, 0)?;

        let mut attribute_list_size: usize = 0;
        let result = InitializeProcThreadAttributeList(None, 1, None, &mut attribute_list_size);
        if let Err(e) = result {
            // Error { code: HRESULT(0x8007007A), message: "The data area passed to a system call is too small." }
            if e.code() != HRESULT::from_win32(0x8007007A) {
                return Err(e);
            }
        }
        let layout = Layout::from_size_align(attribute_list_size, mem::size_of::<usize>()).unwrap();
        let attribute_list = alloc(layout);
        let attribute_list = LPPROC_THREAD_ATTRIBUTE_LIST(attribute_list as *mut std::ffi::c_void);
        InitializeProcThreadAttributeList(Some(attribute_list), 1, None, &mut attribute_list_size)?;
        UpdateProcThreadAttribute(
            attribute_list,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            Some(hpcon.0 as *mut c_void),
            mem::size_of::<HPCON>(),
            None,
            None,
        )?;

        let mut startup_info_ex = STARTUPINFOEXW {
            StartupInfo: STARTUPINFOW {
                cb: mem::size_of::<STARTUPINFOEXW>() as u32,
                hStdInput: input_read,
                hStdOutput: output_write,
                hStdError: output_write,
                ..mem::zeroed()
            },
            lpAttributeList: attribute_list,
            ..mem::zeroed()
        };

        let mut proc_info: PROCESS_INFORMATION = mem::zeroed();
        let program = format!("{}\0", program);
        let program =
            windows::core::PCWSTR::from_raw(program.encode_utf16().collect::<Vec<u16>>().as_ptr());

        CreateProcessW(
            Some(&program),
            None,
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT,
            None,
            None,
            &mut startup_info_ex.StartupInfo,
            &mut proc_info,
        )?;

        let reader = tokio::fs::File::from_std(std::fs::File::from_raw_handle(output_read.0));
        let writer = tokio::fs::File::from_std(std::fs::File::from_raw_handle(input_write.0));

        let is_closed = Arc::new(Mutex::new(false));
        let mut pty = WinPTY {
            hpcon,
            input_read,
            input_write,
            output_read,
            output_write,
            proc_info,
            size,
            done_future: None,
            is_closed: is_closed.clone(),
        };

        let done_future = async move {
            let handle = ProcHandle {
                handle: pty.proc_info.hProcess,
            };
            task::spawn_blocking(move || {
                let _event: Win32::Foundation::WAIT_EVENT =
                    WaitForSingleObject(handle.handle(), INFINITE);
            })
            .await?;

            pty.release().await?;

            Ok(())
        };

        Ok(ProcessData {
            stdin: Box::new(writer),
            stdout: Box::new(reader),
            terminal: Box::new(WinPTY {
                hpcon,
                input_read,
                input_write,
                output_read,
                output_write,
                proc_info,
                size,
                done_future: Some(Box::pin(done_future)),
                is_closed,
            }),
        })
    }
}

struct WinPTY {
    hpcon: HPCON,
    input_read: HANDLE,
    input_write: HANDLE,
    output_read: HANDLE,
    output_write: HANDLE,
    proc_info: PROCESS_INFORMATION,
    size: Vector2,
    done_future:
        Option<Pin<Box<dyn std::future::Future<Output = Result<(), TerminalError>> + Send>>>,
    is_closed: Arc<Mutex<bool>>,
}

unsafe impl Send for WinPTY {}
unsafe impl Sync for WinPTY {}

async fn close_pty(pty: &mut WinPTY) -> Result<(), TerminalError> {
    let mut is_closed = pty.is_closed.lock().await;
    if *is_closed {
        return Ok(());
    }
    *is_closed = true;
    close(
        pty.hpcon,
        pty.input_read,
        pty.input_write,
        pty.output_read,
        pty.output_write,
        pty.proc_info,
    )
}

impl TerminalLike for WinPTY {
    fn take_done_future(
        &mut self,
    ) -> Option<Pin<Box<dyn std::future::Future<Output = Result<(), TerminalError>> + Send>>> {
        self.done_future.take()
    }

    fn release<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), TerminalError>> + 'a + Send>> {
        let future = async { close_pty(self).await };

        Box::pin(future)
    }

    fn set_size(&mut self, size: crate::canvas::Vector2) -> Result<(), TerminalError> {
        unsafe {
            let mut tty_size = Win32::System::Console::COORD::default();
            tty_size.X = size.x as i16;
            tty_size.Y = size.y as i16;
            let result = windows::Win32::System::Console::ResizePseudoConsole(self.hpcon, tty_size);
            if let Err(e) = result {
                tracing::error!("Error resizing pty: {:?}", e);
            }
            self.size = size;
        }

        Ok(())
    }

    fn size(&self) -> crate::canvas::Vector2 {
        self.size
    }
}

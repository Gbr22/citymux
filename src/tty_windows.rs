use std::alloc::{alloc, Layout};
use std::os::raw::c_void;
use std::{mem, os::windows::io::FromRawHandle, ptr};

use windows::core::HRESULT;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Console::ClosePseudoConsole;
use windows::Win32::System::Threading::{InitializeProcThreadAttributeList, UpdateProcThreadAttribute};
use windows::Win32::System::Threading::EXTENDED_STARTUPINFO_PRESENT;
use windows::Win32::System::Threading::LPPROC_THREAD_ATTRIBUTE_LIST;
use windows::Win32::System::Threading::PROCESS_INFORMATION;
use windows::Win32::System::Threading::PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE;
use windows::Win32::System::Threading::STARTUPINFOEXW;
use windows::Win32::System::Threading::STARTUPINFOW;
use windows::Win32::{self, Foundation::HANDLE, Security::SECURITY_ATTRIBUTES, System::{Console::{CreatePseudoConsole, HPCON}, Pipes::CreatePipe}};
use windows::Win32::System::Threading::CreateProcessW;

use crate::process::{ProcessData, ProcessDataDyn};

pub async fn spawn_interactive_process(program: &str) -> windows::core::Result<ProcessData> {
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
            &mut input_read, &mut input_write, Some(&mut security_attributes), 0)?;
        CreatePipe(&mut output_read, &mut output_write, Some(&mut security_attributes), 0)?;

        let mut tty_size = Win32::System::Console::COORD::default();
        tty_size.X = 80;
        tty_size.Y = 24;
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
        InitializeProcThreadAttributeList(Some(attribute_list), 1, None, &mut attribute_list_size).expect("Alma");
        UpdateProcThreadAttribute(attribute_list, 0, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize, Some(hpcon.0 as *mut c_void), mem::size_of::<HPCON>() as usize, None, None).expect("Körte");

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
        let program = windows::core::PCWSTR::from_raw(program.encode_utf16().collect::<Vec<u16>>().as_ptr());

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

        Ok(ProcessData { stdin: Box::new(writer), stdout: Box::new(reader), dyn_data: Box::new(WinProcessData {
            hpc: hpcon,
            input_read,
            input_write,
            output_read,
            output_write,
            proc_info,
        }) })
    }
}

struct WinProcessData {
    hpc: HPCON,
    input_read: HANDLE,
    input_write: HANDLE,
    output_read: HANDLE,
    output_write: HANDLE,
    proc_info: PROCESS_INFORMATION,
}

impl ProcessDataDyn for WinProcessData {
    fn release(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            ClosePseudoConsole(self.hpc);
            CloseHandle(self.input_read)?;
            CloseHandle(self.input_write)?;
            CloseHandle(self.output_read)?;
            CloseHandle(self.output_write)?;
            CloseHandle(self.proc_info.hProcess)?;
            CloseHandle(self.proc_info.hThread)?;
        }

        Ok(())
    }
}

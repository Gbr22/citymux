use std::{fs, future::Future, pin::Pin, process::Stdio, sync::Arc};

use crossterm_winapi::{ConsoleMode, Handle};
use escape_codes::{get_cursor_position, DisableConcealMode, EnableComprehensiveKeyboardHandling, EnterAlternateScreenBuffer, MoveCursor, RequestCursorPosition};
use tokio::{io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Stdin}, sync::{futures, Mutex, RwLock}, task::JoinSet};
use which::which;
use winapi::{shared::minwindef::DWORD, um::wincon::{ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_INPUT}};
use tty_windows::spawn_interactive_process;

mod escape_codes;
mod tty_windows;
mod process;

struct Size {
    height: usize,
    width: usize,
}

struct Process {
    pub stdout: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdin: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
}

struct State {
    pub stdin: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdout: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub size: Arc<RwLock<Size>>,
    pub processes: Arc<Mutex<Vec<Arc<Mutex<Process>>>>>,
    pub futures: Arc<Mutex<Vec<Pin<Box<dyn std::future::Future<Output = ()>>>>>>,
}

impl State {
    pub async fn get_active_process(&self) -> Result<Option<Arc<Mutex<Process>>>, Box<dyn std::error::Error>> {
        let lock = self.processes.lock().await;
        let first = lock.first();
        Ok(first.cloned())
    }
}

impl State {
    fn new(input: impl AsyncRead + Unpin + Send + Sync + 'static, output: impl AsyncWrite + Unpin + Send + Sync + 'static) -> Self {
        State {
            stdin: Arc::new(Mutex::new(input)),
            stdout: Arc::new(Mutex::new(output)),
            size: Arc::new(RwLock::new(Size { height: 0, width: 0 })),
            processes: Arc::new(Mutex::new(Vec::new())),
            futures: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[derive(Clone)]
struct StateContainer {
    state: Arc<State>,
}

impl StateContainer {
    fn new(state: State) -> Self {
        let state = Arc::new(state);
        StateContainer { state }
    }
    fn get_state(&self) -> Arc<State> {
        self.state.clone()
    }
}

async fn handle_stdin(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let stdin = state_container.get_state().stdin.clone();
        let mut stdin = stdin.lock().await;
        let result = stdin.read_u8().await?;

        if result == b'q' {
            std::process::exit(0);
        }

        let active_process = state_container.get_state().get_active_process().await?;
        if let Some(active_process) = active_process {
            let process = active_process.lock().await;
            let mut stdin = process.stdin.lock().await;
            stdin.write(&[result]).await?;
            if result == b'\r' {
                stdin.write(&[b'\n']).await?;
            }
            stdin.flush().await?;
        }
    }
}

async fn handle_process(state_container: StateContainer, process: Arc<Mutex<Process>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut escape_distance: Option<usize> = None;
    let mut is_csi = false;
    let mut is_osc = false;
    let mut collected = Vec::new();
    loop {
        let stdout = {
            let process = process.lock().await;
            process.stdout.clone()
        };
        let mut stdout = stdout.lock().await;
        let mut buf = [0; 1];
        let n = stdout.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        let byte = buf[0];
        if let Some(escape_distance_value) = escape_distance {
            escape_distance = Some(escape_distance_value + 1);
        }
        if byte == 0x1b {
            escape_distance = Some(0);
            continue;
        }

        let csi_final_bytes = r"@[\]^_`{|}~";
        let is_csi_final_byte = (byte as char).is_alphabetic() || csi_final_bytes.as_bytes().contains(&byte);
        if is_csi && is_csi_final_byte {
            is_csi = false;
            escape_distance = None;
            collected.push(byte);
            print!("[CSI:{:?}]", String::from_utf8_lossy(&collected));
            collected = Vec::new();
            continue;
        }
        if is_csi {
            collected.push(byte);
            continue;
        }
        const ST_C1: u8 = 0x9c;
        const BEL: u8 = 0x07;
        if byte == ST_C1 || (escape_distance == Some(1) && byte == b'\\') || byte == BEL {
            is_osc = false;
            print!("[OSC:{:?}]", String::from_utf8_lossy(&collected));
            collected = Vec::new();
            continue;
        }
        
        if is_osc {
            collected.push(byte);
            continue;
        }
        
        if byte == 0x9b || (escape_distance == Some(1) && byte == b'[') { // CSI
            is_csi = true;
            continue;
        }
        if byte == 0x9d || (escape_distance == Some(1) && byte == b']') { // OSC
            print!("[OSC-E:{:?}]", byte);
            is_osc = true;
            continue;
        }
        
    
        state_container.get_state().stdout.lock().await.write(&buf[..n]).await?;
    }
}

async fn handle_stdout(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let new_futures = {
            let futures = state_container.get_state().futures.clone();
            let mut futures = futures.lock().await;
            let len = futures.len();
            let result: Vec<Pin<Box<dyn Future<Output = ()>>>> = futures.drain(0..len).collect();
            result
        };
        
        for process in new_futures {
            process.await;
        }
    }
}

fn enable_raw_mode() -> Result<(), Box<dyn std::error::Error>> {
    let console_mode = ConsoleMode::from(Handle::current_in_handle()?);
    let current_mode = console_mode.mode()?;
    let new_mode = (current_mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT)) | (ENABLE_VIRTUAL_TERMINAL_INPUT);
    console_mode.set_mode(new_mode)?;

    Ok(())
}

async fn draw_main_menu(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;

    let stdin = state_container.get_state().stdin.clone();
    let mut stdin = stdin.lock().await;
    let stdout = state_container.get_state().stdout.clone();
    let mut stdout = stdout.lock().await;
    stdout.write(EnableComprehensiveKeyboardHandling::default().into()).await?;
    stdout.write(EnterAlternateScreenBuffer::default().into()).await?;
    stdout.write(EnableComprehensiveKeyboardHandling::default().into()).await?;
    stdout.write(&Into::<Vec<u8>>::into(MoveCursor::new(0, 0))).await?;
    stdout.flush().await?;

    let (width, height) = crossterm::terminal::size()?;
    let size = state_container.get_state().size.clone();
    {
        let mut size = size.write().await;
        size.height = height as usize;
        size.width = width as usize;
    }

    Ok(())
}

async fn spawn_process(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    let program = which("bash")?.to_string_lossy().to_string();
    let result = spawn_interactive_process(&program).await?;
    let process = Process {
        stdin: Arc::new(Mutex::new(result.stdin)),
        stdout: Arc::new(Mutex::new(result.stdout)),
    };

    let processes = state_container.get_state().processes.clone();
    {
        let mut processes = processes.lock().await;
        let process = Arc::new(Mutex::new(process));
        let future = {
            let process = process.clone();
            let state_container = state_container.clone();
            async move {
                let result = handle_process(state_container, process).await;
                if let Err(e) = result {
                    eprintln!("Error: {:?}", e);
                }
            }
        };
        
        processes.push(process);
        {
            let futures = state_container.get_state().futures.clone();
            let mut futures = futures.lock().await;
            futures.push(Box::pin(future));
        }
    }

    Ok(())
}

async fn run(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    draw_main_menu(state_container.clone()).await?;
    spawn_process(state_container.clone()).await?;
    let results = tokio::join!(
        handle_stdin(state_container.clone()),
        handle_stdout(state_container.clone()),
    );
    results.0?;
    results.1?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let state_container = StateContainer::new(State::new(io::stdin(), io::stdout()));
    if let Err(e) = run(state_container).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

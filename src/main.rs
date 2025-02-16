use std::{fs, pin::Pin, process::Stdio, sync::Arc};

use crossterm_winapi::{ConsoleMode, Handle};
use escape_codes::{get_cursor_position, DisableConcealMode, EnableComprehensiveKeyboardHandling, EnterAlternateScreenBuffer, MoveCursor, RequestCursorPosition};
use tokio::{io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Stdin}, sync::{Mutex, RwLock}};
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
    pub stdout: Arc<Mutex<dyn AsyncRead + Unpin>>,
    pub stdin: Arc<Mutex<dyn AsyncWrite + Unpin>>,
}

struct State {
    pub stdin: Arc<Mutex<dyn AsyncRead + Unpin>>,
    pub stdout: Arc<Mutex<dyn AsyncWrite + Unpin>>,
    pub size: Arc<RwLock<Size>>,
    pub processes: Arc<Mutex<Vec<Arc<Mutex<Process>>>>>,
}

impl State {
    pub async fn get_active_process(&self) -> Result<Option<Arc<Mutex<Process>>>, Box<dyn std::error::Error>> {
        let lock = self.processes.lock().await;
        
        Ok(lock.first().cloned())
    }
}

impl State {
    fn new(input: impl AsyncRead + Unpin + 'static, output: impl AsyncWrite + Unpin + 'static) -> Self {
        State {
            stdin: Arc::new(Mutex::new(input)),
            stdout: Arc::new(Mutex::new(output)),
            size: Arc::new(RwLock::new(Size { height: 0, width: 0 })),
            processes: Arc::new(Mutex::new(Vec::new())),
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
        println!("Input: {:?} {:?}", result, char::from_u32(result as u32).unwrap_or(' '));

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

async fn handle_stdout(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let processes = state_container.get_state().processes.clone();
        let processes = {
            let processes = processes.lock().await;
            processes.clone()
        };
        for process in processes.iter() {
            let stdout = {
                let process = process.lock().await;
                process.stdout.clone()
            };
            let mut stdout = stdout.lock().await;
            let mut buf = [0; 1024];
            let n = stdout.read(&mut buf).await?;
            if n == 0 {
                return Ok(());
            }

            state_container.get_state().stdout.lock().await.write(&buf[..n]).await?;
        }
    }
}

async fn open_program(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
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
        processes.push(Arc::new(Mutex::new(process)));
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

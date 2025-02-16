use std::{collections::HashMap, fs, future::Future, pin::Pin, process::Stdio, sync::Arc, time::Duration};

use crossterm_winapi::{ConsoleMode, Handle};
use escape_codes::{get_cursor_position, DisableConcealMode, EnableComprehensiveKeyboardHandling, EnterAlternateScreenBuffer, MoveCursor, RequestCursorPosition};
use spawn::spawn_process;
use tokio::{io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Stdin}, sync::{futures, Mutex, RwLock}, task::JoinSet};
use winapi::{shared::minwindef::DWORD, um::wincon::{ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_INPUT}};

mod escape_codes;
mod tty_windows;
mod process;
mod spawn;
mod tty;

#[derive(Clone, Copy)]
struct Vector2 {
    x: usize,
    y: usize,
}

struct Color {}

struct Cell {
    value: char,
    color: Color,
}

struct Process {
    pub stdout: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdin: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub size: Vector2,
    pub buffer: Vec<Cell>,
    pub cursor: Vector2,
}

struct State {
    pub stdin: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdout: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub size: Arc<RwLock<Vector2>>,
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
            size: Arc::new(RwLock::new(Vector2 { y: 0, x: 0 })),
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

const csi_final_bytes: &str = r"@[\]^_`{|}~";

async fn write_input(state_container: StateContainer, data: &[u8], flush: bool) -> Result<(), Box<dyn std::error::Error>> {
    let active_process = state_container.get_state().get_active_process().await?;
    if let Some(active_process) = active_process {
        let process = active_process.lock().await;
        let mut stdin = process.stdin.lock().await;
        stdin.write(data).await?;
        if flush {
            stdin.flush().await?;
        }
    }

    Ok(())
}

async fn write_output(state_container: StateContainer, data: &[u8], flush: bool) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = state_container.get_state().stdout.clone();
    let mut stdout = stdout.lock().await;
    stdout.write(data).await?;
    if flush {
        stdout.flush().await?;   
    }

    Ok(())
}

async fn write_output_str(state_container: StateContainer, data: impl AsRef<str>, flush: bool) -> Result<(), Box<dyn std::error::Error>> {
    write_output(state_container, data.as_ref().as_bytes(), flush).await?;

    Ok(())
}

async fn handle_stdin(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    let mut escape_distance: Option<usize> = None;
    let mut is_csi = false;
    let mut is_osc = false;
    let mut collected = Vec::new();
    
    loop {
        let stdin = state_container.get_state().stdin.clone();
        let mut stdin = stdin.lock().await;

        let mut buf = [0; 1];
        let n = stdin.read(&mut buf).await?;
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
        let is_csi_final_byte = (byte as char).is_alphabetic() || csi_final_bytes.as_bytes().contains(&byte);
        if is_csi && is_csi_final_byte {
            is_csi = false;
            escape_distance = None;
            collected.push(byte);
            print!("[IN-CSI:{:?}]", String::from_utf8_lossy(&collected));
            let prefix = "\x1b[".as_bytes();
            let concat: Vec<u8> = prefix.iter().chain(collected.iter()).map(|e|e.to_owned()).collect();
            write_input(state_container.clone(), &concat, true).await?;

            collected = Vec::new();
            continue;
        }
        if is_csi {
            collected.push(byte);
            continue;
        }

        if byte == 0x9b || (escape_distance == Some(1) && byte == b'[') { // CSI
            is_csi = true;
            continue;
        }

        if byte == b'q' {
            std::process::exit(0);
        }

        write_input(state_container.clone(), &buf[..n], true).await?;
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
        
        let is_csi_final_byte = (byte as char).is_alphabetic() || csi_final_bytes.as_bytes().contains(&byte);
        if is_csi && is_csi_final_byte {
            is_csi = false;
            escape_distance = None;
            collected.push(byte);
            write_output_str(state_container.clone(), format!("[CSI:{:?}]", String::from_utf8_lossy(&collected)), true).await?;
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
            write_output_str(state_container.clone(), format!("[OSC:{:?}]", String::from_utf8_lossy(&collected)), true).await?;
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
            is_osc = true;
            continue;
        }
        
    
        {
            let state = state_container.get_state();
            let mut stdout = state.stdout.lock().await;
            stdout.write(&buf[..n]).await?;
            stdout.flush().await?;
        }
        
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

async fn init_screen(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
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
        size.y = height as usize;
        size.x = width as usize;
    }

    Ok(())
}

async fn draw(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

async fn draw_loop(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        draw(state_container.clone()).await?;
    }
}



async fn run(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    init_screen(state_container.clone()).await?;
    spawn_process(state_container.clone(), Vector2 {
        x: 40,
        y: 25
    }).await?;
    let results = tokio::join!(
        handle_stdin(state_container.clone()),
        handle_stdout(state_container.clone()),
        draw_loop(state_container.clone())
    );
    results.0?;
    results.1?;
    results.2?;

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

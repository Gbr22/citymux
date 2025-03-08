use std::{collections::HashMap, fs::{self, remove_file, OpenOptions}, future::Future, pin::Pin, process::Stdio, sync::{atomic::AtomicUsize, Arc}, time::Duration};

use canvas::{Canvas, TerminalInfo, Vector2};
use crossterm_winapi::{ConsoleMode, Handle};
use draw::draw_loop;
use encoding::CSI_FINAL_BYTES;
use escape_codes::SetAlternateScreenBuffer;
use exit::exit;
use process::TerminalLike;
use span::{Node, NodeData};
use spawn::{create_process, kill_active_span};
use tokio::{io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Stdin}, select, sync::{futures, Mutex, RwLock}, task::JoinSet, time::{timeout, Instant}};
use winapi::{shared::minwindef::DWORD, um::wincon::{ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_INPUT}};

mod escape_codes;
mod tty_windows;
mod process;
mod spawn;
mod encoding;
mod tty;
mod canvas;
mod draw;
mod span;
mod exit;
mod layout;

#[cfg(test)]
mod test;

struct Process {
    pub stdout: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdin: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub terminal_info: Arc<Mutex<TerminalInfo>>,
    pub terminal: Arc<Mutex<Box<dyn TerminalLike>>>,
    pub span_id: usize,
}

struct State {
    pub stdin: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdout: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub size: Arc<RwLock<Vector2>>,
    pub processes: Arc<Mutex<Vec<Arc<Mutex<Process>>>>>,
    pub process_channel: Arc<Mutex<Option<tokio::sync::mpsc::Sender<Pin<Box<dyn std::future::Future<Output = ()> + Send>>>>>>,
    pub last_canvas: Arc<Mutex<Canvas>>,
    pub root_node: Arc<Mutex<Option<Node>>>,
    pub span_id_counter: AtomicUsize,
    pub active_id: AtomicUsize,
}

impl State {
    pub async fn get_active_process(&self) -> Result<Option<Arc<Mutex<Process>>>, Box<dyn std::error::Error>> {
        let active_process_id = self.active_id.load(std::sync::atomic::Ordering::Relaxed);
        let lock = self.processes.lock().await;
        for process in lock.iter() {
            let lock = process.lock().await;
            if lock.span_id == active_process_id {
                return Ok(Some(process.clone()));
            }
        }

        Ok(None)
    }
}

impl State {
    fn new(input: impl AsyncRead + Unpin + Send + Sync + 'static, output: impl AsyncWrite + Unpin + Send + Sync + 'static) -> Self {
        State {
            stdin: Arc::new(Mutex::new(input)),
            stdout: Arc::new(Mutex::new(output)),
            size: Arc::new(RwLock::new(Vector2::default())),
            processes: Arc::new(Mutex::new(Vec::new())),
            process_channel: Arc::new(Mutex::new(None)),
            last_canvas: Arc::new(Mutex::new(Canvas::new(Vector2::new(0, 0)))),
            root_node: Arc::new(Mutex::new(None)),
            span_id_counter: AtomicUsize::new(0),
            active_id: AtomicUsize::new(0),
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

async fn handle_loop<F, R>(mut func: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() -> R,
        R: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>
{
    loop {
        let result = func().await;
        if let Err(e) = result {
            tracing::error!("Error: {:?}", e);
        }
    }
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

        let timeout_result = timeout(Duration::from_millis(100), stdin.read(&mut buf)).await;
        let Ok(result) = timeout_result else {
            if escape_distance == Some(0) {
                let data = "\x1b".as_bytes();
                write_input(state_container.clone(), data, true).await?;
                escape_distance = None;
            }
            continue;
        };
        let n: usize = result?;
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
        let is_csi_final_byte = (byte as char).is_alphabetic() || CSI_FINAL_BYTES.as_bytes().contains(&byte);
        if is_csi && is_csi_final_byte {
            is_csi = false;
            escape_distance = None;
            collected.push(byte);
            let collected_move = collected;
            collected = Vec::new();
            let collected = collected_move;


            let string = String::from_utf8_lossy(&collected).to_string();
            let first_byte = string.as_bytes().first().unwrap_or(&0).to_owned();
            let is_application_key_mode_enabled = {
                let process = state_container.get_state().get_active_process().await?;
                let Some(process) = process else {
                    continue;
                };
                let process = process.lock().await;
                let terminal_info = process.terminal_info.lock().await;
                
                terminal_info.application_key_mode()
            };
            if is_application_key_mode_enabled {
                if "ABCDHF".as_bytes().contains(&first_byte) {
                    let new_string = format!("\x1bO{}", first_byte as char);
                    write_input(state_container.clone(), new_string.as_bytes(), true).await?;
                    continue;
                }
            }
            let prefix = "\x1b[".as_bytes();
            let concat: Vec<u8> = prefix.iter().chain(collected.iter()).map(|e|e.to_owned()).collect();
            write_input(state_container.clone(), &concat, true).await?;
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

        if byte == b'q' && escape_distance == Some(1) {
            kill_active_span(state_container.clone()).await?;
            continue;
        }
        if byte == b'n' && escape_distance == Some(1) {
            create_process(state_container.clone()).await?;
            continue;
        }

        tracing::debug!("[IN:{:?}:{:?}]", byte, byte as char);
        write_input(state_container.clone(), &buf[..n], true).await?;
    }
}

async fn init_proc_handler(state_container: StateContainer) -> Result<tokio::sync::mpsc::Receiver<Pin<Box<dyn Future<Output = ()> + Send>>>, Box<dyn std::error::Error>> {
    let rx: tokio::sync::mpsc::Receiver<Pin<Box<dyn Future<Output = ()> + Send>>> = {
        let state = state_container.get_state();
        let mut process_channel = state.process_channel.lock().await;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        *process_channel = Some(tx);

        rx
    };

    Ok(rx)
}

async fn handle_child_processes(state_container: StateContainer, rx: Arc<Mutex<tokio::sync::mpsc::Receiver<Pin<Box<dyn Future<Output = ()> + Send>>>>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut rx = rx.lock().await;
    let mut join_set = JoinSet::new();
    loop {
        tokio::select! {
            Some(task) = rx.recv() => {
                join_set.spawn(task);
            }
            Some(result) = join_set.join_next() => {
                if let Err(e) = result {
                    tracing::error!("Error in child: {:?}", e);
                }
            }
            else => {
                tracing::error!("No more tasks to join");
            },
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

    let stdout = state_container.get_state().stdout.clone();
    let mut stdout = stdout.lock().await;
    stdout.write(SetAlternateScreenBuffer::enable().into()).await?;
    stdout.flush().await?;

    Ok(())
}

async fn run(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    init_screen(state_container.clone()).await?;
    let rx = init_proc_handler(state_container.clone()).await?;
    let rx = Arc::new(Mutex::new(rx));
    let stdout_handler = handle_loop(||handle_child_processes(state_container.clone(), rx.clone()));
    create_process(state_container.clone()).await?;
    let results = tokio::join!(
        handle_loop(||handle_stdin(state_container.clone())),
        stdout_handler,
        handle_loop(||draw_loop(state_container.clone())),
    );
    results.0?;
    results.1?;
    results.2?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).with_ansi(false).with_writer(|| {
        OpenOptions::new().append(true).create(true).open("log.txt").expect("Failed to open log file")
    }).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    tracing::info!("Starting up");

    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("Panic at {:?}: {:?}", info.location(), info.payload());
        std::process::exit(1);
    }));

    let state_container = StateContainer::new(State::new(io::stdin(), io::stdout()));
    if let Err(e) = run(state_container).await {
        tracing::error!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

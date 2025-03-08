use std::{
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc},
};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Mutex, RwLock},
};

use crate::{
    canvas::{Canvas, TerminalInfo, Vector2},
    process::TerminalLike,
    span::Node,
};

pub struct Process {
    pub stdout: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdin: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub terminal_info: Arc<Mutex<TerminalInfo>>,
    pub terminal: Arc<Mutex<Box<dyn TerminalLike>>>,
    pub span_id: usize,
}

pub struct State {
    pub stdin: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdout: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub size: Arc<RwLock<Vector2>>,
    pub processes: Arc<Mutex<Vec<Arc<Mutex<Process>>>>>,
    pub process_channel: Arc<
        Mutex<
            Option<
                tokio::sync::mpsc::Sender<Pin<Box<dyn std::future::Future<Output = ()> + Send>>>,
            >,
        >,
    >,
    pub last_canvas: Arc<Mutex<Canvas>>,
    pub root_node: Arc<Mutex<Option<Node>>>,
    pub span_id_counter: AtomicUsize,
    pub active_id: AtomicUsize,
}

impl State {
    pub async fn get_active_process(
        &self,
    ) -> Result<Option<Arc<Mutex<Process>>>, Box<dyn std::error::Error>> {
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
    pub fn new(
        input: impl AsyncRead + Unpin + Send + Sync + 'static,
        output: impl AsyncWrite + Unpin + Send + Sync + 'static,
    ) -> Self {
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
pub struct StateContainer {
    state: Arc<State>,
}

impl StateContainer {
    pub fn new(state: State) -> Self {
        let state = Arc::new(state);
        StateContainer { state }
    }
    pub fn get_state(&self) -> Arc<State> {
        self.state.clone()
    }
}

use std::{
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc},
};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Mutex, RwLock},
};

use crate::{
    canvas::{Canvas, Rect, TerminalInfo, Vector2},
    draw::DrawMessage,
    layout::get_span_dimensions,
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
    pub draw_channel: Arc<Mutex<Option<tokio::sync::mpsc::Sender<DrawMessage>>>>,
    pub last_canvas: Arc<Mutex<Canvas>>,
    pub root_node: Arc<Mutex<Option<Node>>>,
    pub span_id_counter: AtomicUsize,
    pub current_mouse_position: Arc<RwLock<Vector2>>,
    pub active_id: AtomicUsize,
}

impl State {
    pub async fn active_process(&self) -> Option<Arc<Mutex<Process>>> {
        let active_process_id = self.active_id.load(std::sync::atomic::Ordering::Relaxed);
        let lock = self.processes.lock().await;
        for process in lock.iter() {
            let lock = process.lock().await;
            if lock.span_id == active_process_id {
                return Some(process.clone());
            }
        }

        None
    }
    pub async fn active_terminal_info(&self) -> Option<Arc<Mutex<TerminalInfo>>> {
        let active_process = self.active_process().await?;
        let terminal_info = { active_process.lock().await.terminal_info.clone() };

        Some(terminal_info)
    }
    pub async fn application_keypad_mode(&self) -> Option<bool> {
        let terminal_info = self.active_terminal_info().await?;
        let terminal_info = terminal_info.lock().await;
        Some(terminal_info.application_keypad_mode())
    }
    pub async fn get_span_dimensions(&self, span_id: usize) -> Option<Rect> {
        let root_node = self.root_node.lock().await;
        let root_node = root_node.as_ref()?;
        let size = self.size.read().await.to_owned();
        get_span_dimensions(root_node, span_id, size)
    }
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
            draw_channel: Arc::new(Mutex::new(None)),
            last_canvas: Arc::new(Mutex::new(Canvas::new(Vector2::new(0, 0)))),
            root_node: Arc::new(Mutex::new(None)),
            span_id_counter: AtomicUsize::new(0),
            active_id: AtomicUsize::new(0),
            current_mouse_position: Arc::new(RwLock::new(Vector2::default())),
        }
    }
    pub fn set_active_span(&self, span_id: usize) {
        self.active_id
            .store(span_id, std::sync::atomic::Ordering::Relaxed)
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
    pub fn state(&self) -> Arc<State> {
        self.state.clone()
    }
}

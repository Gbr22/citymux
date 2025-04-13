use std::{
    collections::HashMap, ops::Deref, pin::Pin, sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    }
};

use renterm::{canvas::Canvas, rect::Rect, vector::Vector2};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Mutex, RwLock},
};

use crate::{
    args::CliArgs, config::Config, draw::DrawMessage, layout::get_span_dimensions,
    process::TerminalLike, span::Node, term::TerminalInfo,
};

pub struct Process {
    pub stdout: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdin: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub terminal_info: Arc<Mutex<TerminalInfo>>,
    pub terminal: Arc<Mutex<Box<dyn TerminalLike>>>,
    pub span_id: usize,
}

pub struct State {
    pub args: CliArgs,
    pub config: Arc<RwLock<Config>>,
    pub stdin: Arc<Mutex<dyn AsyncRead + Unpin + Send + Sync>>,
    pub stdout: Arc<Mutex<dyn AsyncWrite + Unpin + Send + Sync>>,
    pub size: Arc<RwLock<Vector2>>,
    pub processes: Arc<RwLock<Vec<Arc<Mutex<Process>>>>>,
    pub process_channel: Arc<
        Mutex<
            Option<
                tokio::sync::mpsc::Sender<Pin<Box<dyn std::future::Future<Output = ()> + Send>>>,
            >,
        >,
    >,
    pub draw_channel: Arc<Mutex<Option<tokio::sync::mpsc::Sender<DrawMessage>>>>,
    canvas_1: Arc<Mutex<Canvas>>,
    canvas_2: Arc<Mutex<Canvas>>,
    canvas_toggle: AtomicBool,
    pub root_node: Arc<RwLock<Option<Node>>>,
    pub span_id_counter: AtomicUsize,
    pub current_mouse_position: Arc<RwLock<Vector2>>,
    pub current_mouse_buttons: Arc<RwLock<HashMap<u8, bool>>>,
    pub active_id: AtomicUsize,
    pub draw_lock: Arc<Mutex<()>>,
}

impl State {
    pub fn get_last_canvas(&self) -> Arc<Mutex<Canvas>> {
        if self
            .canvas_toggle
            .load(std::sync::atomic::Ordering::Relaxed)
            == true
        {
            self.canvas_1.clone()
        } else {
            self.canvas_2.clone()
        }
    }
    pub fn get_current_canvas(&self) -> Arc<Mutex<Canvas>> {
        if self
            .canvas_toggle
            .load(std::sync::atomic::Ordering::Relaxed)
            == false
        {
            self.canvas_1.clone()
        } else {
            self.canvas_2.clone()
        }
    }
    pub fn swap_canvas(&self) {
        self.canvas_toggle.store(
            !self
                .canvas_toggle
                .load(std::sync::atomic::Ordering::Relaxed),
            std::sync::atomic::Ordering::Relaxed,
        );
    }
    pub async fn active_process(&self) -> Option<Arc<Mutex<Process>>> {
        let active_process_id = self.active_id.load(std::sync::atomic::Ordering::Relaxed);
        let lock = self.processes.read().await;
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
        let root_node = self.root_node.read().await;
        let root_node = root_node.as_ref()?;
        let size = self.size.read().await.to_owned();
        get_span_dimensions(root_node, span_id, size)
    }
    pub fn new(
        args: CliArgs,
        config: Config,
        input: impl AsyncRead + Unpin + Send + Sync + 'static,
        output: impl AsyncWrite + Unpin + Send + Sync + 'static,
    ) -> Self {
        State {
            args,
            config: Arc::new(RwLock::new(config)),
            stdin: Arc::new(Mutex::new(input)),
            stdout: Arc::new(Mutex::new(output)),
            size: Arc::new(RwLock::new(Vector2::null())),
            processes: Arc::new(RwLock::new(Vec::new())),
            process_channel: Arc::new(Mutex::new(None)),
            draw_channel: Arc::new(Mutex::new(None)),
            canvas_1: Arc::new(Mutex::new(Canvas::new(Vector2::new(0, 0)))),
            canvas_2: Arc::new(Mutex::new(Canvas::new(Vector2::new(0, 0)))),
            canvas_toggle: AtomicBool::new(false),
            root_node: Arc::new(RwLock::new(None)),
            span_id_counter: AtomicUsize::new(0),
            active_id: AtomicUsize::new(0),
            current_mouse_position: Arc::new(RwLock::new(Vector2::null())),
            current_mouse_buttons: Arc::new(RwLock::new(HashMap::new())),
            draw_lock: Arc::new(Mutex::new(())),
        }
    }
    pub fn set_active_span(&self, span_id: usize) {
        self.active_id
            .store(span_id, std::sync::atomic::Ordering::Relaxed)
    }
    pub async fn set_mouse_position(&self, position: impl Into<Vector2>) {
        let mut lock = self.current_mouse_position.write().await;
        *lock = position.into();
    }
    pub async fn set_size(&self, size: impl Into<Vector2>) {
        let mut lock = self.size.write().await;
        *lock = size.into();
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

impl Deref for StateContainer {
    type Target = Arc<State>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl std::ops::Not for &StateContainer {
    type Output = StateContainer;

    fn not(self) -> Self::Output {
        self.clone()
    }
}

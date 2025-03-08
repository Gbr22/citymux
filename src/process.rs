use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use tokio::{join, select};
use tokio::sync::Mutex;
use tokio::task::JoinError;

use crate::spawn::{kill_process, kill_span};
use crate::{canvas::{self, TerminalCommand}, encoding::{CsiSequence, OscSequence, CSI_FINAL_BYTES}, Process, StateContainer};

pub struct ProcessData {
    pub stdin: Box<dyn tokio::io::AsyncWrite + Unpin + Send + Sync>,
    pub stdout: Box<dyn tokio::io::AsyncRead + Unpin + Send + Sync>,
    pub terminal: Box<dyn TerminalLike>,
}

pub trait TerminalLike: Send + Sync {
    fn release<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), TerminalError>> + 'a + Send>>;
    fn set_size(&mut self, size: canvas::Vector2) -> Result<(), TerminalError>;
    fn size(&self) -> canvas::Vector2;
    fn take_done_future(&mut self) -> Option<Pin<Box<dyn std::future::Future<Output = Result<(), TerminalError>> + Send>>>;
}

#[derive(Debug)]
pub struct TerminalError {
    error: Box<dyn std::error::Error + Send + Sync>
}

unsafe impl Send for TerminalError {}
unsafe impl Sync for TerminalError {}

impl From<Box<dyn std::error::Error + Send + Sync>> for TerminalError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        TerminalError { error }
    }
}
impl From<JoinError> for TerminalError {
    fn from(error: JoinError) -> Self {
        TerminalError { error: Box::new(error) }
    }
}

impl std::error::Error for TerminalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
}

impl Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

pub async fn handle_process(state_container: StateContainer, process: Arc<Mutex<Process>>) -> Result<(), Box<dyn std::error::Error>> {
    let stdout_future = async {
        
        loop {
            let stdout = {
                let process = process.lock().await;
                process.stdout.clone()
            };
            let mut buffer = vec![0; 4096];
            let mut read_buf = ReadBuf::new(&mut buffer);
            let mut stdout = stdout.lock().await;
            let filled_buf = match stdout.read_buf(&mut read_buf).await {
                Ok(_) => {
                    read_buf.filled()
                },
                Err(err) => {
                    tracing::debug!("Error in stdout: {:?}", err);
                    break;
                }
            };
            if filled_buf.is_empty() {
                break;
            }
            {
                let process = process.lock().await;
                let mut canvas = process.terminal_info.lock().await;
                canvas.process(filled_buf);
            }
        }
    };
    let done_future = {
        let process = process.lock().await;
        let mut terminal = process.terminal.lock().await;
        terminal.take_done_future()
    };
    let done_future = async {
        if let Some(done_future) = done_future {
            done_future.await?;
        }
        Ok::<(), TerminalError>(())
    };
    tokio::select! {
        _ = done_future => {},
        _ = stdout_future => {},
    };
    tracing::debug!("Exiting process");
    let span_id = {
        let process = process.lock().await;
        process.span_id
    };
    tracing::debug!("Exiting process in span: {}", span_id);
    kill_span(state_container, span_id).await?;
    Ok(())
}

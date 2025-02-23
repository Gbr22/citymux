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
        let mut escape_distance: Option<usize> = None;
        let mut is_csi = false;
        let mut is_osc = false;
        let mut is_utf8 = false;
        let mut number_of_bytes_to_read: usize = 0;
        let mut collected = Vec::new();
        let mut buffer = vec![0; 4096];
        let mut read_buf = ReadBuf::new(&mut buffer);
        let mut filled_buf_option: Option<&[u8]> = None;
        loop {
            let stdout = {
                let process = process.lock().await;
                process.stdout.clone()
            };
            let mut stdout = stdout.lock().await;
            if filled_buf_option.is_none() {
                read_buf = ReadBuf::new(&mut buffer);
                let n = match stdout.read_buf(&mut read_buf).await {
                    Ok(n) => n,
                    Err(err) => {
                        tracing::debug!("Error in stdout: {:?}", err);
                        break;
                    }
                };
                if n == 0 {
                    tracing::debug!("EOF in stdout");
                    break;
                }
                tracing::debug!("Read {} bytes: {:?}", n, String::from_utf8_lossy(read_buf.filled()));
                filled_buf_option = Some(read_buf.filled());
                continue;
            }
            let Some(filled_buf) = filled_buf_option else {
                continue;
            };
            let Some(byte) = filled_buf.first() else {
                filled_buf_option = None;
                continue;
            };
            filled_buf_option = Some(&filled_buf[1..]);
            let byte = *byte;
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
                let process = process.lock().await;
                let command = TerminalCommand::Csi(CsiSequence::new(collected));
                let mut canvas = process.terminal_info.lock().await;
                canvas.execute_command(command);
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
                let process = process.lock().await;
                let command = TerminalCommand::Osc(OscSequence::new(collected));
                let mut canvas = process.terminal_info.lock().await;
                canvas.execute_command(command);
                collected = Vec::new();
                continue;
            }
            
            if is_osc && byte != 0x1b {
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
        
            if byte & 0b1111_0000 == 0b1111_0000 {
                is_utf8 = true;
                number_of_bytes_to_read = 3;
                collected.push(byte);
                continue;
            }
            if byte & 0b1110_0000 == 0b1110_0000 {
                is_utf8 = true;
                number_of_bytes_to_read = 2;
                collected.push(byte);
                continue;
            }
            if byte & 0b1100_0000 == 0b1100_0000 {
                is_utf8 = true;
                number_of_bytes_to_read = 1;
                collected.push(byte);
                continue;
            }
            if is_utf8 {
                number_of_bytes_to_read -= 1;
                collected.push(byte);
                if number_of_bytes_to_read <= 0 {
                    is_utf8 = false;
                    let process = process.lock().await;
                    let command = TerminalCommand::string(String::from_utf8_lossy(&collected));
                    let mut canvas = process.terminal_info.lock().await;
                    canvas.execute_command(command);
                    collected.clear();
                }
                continue;
            }
            {
                let process = process.lock().await;
                let command = TerminalCommand::string(format!("{}",byte as char));
                let mut canvas = process.terminal_info.lock().await;
                canvas.execute_command(command);
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

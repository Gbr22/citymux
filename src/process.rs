use std::sync::Arc;
use tokio::{io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt}};

use tokio::sync::Mutex;

use crate::{canvas::{self, TerminalCommand}, encoding::{CsiSequence, OscSequence, CSI_FINAL_BYTES}, Process, StateContainer};

pub struct ProcessData {
    pub stdin: Box<dyn tokio::io::AsyncWrite + Unpin + Send + Sync>,
    pub stdout: Box<dyn tokio::io::AsyncRead + Unpin + Send + Sync>,
    pub dyn_data: Box<dyn ProcessDataDyn>,
}

pub trait ProcessDataDyn {
    fn release(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

pub async fn handle_process(state_container: StateContainer, process: Arc<Mutex<Process>>) -> Result<(), Box<dyn std::error::Error>> {
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
        
        let is_csi_final_byte = (byte as char).is_alphabetic() || CSI_FINAL_BYTES.as_bytes().contains(&byte);
        if is_csi && is_csi_final_byte {
            is_csi = false;
            escape_distance = None;
            collected.push(byte);
            tracing::debug!("[OUT-CSI:{:?}]", String::from_utf8_lossy(&collected));
            let process = process.lock().await;
            let command = TerminalCommand::Csi(CsiSequence::new(collected));
            let mut canvas = process.terminal.lock().await;
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
            tracing::debug!("[OUT-OSC:{:?}]", String::from_utf8_lossy(&collected));
            let process = process.lock().await;
            let command = TerminalCommand::Osc(OscSequence::new(collected));
            let mut canvas = process.terminal.lock().await;
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
    
        {
            //tracing::debug!("[OUT:{:?}:{:?}]", byte, byte as char);
            let process = process.lock().await;
            let command = TerminalCommand::String(format!("{}",byte as char));
            let mut canvas = process.terminal.lock().await;
            canvas.execute_command(command);
        }
    }
}

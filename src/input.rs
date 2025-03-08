use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::{
    encoding::CSI_FINAL_BYTES,
    spawn::{create_process, kill_active_span},
    state::StateContainer,
};

pub async fn write_input(
    state_container: StateContainer,
    data: &[u8],
    flush: bool,
) -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn handle_stdin(
    state_container: StateContainer,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut escape_distance: Option<usize> = None;
    let mut is_csi = false;
    let is_osc = false;
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
        let is_csi_final_byte =
            (byte as char).is_alphabetic() || CSI_FINAL_BYTES.as_bytes().contains(&byte);
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
            if is_application_key_mode_enabled && "ABCDHF".as_bytes().contains(&first_byte) {
                let new_string = format!("\x1bO{}", first_byte as char);
                write_input(state_container.clone(), new_string.as_bytes(), true).await?;
                continue;
            }
            let prefix = "\x1b[".as_bytes();
            let concat: Vec<u8> = prefix
                .iter()
                .chain(collected.iter())
                .map(|e| e.to_owned())
                .collect();
            write_input(state_container.clone(), &concat, true).await?;
            continue;
        }
        if is_csi {
            collected.push(byte);
            continue;
        }

        if byte == 0x9b || (escape_distance == Some(1) && byte == b'[') {
            // CSI
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

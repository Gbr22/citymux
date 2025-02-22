use std::{collections::HashMap, sync::Arc};
use which::which;
use tokio::sync::Mutex;

use crate::{canvas::TerminalInfo, process::handle_process, tty::spawn_interactive_process, Process, StateContainer, Vector2};

pub async fn spawn_process(state_container: StateContainer, size: Vector2) -> Result<Arc<Mutex<Process>>, Box<dyn std::error::Error>> {
    let program = "cmd";
    let program = which(program)?.to_string_lossy().to_string();
    let env = HashMap::new();
    
    let result = spawn_interactive_process(&program, env, size).await?;
    let process = Process {
        stdin: Arc::new(Mutex::new(result.stdin)),
        stdout: Arc::new(Mutex::new(result.stdout)),
        terminal_info: Arc::new(Mutex::new(TerminalInfo::new(size))),
        terminal: Arc::new(Mutex::new(result.terminal)),
    };

    let process = Arc::new(Mutex::new(process));
    let processes = state_container.get_state().processes.clone();
    {
        let mut processes = processes.lock().await;
        let future = {
            let process = process.clone();
            let state_container = state_container.clone();
            async move {
                let result = handle_process(state_container, process).await;
                if let Err(e) = result {
                    eprintln!("Error: {:?}", e);
                }
            }
        };
        
        processes.push(process.clone());
        {
            let futures = state_container.get_state().futures.clone();
            let mut futures = futures.lock().await;
            futures.push(Box::pin(future));
        }
    }

    Ok(process)
}

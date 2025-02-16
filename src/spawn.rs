use std::{collections::HashMap, sync::Arc};
use which::which;
use tokio::sync::Mutex;

use crate::{handle_process, tty::spawn_interactive_process, Process, StateContainer, Vector2};

pub async fn spawn_process(state_container: StateContainer, size: Vector2) -> Result<(), Box<dyn std::error::Error>> {
    let program = "cmd";
    let program = which(program)?.to_string_lossy().to_string();
    let env = HashMap::new();
    
    let result = spawn_interactive_process(&program, env, size).await?;
    let process = Process {
        stdin: Arc::new(Mutex::new(result.stdin)),
        stdout: Arc::new(Mutex::new(result.stdout)),
        size,
        cursor: Vector2 { x: 0, y: 0 },
        buffer: Vec::new(),
    };

    let processes = state_container.get_state().processes.clone();
    {
        let mut processes = processes.lock().await;
        let process = Arc::new(Mutex::new(process));
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
        
        processes.push(process);
        {
            let futures = state_container.get_state().futures.clone();
            let mut futures = futures.lock().await;
            futures.push(Box::pin(future));
        }
    }

    Ok(())
}

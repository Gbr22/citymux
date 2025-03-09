use std::{future::Future, pin::Pin, sync::Arc};

use crate::draw::draw_loop;
use crate::escape_codes::{AllMotionTracking, SetAlternateScreenBuffer, SgrMouseHandling};
use crate::input::handle_stdin;
use crate::size::update_size;
use crate::spawn::create_process;
use crate::state::StateContainer;
use crate::terminal::enable_raw_mode;
use tokio::{io::AsyncWriteExt, sync::Mutex, task::JoinSet};

async fn handle_loop<F, R>(func: F) -> anyhow::Result<()>
where
    F: Fn() -> R,
    R: std::future::Future<Output = anyhow::Result<()>>,
{
    loop {
        let result = func().await;
        if let Err(e) = result {
            tracing::error!("Error: {:?}", e);
        }
    }
}

async fn init_proc_handler(
    state_container: StateContainer,
) -> Result<
    tokio::sync::mpsc::Receiver<Pin<Box<dyn Future<Output = ()> + Send>>>,
    Box<dyn std::error::Error>,
> {
    let rx: tokio::sync::mpsc::Receiver<Pin<Box<dyn Future<Output = ()> + Send>>> = {
        let state = state_container.state();
        let mut process_channel = state.process_channel.lock().await;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        *process_channel = Some(tx);

        rx
    };

    Ok(rx)
}

async fn handle_child_processes(
    state_container: StateContainer,
    rx: Arc<Mutex<tokio::sync::mpsc::Receiver<Pin<Box<dyn Future<Output = ()> + Send>>>>>,
) -> anyhow::Result<()> {
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

async fn init_screen(state_container: StateContainer) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    update_size(state_container.clone()).await?;

    let stdout = state_container.state().stdout.clone();
    let mut stdout = stdout.lock().await;
    stdout
        .write(SetAlternateScreenBuffer::enable().into())
        .await?;
    stdout.write(AllMotionTracking::new(true).into()).await?;
    stdout.write(SgrMouseHandling::new(true).into()).await?;
    stdout.flush().await?;

    Ok(())
}

pub async fn run_application(
    state_container: StateContainer,
) -> Result<(), Box<dyn std::error::Error>> {
    init_screen(state_container.clone()).await?;
    let rx = init_proc_handler(state_container.clone()).await?;
    let rx = Arc::new(Mutex::new(rx));
    let stdout_handler =
        handle_loop(|| handle_child_processes(state_container.clone(), rx.clone()));
    create_process(state_container.clone()).await?;
    let results = tokio::join!(
        handle_loop(|| handle_stdin(state_container.clone())),
        stdout_handler,
        handle_loop(|| draw_loop(state_container.clone())),
    );
    results.0?;
    results.1?;
    results.2?;

    Ok(())
}

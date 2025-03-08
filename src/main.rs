use std::fs::OpenOptions;

use canvas::Vector2;
use exit::exit;
use startup::run_application;
use state::{State, StateContainer};
use tokio::io::{self};

mod canvas;
mod draw;
mod encoding;
mod escape_codes;
mod exit;
mod input;
mod layout;
mod process;
mod span;
mod spawn;
mod startup;
mod state;
mod terminal;
mod tty;
mod tty_windows;

#[cfg(test)]
mod test;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .with_writer(|| {
            OpenOptions::new()
                .append(true)
                .create(true)
                .open("log.txt")
                .expect("Failed to open log file")
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    tracing::info!("Starting up");

    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("Panic at {:?}: {:?}", info.location(), info.payload());
        exit(1);
    }));

    let state_container = StateContainer::new(State::new(io::stdin(), io::stdout()));
    if let Err(e) = run_application(state_container).await {
        tracing::error!("Error: {}", e);
        exit(1);
    }

    Ok(())
}

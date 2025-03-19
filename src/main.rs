use std::fs::OpenOptions;

use args::CliArgs;
use clap::Parser;
use config::get_config;
use error::trace_error;
use exit::exit;
use startup::run_application;
use state::{State, StateContainer};
use tokio::io::{self};

mod draw;
mod encoding;
mod escape_codes;
mod exit;
mod input;
mod layout;
mod process;
mod size;
mod span;
mod spawn;
mod startup;
mod state;
mod terminal;
mod tty;
mod tty_windows;
mod term;
mod args;
mod config;
mod error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    if args.enable_logging && args.log_file.is_some() {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_writer(|| {
                let log_file = CliArgs::parse().log_file.expect("Expected log file path to be Some");

                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(log_file)
                    .expect("Failed to open log file")
            })
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
    }

    tracing::info!("Starting up");
    let config = get_config();
    tracing::debug!("Current config: {:?}", config);

    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("Panic at {:?}: {:?}", info.location(), info.payload());
        exit(1);
    }));

    let state_container = StateContainer::new(State::new(args,config,io::stdin(), io::stdout()));
    if let Err(e) = run_application(state_container).await {
        trace_error("in application", &e);
        exit(1);
    }

    Ok(())
}

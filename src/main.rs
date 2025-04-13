use std::{env, fs::OpenOptions};

use args::CliArgs;
use config::get_config;
use data_encoding::BASE32HEX_NOPAD;
use error::trace_error;
use exit::exit;
use startup::run_application;
use state::{State, StateContainer};
use tokio::io::{self};
use tty::TtyParameters;

mod args;
mod config;
mod draw;
mod encoding;
mod error;
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
mod term;
mod terminal;
mod tty;
mod tty_windows;

async fn run_multiplexer() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    if args.enable_logging && args.log_file.is_some() {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_writer(|| {
                let log_file = CliArgs::parse()
                    .log_file
                    .expect("Expected log file path to be Some");

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

    let state_container = StateContainer::new(State::new(args, config, io::stdin(), io::stdout()));
    if let Err(e) = run_application(state_container).await {
        trace_error("in application", &e);
        exit(1);
    }

    Ok(())
}

async fn run_subprocess(tty_params: TtyParameters) -> anyhow::Result<()> {
    let mut child = std::process::Command::new(tty_params.executable)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let result = child.wait()?;
    std::process::exit(result.code().unwrap_or(1));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let program = env::args().next();
    let program = program.unwrap_or_else(|| "".to_string());
    if program.starts_with("!") {
        if program.starts_with("!spawn-") {
            let value = program.strip_prefix("!spawn-").unwrap();
            let value = BASE32HEX_NOPAD.decode(value.as_bytes())?;
            let value = serde_cbor::from_slice::<tty::TtyParameters>(&value)?;
            run_subprocess(value).await?;
        }
        return Ok(());
    } else {
        run_multiplexer().await?
    }

    Ok(())
}

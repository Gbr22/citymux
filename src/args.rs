use clap::arg;
use clap::Arg;
use clap::Args;
use clap::Command;
use clap::Parser;

pub struct CliArgs {
    pub log_file: Option<String>,
    pub enable_logging: bool,
}

impl CliArgs {
    pub fn parse() -> CliArgs {
        let matches = get_clap_parser().get_matches();
        let log_file = matches.get_one::<String>("logFile").map(|e| e.to_string());
        let enable_logging = matches
            .get_one::<bool>("enableLogging")
            .map(|e| *e)
            .unwrap_or_default();

        CliArgs {
            log_file,
            enable_logging,
        }
    }
}

pub fn get_clap_parser() -> Command {
    Command::new("citymux")
        .about("Terminal multiplexer")
        .arg(
            Arg::new("logFile")
                .long("log-file")
                .value_name("FILE")
                .help("Set the log file path")
                .required(false),
        )
        .arg(
            Arg::new("enableLogging")
                .long("enable-logging")
                .help("Enable logging")
                .num_args(0)
                .required(false),
        )
}

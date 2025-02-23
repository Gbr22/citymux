use std::io::Write;

use crate::escape_codes::{ResetStyle, SetAlternateScreenBuffer};

fn exit_helper() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = std::io::stdout();
    stdout.write(ResetStyle::default().into())?;
    stdout.write(SetAlternateScreenBuffer::disable().into())?;
    stdout.flush()?;

    std::process::exit(0);
}

pub fn exit() {
    let result = exit_helper();
    if let Err(e) = result {
        tracing::error!("Error: {:?}", e);
        std::process::exit(1);
    }
}
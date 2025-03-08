use std::io::Write;

use crate::escape_codes::{ResetStyle, SetAlternateScreenBuffer};

fn exit_helper(status_code: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = std::io::stdout();
    stdout.write(ResetStyle::default().into())?;
    stdout.write(SetAlternateScreenBuffer::disable().into())?;
    stdout.flush()?;

    std::process::exit(status_code);
}

pub fn exit(status_code: i32) {
    let result = exit_helper(status_code);
    if let Err(e) = result {
        tracing::error!("Error: {:?}", e);
        std::process::exit(1);
    }
}

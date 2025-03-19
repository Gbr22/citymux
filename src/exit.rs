use std::io::Write;

use crate::escape_codes::{
    AllMotionTracking, ResetStyle, SetAlternateScreenBuffer, SetCursorVisibility, SetWin32InputMode, SgrMouseHandling
};

fn exit_helper(status_code: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = std::io::stdout();
    stdout.write(ResetStyle::default().into())?;
    stdout.write(SetWin32InputMode::new(false).into())?;
    stdout.write(SetAlternateScreenBuffer::new(false).into())?;
    stdout.write(AllMotionTracking::new(false).into())?;
    stdout.write(SgrMouseHandling::new(false).into())?;
    stdout.write(SetCursorVisibility::new(true).into())?;
    stdout.flush()?;

    std::process::exit(status_code);
}

pub fn exit(status_code: i32) {
    tracing::info!("Exiting application with code: {}", status_code);
    let result = exit_helper(status_code);
    if let Err(e) = result {
        tracing::error!("Error during exit: {:?}", e);
        std::process::exit(1);
    }
}

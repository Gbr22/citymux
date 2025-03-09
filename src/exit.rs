use std::io::Write;

use crate::escape_codes::{
    AllMotionTracking, ResetStyle, SetAlternateScreenBuffer, SetCursorVisibility, SgrMouseHandling,
};

fn exit_helper(status_code: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = std::io::stdout();
    stdout.write(ResetStyle::default().into())?;
    stdout.write(SetAlternateScreenBuffer::disable().into())?;
    stdout.write(AllMotionTracking::new(false).into())?;
    stdout.write(SgrMouseHandling::new(false).into())?;
    stdout.write(SetCursorVisibility::new(true).into())?;
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

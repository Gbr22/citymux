use std::io::Write;

use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, PopKeyboardEnhancementFlags,
    },
    execute,
};

use crate::escape_codes::{
    AllMotionTracking, ResetStyle, SetAlternateScreenBuffer, SetCursorVisibility,
    SetWin32InputMode, SgrMouseHandling,
};

fn exit_helper(status_code: i32) -> Result<(), Box<dyn std::error::Error>> {
    let _ignored = crossterm::terminal::disable_raw_mode();
    let _ignored = execute!(
        std::io::stdout(),
        DisableBracketedPaste,
        DisableFocusChange,
        DisableMouseCapture,
    );

    let _ignored = execute!(std::io::stdout(), PopKeyboardEnhancementFlags,);

    let mut stdout = std::io::stdout();
    let _ignored = stdout.write(ResetStyle::default().into());
    let _ignored = stdout.write(SetAlternateScreenBuffer::new(false).into());
    let _ignored = stdout.write(SetCursorVisibility::new(true).into());
    let _ignored = stdout.flush();

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

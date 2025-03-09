use crossterm_winapi::{ConsoleMode, Handle};
use winapi::um::wincon::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_MOUSE_INPUT, ENABLE_PROCESSED_INPUT,
    ENABLE_VIRTUAL_TERMINAL_INPUT, ENABLE_WINDOW_INPUT,
};

pub fn enable_raw_mode() -> Result<(), Box<dyn std::error::Error>> {
    let console_mode = ConsoleMode::from(Handle::current_in_handle()?);
    let current_mode = console_mode.mode()?;
    let new_mode = (current_mode
        & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
        | (ENABLE_VIRTUAL_TERMINAL_INPUT | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT);
    console_mode.set_mode(new_mode)?;

    Ok(())
}

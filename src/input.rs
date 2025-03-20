use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use renterm::vector::Vector2;
use tokio::io::AsyncWriteExt;

use crate::{
    draw::trigger_draw, spawn::{create_process, kill_active_span}, state::StateContainer
};

pub async fn write_input(
    state_container: StateContainer,
    data: &[u8],
    flush: bool,
) -> anyhow::Result<()> {
    let active_process = state_container.state().active_process().await;
    if let Some(active_process) = active_process {
        let process = active_process.lock().await;
        let mut stdin = process.stdin.lock().await;
        stdin.write(data).await?;
        if flush {
            stdin.flush().await?;
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct KeyEventConversionOptions {
    pub is_application_keypad_mode_enabled: bool,
    _private: (),
}

impl KeyEventConversionOptions {
    pub fn with_application_keypad_mode(mut self, is_enabled: bool) -> Self {
        self.is_application_keypad_mode_enabled = is_enabled;
        self
    }
}

impl Default for KeyEventConversionOptions {
    fn default() -> Self {
        Self {
            is_application_keypad_mode_enabled: false,
            _private: (),
        }
    }
}

fn key_event_to_bytes(event: KeyEvent, options: KeyEventConversionOptions) -> Vec<u8> {
    let mut bytes = Vec::new();
    if event.kind == crossterm::event::KeyEventKind::Press || event.kind == crossterm::event::KeyEventKind::Repeat {
        match event.code {
            KeyCode::Backspace => {
                bytes.push(0x7f);
            },
            KeyCode::Enter => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOM".as_bytes());
                }
                else {
                    bytes.push(b'\n');
                }
            },
            KeyCode::Left => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOD".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[D".as_bytes());
                }
            },
            KeyCode::Right => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOC".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[C".as_bytes());
                }
            },
            KeyCode::Up => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOA".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[A".as_bytes());
                }
            },
            KeyCode::Down => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOB".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[B".as_bytes());
                }
            },
            KeyCode::Home => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOH".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[H".as_bytes());
                }
            },
            KeyCode::End => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOF".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[F".as_bytes());
                }
            },
            KeyCode::Delete => {
                bytes.extend_from_slice("\x1b[3~".as_bytes());
            },
            KeyCode::PageUp => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bO5".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[5~".as_bytes());
                }
            },
            KeyCode::PageDown => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bO6".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[6~".as_bytes());
                }
            },
            KeyCode::Tab => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOI".as_bytes());
                }
                else {
                    bytes.push(b'\t');
                }
            },
            KeyCode::Insert => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bO2".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[2~".as_bytes());
                }
            },
            KeyCode::BackTab => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOZ".as_bytes());
                }
                else {
                    bytes.extend_from_slice("\x1b[Z".as_bytes());
                }
            },
            KeyCode::F(_value) => {

            },
            KeyCode::Char(char) => {
                if event.modifiers.intersects(KeyModifiers::CONTROL) && char.is_ascii_alphabetic() {
                    let char = char.to_ascii_uppercase();
                    bytes.push(char as u8 - 'A' as u8 + 1);
                }
                else if event.modifiers.intersects(KeyModifiers::ALT) {
                    bytes.push(0x1b);
                    let string = format!("{}", char);
                    bytes.extend_from_slice(string.as_bytes());
                }
                else {
                    let string = format!("{}", char);
                    bytes.extend_from_slice(string.as_bytes());
                }
            },
            KeyCode::Null => {
                bytes.push(0);
            },
            KeyCode::Esc => {
                bytes.push(0x1b);
            },
            KeyCode::CapsLock => {},
            KeyCode::ScrollLock => {},
            KeyCode::NumLock => {},
            KeyCode::PrintScreen => {},
            KeyCode::Pause => {},
            KeyCode::Menu => {},
            KeyCode::KeypadBegin => {},
            KeyCode::Media(media_key_code) => {},
            KeyCode::Modifier(modifier_key_code) => {},
        };
    }
    
    bytes
}

async fn handle_shortcuts(state_container: StateContainer, event: KeyEvent) -> anyhow::Result<bool> {
    if event.code == KeyCode::Char('q') && event.modifiers.intersects(KeyModifiers::ALT) {
        kill_active_span(state_container.clone()).await?;
        return Ok(true);
    }
    else if event.code == KeyCode::Char('n') && event.modifiers.intersects(KeyModifiers::ALT) {
        create_process(state_container.clone()).await?;
        return Ok(true);
    }

    Ok(false)
}

async fn handle_key_event(state_container: StateContainer, event: KeyEvent) -> anyhow::Result<()> {
    if handle_shortcuts(state_container.clone(), event).await? == true {
        return Ok(());
    }

    let data = key_event_to_bytes(event, KeyEventConversionOptions::default()
            .with_application_keypad_mode(state_container.state().application_keypad_mode().await.unwrap_or(false)));
        write_input(state_container, &data, true).await?;

    Ok(())
}

pub async fn handle_stdin(state_container: StateContainer) -> anyhow::Result<()> {
    loop {
        let mut reader = EventStream::new();
        loop {
            let maybe_event = reader.next().await;
            if let Some(Ok(Event::Key(key))) = maybe_event {
                handle_key_event(state_container.clone(), key).await?;
                trigger_draw(state_container.clone()).await;
            }
            if let Some(Ok(Event::Resize(x, y))) = maybe_event {
                let state = state_container.state();
                let mut size = state.size.write().await;
                *size = Vector2::new(x as isize, y as isize);
                trigger_draw(state_container.clone()).await;
            }
        }
    }
}

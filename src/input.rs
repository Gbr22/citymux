use crossterm::event::{
    Event, EventStream, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind,
};
use futures::StreamExt;
use renterm::{scalar::Scalar, vector::Vector2};
use tokio::io::AsyncWriteExt;

use crate::{
    draw::trigger_draw,
    spawn::{create_process, kill_active_span},
    state::StateContainer,
    term::{MouseProtocolEncoding, MouseProtocolMode},
};

pub async fn write_input(
    state_container: StateContainer,
    data: &[u8],
    flush: bool,
) -> anyhow::Result<()> {
    let active_process = state_container.state().active_process().await;
    if let Some(active_process) = active_process {
        let process = active_process.read().await;
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
    if event.kind == crossterm::event::KeyEventKind::Press
        || event.kind == crossterm::event::KeyEventKind::Repeat
    {
        match event.code {
            KeyCode::Backspace => {
                bytes.push(0x7f);
            }
            KeyCode::Enter => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOM".as_bytes());
                } else {
                    bytes.push(b'\r');
                }
            }
            KeyCode::Left => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOD".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[D".as_bytes());
                }
            }
            KeyCode::Right => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOC".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[C".as_bytes());
                }
            }
            KeyCode::Up => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOA".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[A".as_bytes());
                }
            }
            KeyCode::Down => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOB".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[B".as_bytes());
                }
            }
            KeyCode::Home => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOH".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[H".as_bytes());
                }
            }
            KeyCode::End => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOF".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[F".as_bytes());
                }
            }
            KeyCode::Delete => {
                bytes.extend_from_slice("\x1b[3~".as_bytes());
            }
            KeyCode::PageUp => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bO5".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[5~".as_bytes());
                }
            }
            KeyCode::PageDown => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bO6".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[6~".as_bytes());
                }
            }
            KeyCode::Tab => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOI".as_bytes());
                } else {
                    bytes.push(b'\t');
                }
            }
            KeyCode::Insert => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bO2".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[2~".as_bytes());
                }
            }
            KeyCode::BackTab => {
                if options.is_application_keypad_mode_enabled {
                    bytes.extend_from_slice("\x1bOZ".as_bytes());
                } else {
                    bytes.extend_from_slice("\x1b[Z".as_bytes());
                }
            }
            KeyCode::F(_value) => {}
            KeyCode::Char(char) => {
                if event.modifiers.intersects(KeyModifiers::CONTROL) && char.is_ascii_alphabetic() {
                    let char = char.to_ascii_uppercase();
                    bytes.push(char as u8 - 'A' as u8 + 1);
                } else if event.modifiers.intersects(KeyModifiers::ALT)
                    && char.is_ascii_alphabetic()
                {
                    bytes.push(0x1b);
                    let string = format!("{}", char);
                    bytes.extend_from_slice(string.as_bytes());
                } else {
                    let string = format!("{}", char);
                    bytes.extend_from_slice(string.as_bytes());
                }
            }
            KeyCode::Null => {
                bytes.push(0);
            }
            KeyCode::Esc => {
                bytes.push(0x1b);
            }
            KeyCode::CapsLock => {}
            KeyCode::ScrollLock => {}
            KeyCode::NumLock => {}
            KeyCode::PrintScreen => {}
            KeyCode::Pause => {}
            KeyCode::Menu => {}
            KeyCode::KeypadBegin => {}
            KeyCode::Media(media_key_code) => {}
            KeyCode::Modifier(modifier_key_code) => {}
        };
    }

    bytes
}

async fn handle_navigation(state: &StateContainer, direction: Vector2) -> anyhow::Result<()> {
    let processess = state.processes.read().await;
    let current_process = state.active_process().await;
    let Some(current_process) = current_process else {
        return Ok(());
    };
    let current_process = current_process.read().await;
    let current_dimensions = state.get_span_dimensions(current_process.span_id).await;
    let Some(current_dimensions) = current_dimensions else {
        return Ok(());
    };
    let position: Vector2 = match direction.signnum().into() {
        (-1, 0) => (
            current_dimensions.position().x - 1,
            current_dimensions.position().y + current_dimensions.size().y / 2,
        ),
        (1, 0) => (
            current_dimensions.position().x + current_dimensions.size().x + 1,
            current_dimensions.position().y + current_dimensions.size().y / 2,
        ),
        (0, -1) => (
            current_dimensions.position().x + current_dimensions.size().x / 2,
            current_dimensions.position().y - 1,
        ),
        (0, 1) => (
            current_dimensions.position().x + current_dimensions.size().x / 2,
            current_dimensions.position().y + current_dimensions.size().y + 1,
        ),
        _ => current_dimensions.position().into(),
    }
    .into();

    tracing::debug!("dim: {:?} position: {:?}", current_dimensions, position);

    for process in processess.iter() {
        let process = process.clone();
        let process = process.read().await;
        let rect = state.get_span_dimensions(process.span_id).await;
        let Some(rect) = rect else {
            continue;
        };
        if rect.contains(position.clone()) {
            state.set_active_span(process.span_id);
            break;
        }
    }

    Ok(())
}

async fn handle_shortcuts(
    state_container: &StateContainer,
    event: KeyEvent,
) -> anyhow::Result<bool> {
    if event.code == KeyCode::Char('q')
        && event.modifiers.intersects(KeyModifiers::ALT)
        && event.kind == crossterm::event::KeyEventKind::Press
    {
        kill_active_span(state_container.clone()).await?;
        return Ok(true);
    } else if event.code == KeyCode::Char('n')
        && event.modifiers.intersects(KeyModifiers::ALT)
        && event.kind == crossterm::event::KeyEventKind::Press
    {
        create_process(state_container.clone()).await?;
        return Ok(true);
    } else if event.code == KeyCode::Left
        && event.modifiers.intersects(KeyModifiers::ALT)
        && event.kind == crossterm::event::KeyEventKind::Press
    {
        return handle_navigation(state_container, Vector2::new(-1, 0))
            .await
            .map(|_| true);
    } else if event.code == KeyCode::Right
        && event.modifiers.intersects(KeyModifiers::ALT)
        && event.kind == crossterm::event::KeyEventKind::Press
    {
        return handle_navigation(state_container, Vector2::new(1, 0))
            .await
            .map(|_| true);
    } else if event.code == KeyCode::Up
        && event.modifiers.intersects(KeyModifiers::ALT)
        && event.kind == crossterm::event::KeyEventKind::Press
    {
        return handle_navigation(state_container, Vector2::new(0, -1))
            .await
            .map(|_| true);
    } else if event.code == KeyCode::Down
        && event.modifiers.intersects(KeyModifiers::ALT)
        && event.kind == crossterm::event::KeyEventKind::Press
    {
        return handle_navigation(state_container, Vector2::new(0, 1))
            .await
            .map(|_| true);
    }

    Ok(false)
}

async fn handle_key_event(state_container: StateContainer, event: KeyEvent) -> anyhow::Result<()> {
    if handle_shortcuts(&state_container, event).await? == true {
        return Ok(());
    }

    let data = key_event_to_bytes(
        event,
        KeyEventConversionOptions::default().with_application_keypad_mode(
            state_container
                .state()
                .application_keypad_mode()
                .await
                .unwrap_or(false),
        ),
    );
    write_input(state_container, &data, true).await?;

    Ok(())
}

fn map_button_to_int(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

async fn set_mouse_button_state(state_container: &StateContainer, button: u8, value: bool) {
    let state = state_container.state();
    let mut current_mouse_buttons = state.current_mouse_buttons.write().await;
    current_mouse_buttons.insert(button, value);
}

async fn has_mouse_press(state_container: &StateContainer) -> bool {
    let state = state_container.state();
    let map = state.current_mouse_buttons.read().await;

    map.iter().any(|(_, value)| *value)
}

async fn handle_mouse_event(
    state: &StateContainer,
    event: crossterm::event::MouseEvent,
) -> anyhow::Result<()> {
    let position: Vector2 = Vector2::new(event.column, event.row);
    let button = match event.kind {
        MouseEventKind::Down(button) => map_button_to_int(button),
        MouseEventKind::Up(button) => map_button_to_int(button),
        MouseEventKind::Drag(button) => map_button_to_int(button),
        MouseEventKind::ScrollUp => 64,
        MouseEventKind::ScrollDown => 65,
        _ => 0,
    };
    let _is_scroll = [64, 65].contains(&button);
    let is_release = if let MouseEventKind::Up(_) = event.kind {
        true
    } else {
        false
    };
    let is_press = if let MouseEventKind::Down(_) = event.kind {
        true
    } else {
        false
    };

    match event.kind {
        MouseEventKind::Down(button) => {
            set_mouse_button_state(state, map_button_to_int(button), true).await;
        }
        MouseEventKind::Up(button) => {
            set_mouse_button_state(state, map_button_to_int(button), false).await;
        }
        _ => {}
    }

    let has_mouse_press = has_mouse_press(state).await;

    {
        let mut mouse_position = state.current_mouse_position.write().await;
        *mouse_position = position.clone();
    }

    let processess = state.processes.read().await;
    for process in processess.iter() {
        let process = process.clone();
        let process = process.read().await;
        let rect = state.get_span_dimensions(process.span_id).await;
        let Some(rect) = rect else {
            continue;
        };
        if rect.contains(position.clone()) {
            let shifted_position = position.clone() - rect.position();
            let terminal_info = process.terminal_info.lock().await;
            let mouse_mode = terminal_info.mouse_protocol_mode();
            if is_press {
                state.set_active_span(process.span_id);
            }
            let mut should_write = false;
            match mouse_mode {
                MouseProtocolMode::None => {}
                MouseProtocolMode::Press => {
                    should_write = is_press;
                }
                MouseProtocolMode::PressRelease => {
                    should_write = is_press || is_release;
                }
                MouseProtocolMode::ButtonMotion => {
                    if has_mouse_press {
                        should_write = true;
                    }
                }
                MouseProtocolMode::AnyMotion => {
                    should_write = true;
                }
            }
            if should_write {
                let encoding = terminal_info.mouse_protocol_encoding();
                const LEGACY_MOUSE_MODE_OFFSET: u16 = 32;
                const LEGACY_MOUSE_MODE_COORDINATE_OFFSET: u16 = LEGACY_MOUSE_MODE_OFFSET + 1;
                let mouse_position_offset_vector: Vector2 = Vector2::new(
                    LEGACY_MOUSE_MODE_COORDINATE_OFFSET,
                    LEGACY_MOUSE_MODE_COORDINATE_OFFSET,
                );
                tracing::debug!("Sending mouse event: position: {:?} button: {:?} is_release: {:?}, encoding: {:?}", position, button, is_release, encoding);
                match encoding {
                    MouseProtocolEncoding::Default => {
                        let shifted_position = shifted_position + mouse_position_offset_vector;
                        let button = 3;
                        let data = format!(
                            "\x1b[M{}{}{}",
                            char::from_u32((button + LEGACY_MOUSE_MODE_OFFSET) as u32)
                                .unwrap_or_default(),
                            char::from_u32(shifted_position.x as u32).unwrap_or_default(),
                            char::from_u32(shifted_position.y as u32).unwrap_or_default()
                        );
                        let data = data.as_bytes();
                        let mut stdin = process.stdin.lock().await;
                        stdin.write(data).await?;
                        stdin.flush().await?;
                    }
                    MouseProtocolEncoding::Sgr => {
                        let command = if is_release { 'm' } else { 'M' };
                        let data = format!(
                            "\x1b[<{};{};{}{}",
                            button, shifted_position.x, shifted_position.y, command
                        );
                        let data = data.as_bytes();
                        let mut stdin = process.stdin.lock().await;
                        stdin.write(data).await?;
                        stdin.flush().await?;
                    }
                    MouseProtocolEncoding::Utf8 => {
                        let command = if is_release { 'm' } else { 'M' };
                        let data = format!(
                            "\x1b[<{};{};{}{}",
                            char::from_u32(button as u32).unwrap_or_default(),
                            char::from_u32(shifted_position.x as u32).unwrap_or_default(),
                            char::from_u32(shifted_position.y as u32).unwrap_or_default(),
                            command
                        );
                        let data = data.as_bytes();
                        let mut stdin = process.stdin.lock().await;
                        stdin.write(data).await?;
                        stdin.flush().await?;
                    }
                }
            }
            break;
        }
    }

    Ok(())
}

pub async fn handle_stdin(state: StateContainer) -> anyhow::Result<()> {
    loop {
        let mut reader = EventStream::new();
        loop {
            let maybe_event = reader.next().await;
            if let Some(Ok(Event::Key(key))) = maybe_event {
                handle_key_event(state.to_owned(), key).await?;
                trigger_draw(&state).await;
            }
            if let Some(Ok(Event::Resize(x, y))) = maybe_event {
                state.set_size((x, y)).await;
                trigger_draw(&state).await;
            }
            if let Some(Ok(Event::Mouse(event))) = maybe_event {
                state.set_mouse_position((event.column, event.row)).await;
                handle_mouse_event(&state, event).await?;
                trigger_draw(&state).await;
            }
        }
    }
}

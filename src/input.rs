use std::{ops::DerefMut, time::Duration};

use renterm::vector::Vector2;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::{
    spawn::{create_process, kill_active_span},
    state::StateContainer, term::{MouseProtocolEncoding, MouseProtocolMode},
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

struct Performer {
    state_container: StateContainer,
    futures: Vec<tokio::task::JoinHandle<anyhow::Result<()>>>,
    mouse_sequence_remaining: u8,
    mouse_button: u16,
    mouse_x: u16,
    has_mouse_press: bool,
}

const LEGACY_MOUSE_MODE_OFFSET: u16 = 32;
const LEGACY_MOUSE_MODE_COORDINATE_OFFSET: u16 = LEGACY_MOUSE_MODE_OFFSET + 1;
const MOUSE_POSITION_OFFSET_VECTOR: Vector2 = Vector2::new(
    LEGACY_MOUSE_MODE_COORDINATE_OFFSET as isize,
    LEGACY_MOUSE_MODE_COORDINATE_OFFSET as isize,
);

impl Performer {
    pub fn new(state_container: StateContainer) -> Self {
        Self {
            state_container,
            futures: Vec::new(),
            mouse_button: 0,
            mouse_x: 0,
            mouse_sequence_remaining: 0,
            has_mouse_press: false,
        }
    }
    pub async fn block(&mut self) -> anyhow::Result<()> {
        let mut join_set = tokio::task::JoinSet::new();
        for future in self.futures.drain(..) {
            join_set.spawn(future);
        }
        loop {
            tokio::select! {
                Some(result) = join_set.join_next() => {
                    if let Err(e) = result {
                        tracing::error!("Error in child: {:?}", e);
                    }
                }
                else => {
                    break;
                },
            }
        }
        Ok(())
    }
    fn on_mouse_event(&mut self, button: u16, x: usize, y: usize, is_release: bool) {
        let position = Vector2::new(x as isize, y as isize);
        let is_scroll = [64, 65].contains(&button);
        let is_release = is_release || button == 3;
        let is_press = [0, 1, 2].contains(&button) && !is_release;
        if is_press {
            self.has_mouse_press = true;
        }
        if is_release {
            self.has_mouse_press = false;
        }
        let has_mouse_press = self.has_mouse_press;

        let state = self.state_container.state();
        self.run(|| async move {
            let mut mouse_position = state.current_mouse_position.write().await;
            let mouse_position = mouse_position.deref_mut();
            *mouse_position = Vector2::new(x as isize, y as isize);

            let processess = state.processes.read().await;
            for process in processess.iter() {
                let process = process.clone();
                let process = process.lock().await;
                let rect = state.get_span_dimensions(process.span_id).await;
                let Some(rect) = rect else {
                    continue;
                };
                if rect.contains(position) {
                    let shifted_position = position - rect.position();
                    let terminal_info = process.terminal_info.lock().await;
                    let mouse_mode = terminal_info.mouse_protocol_mode();
                    if is_press {
                        state.set_active_span(process.span_id);
                    }
                    let mut should_write = false;
                    if is_scroll {
                        should_write = true;
                    }
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
                        match encoding {
                            MouseProtocolEncoding::Default => {
                                let shifted_position =
                                    shifted_position + MOUSE_POSITION_OFFSET_VECTOR;
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
        });
    }
    fn run<Fn, Fut>(&mut self, func: Fn)
    where
        Fn: FnOnce() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let fut = func();
        let join_handle = tokio::spawn(fut);
        self.futures.push(join_handle);
    }
}

impl vte::Perform for Performer {
    fn print(&mut self, char: char) {
        if self.mouse_sequence_remaining > 0 {
            if self.mouse_sequence_remaining == 3 {
                self.mouse_button = (char as usize - LEGACY_MOUSE_MODE_OFFSET as usize) as u16;
            } else if self.mouse_sequence_remaining == 2 {
                self.mouse_x = char as u16 - LEGACY_MOUSE_MODE_COORDINATE_OFFSET;
            } else if self.mouse_sequence_remaining == 1 {
                self.on_mouse_event(
                    self.mouse_button,
                    self.mouse_x as usize,
                    char as usize - LEGACY_MOUSE_MODE_COORDINATE_OFFSET as usize,
                    false,
                );
            }
            self.mouse_sequence_remaining -= 1;
            return;
        }
        let new_string = format!("{}", char);
        let state = self.state_container.clone();
        let future = async move { write_input(state, new_string.as_bytes(), true).await };
        self.futures.push(tokio::spawn(future));
        tracing::debug!("[PRINT] {:?}", char);
    }
    fn execute(&mut self, byte: u8) {
        let bytes = [byte];
        let state = self.state_container.clone();
        self.run(|| async move { write_input(state, &bytes, true).await });
        tracing::debug!("[EXECUTE] {:?}", byte);
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        tracing::debug!("[OSC] Params: {:?}", _params);
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let is_mouse = action == 'M' || action == 'm';
        let is_release = action == 'm';
        let is_sgr = String::from_utf8_lossy(intermediates) == "<";
        if is_mouse && !is_sgr {
            self.mouse_sequence_remaining = 3;
            return;
        }
        if is_mouse && is_sgr {
            let mut it = params.iter();
            let button = it.next().unwrap_or_default();
            let x = it.next().unwrap_or_default();
            let y = it.next().unwrap_or_default();
            let button = button.first().unwrap_or(&0);
            let x = x.first().unwrap_or(&1);
            let y = y.first().unwrap_or(&1);
            self.on_mouse_event(*button, *x as usize - 1, *y as usize - 1, is_release);
            return;
        }
        let state = self.state_container.clone();
        let owned_intermediates = intermediates.to_vec();
        let owned_params: Vec<Vec<u16>> = params.iter().map(|e| e.to_vec()).collect();

        tracing::debug!(
            "[CSI] Params: {:?}, int: {:?}, action: {:?}",
            owned_params,
            owned_intermediates,
            action
        );

        self.run(|| async move {
            let params_string: Vec<String> = owned_params
                .iter()
                .map(|e| {
                    let strings: Vec<String> = e.iter().map(|e| format!("{}", e)).collect();
                    strings.join(":")
                })
                .collect();
            let mut params_string = params_string.join(";");
            if params_string == "0" {
                params_string = "".to_string();
            }
            let application_keypad_mode = state
                .state()
                .application_keypad_mode()
                .await
                .unwrap_or(false);
            if application_keypad_mode
                && "ABCDHF".contains(action)
                && owned_intermediates.is_empty()
            {
                let bytes = [0x1b, b'O', action as u8];
                tracing::debug!("Writing in keypad mode");
                return write_input(state.clone(), &bytes, true).await;
            }
            let mut bytes: Vec<u8> = Vec::new();
            bytes.extend(b"\x1b[");
            bytes.extend(owned_intermediates);
            bytes.extend(params_string.as_bytes());
            let action = format!("{}", action);
            bytes.extend(action.as_bytes());
            write_input(state.clone(), &bytes, true).await
        });
        
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        if ignore {
            return;
        }
        tracing::debug!("[ESC] {:?} {:?}", intermediates, byte);
    }

    fn terminated(&self) -> bool {
        false
    }
}

pub async fn handle_stdin(state_container: StateContainer) -> anyhow::Result<()> {
    let mut escape_distance: Option<usize> = None;

    let mut parser = vte::Parser::new();

    let mut performer = Performer::new(state_container.clone());

    loop {
        let stdin = state_container.state().stdin.clone();
        let mut stdin = stdin.lock().await;

        let mut buf = [0; 1];

        let timeout_result = timeout(Duration::from_millis(100), stdin.read(&mut buf)).await;
        let Ok(result) = timeout_result else {
            if escape_distance == Some(0) {
                let data = "\x1b".as_bytes();
                write_input(state_container.clone(), data, true).await?;
                escape_distance = None;
            }
            continue;
        };
        let n: usize = result?;
        if n == 0 {
            return Ok(());
        }
        let byte = buf[0];
        parser.advance(&mut performer, &buf);
        performer.block().await?;

        if let Some(escape_distance_value) = escape_distance {
            escape_distance = Some(escape_distance_value + 1);
        }
        if byte == 0x1b {
            escape_distance = Some(0);
            continue;
        }
        if byte == b'q' && escape_distance == Some(1) {
            kill_active_span(state_container.clone()).await?;
            continue;
        }
        if byte == b'n' && escape_distance == Some(1) {
            create_process(state_container.clone()).await?;
            continue;
        }
    }
}

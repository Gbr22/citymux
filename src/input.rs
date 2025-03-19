use std::{future::Future, ops::DerefMut, time::Duration};

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

#[derive(Clone, Debug, PartialEq, Eq)]
struct KeyModifiers {
    pub value: u8
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self {
            value: 0,
        }
    }
}

impl From<u8> for KeyModifiers {
    fn from(value: u8) -> Self {
        Self {
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Win32ControlKeyState {
    pub value: u16,
}

impl From<u16> for Win32ControlKeyState {
    fn from(value: u16) -> Self {
        Self {
            value,
        }
    }
}

impl Win32ControlKeyState {
    pub const RIGHT_ALT_PRESSED: u16 = 0x0001;
    pub const LEFT_ALT_PRESSED: u16 = 0x0002;
    pub const RIGHT_CTRL_PRESSED: u16 = 0x0004;
    pub const LEFT_CTRL_PRESSED: u16 = 0x0008;
    pub const SHIFT_PRESSED: u16 = 0x0010;
    pub const NUMLOCK_ON: u16 = 0x0020;
    pub const SCROLLLOCK_ON: u16 = 0x0040;
    pub const CAPSLOCK_ON: u16 = 0x0080;
    pub const ENHANCED_KEY: u16 = 0x0100;
}

impl Into<KeyModifiers> for Win32ControlKeyState {
    fn into(self) -> KeyModifiers {
        let mut value = 0;
        if self.value & Self::SHIFT_PRESSED != 0 {
            value |= KeyModifiers::SHIFT;
        }
        if self.value & Self::LEFT_ALT_PRESSED != 0 || self.value & Self::RIGHT_ALT_PRESSED != 0 {
            value |= KeyModifiers::ALT;
        }
        if self.value & Self::LEFT_CTRL_PRESSED != 0 || self.value & Self::RIGHT_CTRL_PRESSED != 0 {
            value |= KeyModifiers::CTRL;
        }
        KeyModifiers::from(value)
    }
}

impl KeyModifiers {
    pub const SHIFT:     u8 = 0b1;         // (1)
    pub const ALT:       u8 = 0b10;        // (2)
    pub const CTRL:      u8 = 0b100;       // (4)
    pub const SUPER:     u8 = 0b1000;      // (8)
    pub const HYPER:     u8 = 0b10000;     // (16)
    pub const META:      u8 = 0b100000;    // (32)
    pub const CAPS_LOCK: u8 = 0b1000000;   // (64)
    pub const NUM_LOCK:  u8 = 0b10000000;  // (128)

    pub fn shift_key(&self) -> bool {
        self.value & Self::SHIFT != 0
    }
    pub fn alt_key(&self) -> bool {
        self.value & Self::ALT != 0
    }
    pub fn ctrl_key(&self) -> bool {
        self.value & Self::CTRL != 0
    }
    pub fn super_key(&self) -> bool {
        self.value & Self::SUPER != 0
    }
    pub fn hyper_key(&self) -> bool {
        self.value & Self::HYPER != 0
    }
    pub fn meta_key(&self) -> bool {
        self.value & Self::META != 0
    }
}

#[derive(Clone, Debug)]
struct KeyEventInfo {
    unicode_value: char,
    modifiers: KeyModifiers,
}

impl KeyEventInfo {
    pub fn new(key: char) -> Self {
        Self {
            unicode_value: key,
            modifiers: KeyModifiers::default(),
        }
    }
    pub fn key(&self) -> char {
        self.unicode_value
    }
    pub fn with_modifiers(mut self, modifiers: impl Into<KeyModifiers>) -> Self {
        self.modifiers = modifiers.into();
        self
    }
    pub fn modifiers(&self) -> KeyModifiers {
        self.modifiers.clone()
    }
}

async fn handle_key_press(state_container: StateContainer, info: KeyEventInfo) -> anyhow::Result<()> {
    tracing::debug!("Key press: {:?}", info);
    
    if info.key() == 'q' && info.modifiers().alt_key() {
        kill_active_span(state_container.clone()).await?;
    }
    else if info.key() == 'n' && info.modifiers().alt_key() {
        create_process(state_container.clone()).await?;
    }
    else if info.key() == '\x1b' {
        let data = "\x1b".as_bytes();
        write_input(state_container.clone(), data, true).await?;
    }
    else if info.modifiers() == KeyModifiers::default() {
        let new_string = format!("{}", info.key());
        let state = state_container.clone();
        write_input(state, new_string.as_bytes(), true).await?;
    }

    Ok(())
}

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
                        tracing::debug!("Sending mouse event: position: {:?} button: {:?} is_release: {:?}, encoding: {:?}", position, button, is_release, encoding);
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

fn get_param(params: &[Vec<u16>], first: usize, second: usize) -> u16 {
    *params.get(first).unwrap_or(&Vec::new()).get(second).unwrap_or(&0)
}

impl vte::Perform for Performer {
    fn print(&mut self, char: char) {
        tracing::debug!("[PRINT] {:?}", char);
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
        let clone = self.state_container.clone();
        let info = KeyEventInfo::new(char);
        self.run(|| async move {
            handle_key_press(clone, info).await
        });
    }
    fn execute(&mut self, byte: u8) {
        tracing::debug!("[EXECUTE] {:?}", byte);
        let bytes = [byte];
        let state = self.state_container.clone();
        self.run(|| async move { write_input(state, &bytes, true).await });
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
        tracing::debug!(
            "[CSI] Params: {:?}, int: {:?}, action: {:?}",
            &params,
            &intermediates,
            action
        );
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
                return write_input(state.clone(), &bytes, true).await;
            }

            if action == '_' {
                let virtual_key_code = get_param(&owned_params, 0, 0);
                let virtual_scan_code = get_param(&owned_params, 1, 0);
                let utf16_point = get_param(&owned_params, 2, 0);
                let key_down = get_param(&owned_params, 3, 0);
                let control_key_state = get_param(&owned_params, 4, 0);
                let repeat_count = get_param(&owned_params, 5, 0);
                let is_key_down = repeat_count > 0;

                if virtual_key_code == 0 && virtual_scan_code == 0 && is_key_down == false {
                    return Ok(());
                }
                let unicode_key = char::from_u32(utf16_point as u32).unwrap_or_default();

                let modifiers = Win32ControlKeyState::from(control_key_state);
                let info = KeyEventInfo::new(unicode_key).with_modifiers(modifiers);

                handle_key_press(state, info).await?;
            };

            Ok(())
        });
        
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        tracing::debug!("Unknown escape code: {:?} {:?} {:?}", intermediates, byte, ignore);
    }

    fn terminated(&self) -> bool {
        false
    }
}

pub async fn handle_stdin(state_container: StateContainer) -> anyhow::Result<()> {
    let mut did_end_with_escape = false;

    let mut parser = vte::Parser::new();

    let mut performer = Performer::new(state_container.clone());

    loop {
        let stdin = state_container.state().stdin.clone();
        let mut stdin = stdin.lock().await;

        let mut buffer = [0; 1024];

        let timeout_result = timeout(Duration::from_millis(100), stdin.read(&mut buffer)).await;
        let Ok(result) = timeout_result else {
            if did_end_with_escape == true {
                handle_key_press(state_container.clone(), KeyEventInfo::new('\x1b')).await?;
            }
            did_end_with_escape = false;
            continue;
        };
        let len: usize = result?;
        if len == 0 {
            return Ok(());
        }
        //tracing::debug!("Read {} bytes: {:?}", len,String::from_utf8_lossy(&buffer[..len]));
        did_end_with_escape = buffer[len-1] == 0x1b;
        
        let mut index = 0;
        while index < len {
            let bytes = &buffer[index..index+1];
            parser.advance(&mut performer, &bytes);
            performer.block().await?;
            index = index + 1;
        }
    }
}

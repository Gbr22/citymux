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

impl <T: Into<KeyModifiers> + Clone> ToKeyModifiers for T {
    fn to_key_modifiers(&self) -> KeyModifiers {
        self.clone().into()
    }
}
impl From<u8> for KeyModifiers {
    fn from(value: u8) -> Self {
        Self {
            value,
        }
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
struct KeyPressInfo {
    key: char,
    modifiers: KeyModifiers,
}

trait ToKeyModifiers {
    fn to_key_modifiers(&self) -> KeyModifiers;
}

impl KeyPressInfo {
    pub fn new(key: char) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::default(),
        }
    }
    pub fn key(&self) -> char {
        self.key
    }
    pub fn with_modifiers(mut self, modifiers: &impl ToKeyModifiers) -> Self {
        self.modifiers = modifiers.to_key_modifiers();
        self
    }
    pub fn modifiers(&self) -> KeyModifiers {
        self.modifiers.clone()
    }
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
    fn on_key_press(&mut self, info: KeyPressInfo) {
        tracing::debug!("Key press: {:?}", info);
        let state_container = self.state_container.clone();
        self.run(|| async move {
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
        });
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
        self.on_key_press(KeyPressInfo::new(char));
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
        if intermediates.len() == 0 {
            self.on_key_press(KeyPressInfo::new(char::from(byte))
                .with_modifiers(&KeyModifiers::ALT));
        }
        else {
            tracing::debug!("Unknown escape code: {:?} {:?} {:?}", intermediates, byte, ignore);
        }
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
                performer.on_key_press(KeyPressInfo::new('\x1b'));
            }
            did_end_with_escape = false;
            continue;
        };
        let len: usize = result?;
        if len == 0 {
            return Ok(());
        }
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

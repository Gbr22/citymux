use renterm::{
    canvas::Canvas,
    cell::{Cell, CellValue},
    style::Style,
    surface::Surface,
    vector::Vector2,
};
use std::fmt::Debug;
use vt100::Parser;

pub struct TerminalInfo {
    size: Vector2,
    parser: Parser,
}

impl Debug for TerminalInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalInfo").finish()
    }
}

const MIN_TERMINAL_SIZE: Vector2 = Vector2 { x: 5, y: 5 };

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MouseProtocolMode {
    None,
    Press,
    PressRelease,
    ButtonMotion,
    AnyMotion,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MouseProtocolEncoding {
    Default,
    Utf8,
    Sgr,
}

impl From<vt100::MouseProtocolMode> for MouseProtocolMode {
    fn from(value: vt100::MouseProtocolMode) -> Self {
        match value {
            vt100::MouseProtocolMode::None => MouseProtocolMode::None,
            vt100::MouseProtocolMode::Press => MouseProtocolMode::Press,
            vt100::MouseProtocolMode::PressRelease => MouseProtocolMode::PressRelease,
            vt100::MouseProtocolMode::ButtonMotion => MouseProtocolMode::ButtonMotion,
            vt100::MouseProtocolMode::AnyMotion => MouseProtocolMode::AnyMotion,
        }
    }
}

impl From<vt100::MouseProtocolEncoding> for MouseProtocolEncoding {
    fn from(value: vt100::MouseProtocolEncoding) -> Self {
        match value {
            vt100::MouseProtocolEncoding::Default => MouseProtocolEncoding::Default,
            vt100::MouseProtocolEncoding::Utf8 => MouseProtocolEncoding::Utf8,
            vt100::MouseProtocolEncoding::Sgr => MouseProtocolEncoding::Sgr,
        }
    }
}

impl TerminalInfo {
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }
    pub fn application_keypad_mode(&self) -> bool {
        self.parser.screen().application_keypad()
    }
    pub fn mouse_protocol_mode(&self) -> MouseProtocolMode {
        self.parser.screen().mouse_protocol_mode().into()
    }
    pub fn mouse_protocol_encoding(&self) -> MouseProtocolEncoding {
        self.parser.screen().mouse_protocol_encoding().into()
    }
    pub fn new(size: Vector2) -> Self {
        let size = size.max(MIN_TERMINAL_SIZE);
        TerminalInfo {
            parser: vt100::Parser::new(size.y as u16, size.x as u16, 0),
            size,
        }
    }
    pub fn set_size(&mut self, size: Vector2) {
        let size = size.max(MIN_TERMINAL_SIZE);
        if self.size == size {
            return;
        }
        self.parser.set_size(size.y as u16, size.x as u16);
        self.size = size;
    }
    pub fn title(&self) -> String {
        self.parser.screen().title().to_string()
    }
    pub fn cursor_position(&self) -> Vector2 {
        let (y, x) = self.parser.screen().cursor_position();
        Vector2::new(x, y)
    }
    pub fn is_cursor_visible(&self) -> bool {
        !self.parser.screen().hide_cursor()
    }
    pub fn draw(&self, canvas: &mut impl Surface) {
        let screen = self.parser.screen();
        let (height, width) = screen.size();
        let size = Vector2::new(width, height);
        canvas.set_size(size);
        for y in 0..height {
            for x in 0..width {
                let position = (x, y).into();
                let cell = screen.cell(y, x);
                let Some(cell) = cell else {
                    let style = Style::default();
                    let value = CellValue::from(" ");
                    let cell = Cell::new_styled(value, style);
                    canvas.set_cell(position, cell);
                    continue;
                };
                let style = Style::default()
                    .with_background_color(cell.bgcolor())
                    .with_foreground_color(cell.fgcolor());
                let string_value = cell.contents();
                let string_value = if string_value.is_empty() {
                    " ".to_string()
                } else {
                    string_value
                };
                let value = CellValue::from(string_value);
                let cell = Cell::new_styled(value, style);
                canvas.set_cell(position, cell);
            }
        }
    }
    pub fn canvas(&self) -> Canvas {
        let mut canvas = Canvas::default();
        self.draw(&mut canvas);

        canvas
    }
}

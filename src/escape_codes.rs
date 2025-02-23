use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use crate::canvas::{Style, Vector2};

pub struct MoveCursor {
    y: isize,
    x: isize,
}

impl MoveCursor {
    pub fn new(y: isize, x: isize) -> Self {
        MoveCursor { y, x }
    }
}

impl From<Vector2> for MoveCursor {
    fn from(vector: Vector2) -> Self {
        MoveCursor { y: vector.y, x: vector.x }
    }
}

impl Into<Vec<u8>> for MoveCursor {
    fn into(self) -> Vec<u8> {
        let string = format!("\x1b[{};{}H", self.y+1, self.x+1);
        string.as_bytes().to_owned()
    }
}

pub struct SetAlternateScreenBuffer {
    is_enabled: bool,
}

impl SetAlternateScreenBuffer {
    pub fn new(value: bool) -> Self {
        SetAlternateScreenBuffer {
            is_enabled: value
        }
    }
    pub fn enable() -> Self {
        SetAlternateScreenBuffer::new(true)
    }
    pub fn disable() -> Self {
        SetAlternateScreenBuffer::new(false)
    }
}

impl Into<&[u8]> for SetAlternateScreenBuffer {
    fn into(self) -> &'static [u8] {
        match self.is_enabled {
            true => "\x1b[?1049h".as_bytes(),
            false => "\x1b[?1049l".as_bytes(),
        }
    }
}

pub struct EnableConcealMode {}

impl Default for EnableConcealMode {
    fn default() -> Self {
        EnableConcealMode {}
    }
}

impl Into<&[u8]> for EnableConcealMode {
    fn into(self) -> &'static [u8] {
        "\x1b[8m".as_bytes()
    }
}

pub struct DisableConcealMode {}

impl Default for DisableConcealMode {
    fn default() -> Self {
        DisableConcealMode {}
    }
}

impl Into<&[u8]> for DisableConcealMode {
    fn into(self) -> &'static [u8] {
        "\x1b[28m".as_bytes()
    }
}

pub struct RequestCursorPosition {}

impl Default for RequestCursorPosition {
    fn default() -> Self {
        RequestCursorPosition {}
    }
}

impl Into<&[u8]> for RequestCursorPosition {
    fn into(self) -> &'static [u8] {
        "\x1b[6n".as_bytes()
    }
}

pub struct EnableComprehensiveKeyboardHandling  {}

impl Default for EnableComprehensiveKeyboardHandling {
    fn default() -> Self {
        EnableComprehensiveKeyboardHandling {}
    }
}

impl Into<&[u8]> for EnableComprehensiveKeyboardHandling {
    fn into(self) -> &'static [u8] {
        "\x1b[>1u".as_bytes()
    }
}

pub struct EraseInDisplay  {
    value: u8,
}

impl Default for EraseInDisplay {
    fn default() -> Self {
        EraseInDisplay {
            value: 3
        }
    }
}

pub enum EraseInDisplayKind {
    FromCursorToEndOfScreen = 0,
    FromCursorToBeginningOfScreen = 1,
    EntireScreen = 2,
    EntireScreenAndScrollbackBuffer = 3,
}

impl EraseInDisplay {
    pub fn new(value: EraseInDisplayKind) -> Self {
        EraseInDisplay {
            value: value as u8
        }
    }
}

impl From<EraseInDisplayKind> for EraseInDisplay {
    fn from(kind: EraseInDisplayKind) -> Self {
        EraseInDisplay {
            value: kind as u8
        }
    }
}

impl Into<Vec<u8>> for EraseInDisplay {
    fn into(self) -> Vec<u8> {
        let string = format!("\x1b[{}J", self.value);
        string.as_bytes().to_owned()
    }
}

pub struct SetCursorVisibility {
    is_visible: bool,
}

impl SetCursorVisibility {
    pub fn new(value: bool) -> Self {
        SetCursorVisibility {
            is_visible: value
        }
    }
}

impl From<bool> for SetCursorVisibility {
    fn from(kind: bool) -> Self {
        SetCursorVisibility {
            is_visible: kind
        }
    }
}

impl Into<&[u8]> for SetCursorVisibility {
    fn into(self) -> &'static [u8] {
        if self.is_visible {
            "\x1b[?25h".as_bytes()
        } else {
            "\x1b[?25l".as_bytes()
        }
    }
}

pub struct ResetStyle {
    _private: (),
}
impl Default for ResetStyle {
    fn default() -> Self {
        ResetStyle {
            _private: ()
        }
    }
}
impl Into<&[u8]> for ResetStyle {
    fn into(self) -> &'static [u8] {
        "\x1b[0m".as_bytes()
    }
}

pub struct EraseCharacter {
    count: usize,
}

impl EraseCharacter {
    pub fn new(count: impl TryInto<usize>) -> Self {
        EraseCharacter {
            count: count.try_into().unwrap_or(0)
        }
    }
}

impl Into<Vec<u8>> for EraseCharacter {
    fn into(self) -> Vec<u8> {
        let string = format!("\x1b[{}X", self.count);
        string.as_bytes().to_owned()
    }
}

pub struct CursorForward {
    count: usize,
}
impl CursorForward {
    pub fn new(count: impl TryInto<usize>) -> Self {
        CursorForward {
            count: count.try_into().unwrap_or(0)
        }
    }
}
impl Into<Vec<u8>> for CursorForward {
    fn into(self) -> Vec<u8> {
        let string = format!("\x1b[{}C", self.count);
        string.as_bytes().to_owned()
    }
}

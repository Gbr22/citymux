use crate::canvas::Vector2;

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
        MoveCursor {
            y: vector.y,
            x: vector.x,
        }
    }
}

impl From<MoveCursor> for Vec<u8> {
    fn from(val: MoveCursor) -> Self {
        let string = format!("\x1b[{};{}H", val.y + 1, val.x + 1);
        string.as_bytes().to_owned()
    }
}

pub struct SetAlternateScreenBuffer {
    is_enabled: bool,
}

impl SetAlternateScreenBuffer {
    pub fn new(value: bool) -> Self {
        SetAlternateScreenBuffer { is_enabled: value }
    }
    pub fn enable() -> Self {
        SetAlternateScreenBuffer::new(true)
    }
    pub fn disable() -> Self {
        SetAlternateScreenBuffer::new(false)
    }
}

impl From<SetAlternateScreenBuffer> for &[u8] {
    fn from(val: SetAlternateScreenBuffer) -> Self {
        match val.is_enabled {
            true => "\x1b[?1049h".as_bytes(),
            false => "\x1b[?1049l".as_bytes(),
        }
    }
}

#[derive(Default)]
pub struct EnableConcealMode {}

impl From<EnableConcealMode> for &[u8] {
    fn from(val: EnableConcealMode) -> Self {
        "\x1b[8m".as_bytes()
    }
}

#[derive(Default)]
pub struct DisableConcealMode {}

impl From<DisableConcealMode> for &[u8] {
    fn from(val: DisableConcealMode) -> Self {
        "\x1b[28m".as_bytes()
    }
}

#[derive(Default)]
pub struct RequestCursorPosition {}

impl From<RequestCursorPosition> for &[u8] {
    fn from(val: RequestCursorPosition) -> Self {
        "\x1b[6n".as_bytes()
    }
}

#[derive(Default)]
pub struct EnableComprehensiveKeyboardHandling {}

impl From<EnableComprehensiveKeyboardHandling> for &[u8] {
    fn from(val: EnableComprehensiveKeyboardHandling) -> Self {
        "\x1b[>1u".as_bytes()
    }
}

pub struct EraseInDisplay {
    value: u8,
}

impl Default for EraseInDisplay {
    fn default() -> Self {
        EraseInDisplay { value: 3 }
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
        EraseInDisplay { value: value as u8 }
    }
}

impl From<EraseInDisplayKind> for EraseInDisplay {
    fn from(kind: EraseInDisplayKind) -> Self {
        EraseInDisplay { value: kind as u8 }
    }
}

impl From<EraseInDisplay> for Vec<u8> {
    fn from(val: EraseInDisplay) -> Self {
        let string = format!("\x1b[{}J", val.value);
        string.as_bytes().to_owned()
    }
}

pub struct SetCursorVisibility {
    is_visible: bool,
}

impl SetCursorVisibility {
    pub fn new(value: bool) -> Self {
        SetCursorVisibility { is_visible: value }
    }
}

impl From<bool> for SetCursorVisibility {
    fn from(kind: bool) -> Self {
        SetCursorVisibility { is_visible: kind }
    }
}

impl From<SetCursorVisibility> for &[u8] {
    fn from(val: SetCursorVisibility) -> Self {
        if val.is_visible {
            "\x1b[?25h".as_bytes()
        } else {
            "\x1b[?25l".as_bytes()
        }
    }
}

#[derive(Default)]
pub struct ResetStyle {
    _private: (),
}
impl From<ResetStyle> for &[u8] {
    fn from(val: ResetStyle) -> Self {
        "\x1b[0m".as_bytes()
    }
}

pub struct EraseCharacter {
    count: usize,
}

impl EraseCharacter {
    pub fn new(count: impl TryInto<usize>) -> Self {
        EraseCharacter {
            count: count.try_into().unwrap_or(0),
        }
    }
}

impl From<EraseCharacter> for Vec<u8> {
    fn from(val: EraseCharacter) -> Self {
        let string = format!("\x1b[{}X", val.count);
        string.as_bytes().to_owned()
    }
}

pub struct CursorForward {
    count: usize,
}
impl CursorForward {
    pub fn new(count: impl TryInto<usize>) -> Self {
        CursorForward {
            count: count.try_into().unwrap_or(0),
        }
    }
}
impl From<CursorForward> for Vec<u8> {
    fn from(val: CursorForward) -> Self {
        let string = format!("\x1b[{}C", val.count);
        string.as_bytes().to_owned()
    }
}

pub struct AllMotionTracking {
    is_enabled: bool,
}

impl AllMotionTracking {
    pub fn new(value: bool) -> Self {
        AllMotionTracking { is_enabled: value }
    }
}

impl From<AllMotionTracking> for &[u8] {
    fn from(val: AllMotionTracking) -> Self {
        match val.is_enabled {
            true => "\x1b[?1003h".as_bytes(),
            false => "\x1b[?1003l".as_bytes(),
        }
    }
}

pub struct SgrMouseHandling {
    is_enabled: bool,
}

impl SgrMouseHandling {
    pub fn new(value: bool) -> Self {
        SgrMouseHandling { is_enabled: value }
    }
}

impl From<SgrMouseHandling> for &[u8] {
    fn from(val: SgrMouseHandling) -> Self {
        match val.is_enabled {
            true => "\x1b[?1006h".as_bytes(),
            false => "\x1b[?1006l".as_bytes(),
        }
    }
}

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use crate::canvas::Vector2;

pub struct MoveCursor {
    y: usize,
    x: usize,
}

impl MoveCursor {
    pub fn new(y: usize, x: usize) -> Self {
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

pub struct EnterAlternateScreenBuffer {}

impl Default for EnterAlternateScreenBuffer {
    fn default() -> Self {
        EnterAlternateScreenBuffer {}
    }
}

impl Into<&[u8]> for EnterAlternateScreenBuffer {
    fn into(self) -> &'static [u8] {
        "\x1b[?1049h".as_bytes()
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

pub async fn get_cursor_position<R, W>(input: &mut R, output: &mut W) -> Result<(usize, usize), Box<dyn std::error::Error>>
    where
        R: AsyncRead + Unpin + ?Sized,
        W: AsyncWrite + Unpin + ?Sized
{
    output.write(EnableConcealMode::default().into()).await?;
    output.write(RequestCursorPosition::default().into()).await?;
    output.flush().await?;

    output.write("\n".as_bytes()).await?;
    println!("Requested cursor position");

    input.read_u8().await?; // Read the ESC character
    input.read_u8().await?; // Read the [ character

    let mut buf_reader = BufReader::new(input);
    let mut y = Vec::new();
    buf_reader.read_until(b';', &mut y).await?;
    let y = &y[0..y.len()-1];
    let y = String::from_utf8_lossy(&y).to_string();
    let y = usize::from_str_radix(&y, 10)?;
    let mut x = Vec::new();
    buf_reader.read_until(b'R', &mut x).await?;
    let x = &x[0..x.len()-1];
    let x = String::from_utf8_lossy(&x).to_string();
    let x = usize::from_str_radix(&x, 10)?;
    println!("Read x: {:?}, y: {:?}", x, y);

    Ok((y, x))
}
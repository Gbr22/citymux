use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, Sub};

use vt100::Parser;

use crate::encoding::{CsiSequence, OscSequence};

#[derive(Clone, Copy, Default, Debug, Eq)]
pub struct Vector2 {
    pub x: isize,
    pub y: isize,
}

impl From<Vector2> for Rect {
    fn from(value: Vector2) -> Self {
        Rect::new(Vector2::null(), value)
    }
}

impl Vector2 {
    pub fn null() -> Self {
        Vector2::new(0, 0)
    }
    pub fn max(self, other: Self) -> Self {
        Vector2 {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }
    pub fn min(self, other: Self) -> Self {
        Vector2 {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Rect {
    position: Vector2,
    size: Vector2,
}

impl Rect {
    pub fn contains(&self, vector: Vector2) -> bool {
        vector.x >= self.position().x
            && vector.y >= self.position().y
            && vector.x < self.position().x + self.size().x
            && vector.y < self.position().y + self.size().y
    }
    pub fn position(&self) -> Vector2 {
        self.position
    }
    pub fn top_left(&self) -> Vector2 {
        self.position()
    }
    pub fn bottom_right(&self) -> Vector2 {
        self.position() + self.size()
    }
    pub fn size(&self) -> Vector2 {
        self.size
    }
}

pub trait Drawable {
    fn draw(&self, canvas: &mut dyn Surface);
}

#[derive(Clone, Copy, Default, Debug)]
pub struct BorderSize {
    pub size: usize,
}

impl From<usize> for BorderSize {
    fn from(value: usize) -> Self {
        BorderSize { size: value }
    }
}

impl From<isize> for BorderSize {
    fn from(value: isize) -> Self {
        BorderSize {
            size: value.unsigned_abs(),
        }
    }
}

impl Sub<BorderSize> for Rect {
    type Output = Rect;

    fn sub(mut self, rhs: BorderSize) -> Self::Output {
        self.position.x += rhs.size as isize;
        self.position.y += rhs.size as isize;
        self.size.x -= rhs.size as isize;
        self.size.y -= rhs.size as isize;

        self
    }
}

impl Rect {
    pub fn new(position: Vector2, size: Vector2) -> Self {
        let size = size.max(Vector2::null());
        Rect { position, size }
    }
}

impl From<(isize, isize)> for Vector2 {
    fn from(value: (isize, isize)) -> Self {
        Vector2 {
            x: value.0,
            y: value.1,
        }
    }
}

impl Add for Vector2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vector2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Vector2 {
    pub const fn new(x: isize, y: isize) -> Self {
        Vector2 { x, y }
    }
}

impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct Canvas {
    cells: Vec<Cell>,
    size: Vector2,
}

pub struct CanvasView<'a> {
    canvas: Box<&'a mut dyn Surface>,
    rect: Rect,
}

impl<'a> CanvasView<'a> {
    fn is_position_in_rect(&self, position: Vector2) -> bool {
        if position.x < 0 || position.y < 0 {
            return false;
        }
        if position.x >= self.rect.size.x || position.y >= self.rect.size.y {
            return false;
        }
        true
    }
}

impl <'a> Surface for CanvasView<'a> {
    fn size(&self) -> Vector2 {
        self.rect.size
    }
    fn set_size(&mut self, size: Vector2) {
        self.rect.size = size;
    }
    fn get_cell(&self, position: Vector2) -> Cell {
        if !self.is_position_in_rect(position) {
            return Cell::default();
        }
        let position = position + self.rect.top_left();
        self.canvas.get_cell(position)
    }
    
    fn set_cell(&mut self, position: Vector2, cell: Cell) {
        if !self.is_position_in_rect(position) {
            return;
        }
        let position = position + self.rect.top_left();
        self.canvas.set_cell(position, cell);
    }
    
    fn iter_mut_cells(&mut self) -> std::slice::IterMut<'_, Cell> {
        self.canvas.iter_mut_cells()
    }
    fn to_sub_view(&mut self, rect: Rect) -> CanvasView {
        CanvasView { rect, canvas: Box::new(self) }
    }
}

pub trait Surface {
    fn size(&self) -> Vector2;
    fn set_size(&mut self, size: Vector2);
    fn get_cell(&self, position: Vector2) -> Cell;
    fn set_cell(&mut self, position: Vector2, cell: Cell);
    fn iter_mut_cells(&mut self) -> std::slice::IterMut<'_, Cell>;
    fn to_sub_view(&mut self, rect: Rect) -> CanvasView;
    fn to_view(&mut self) -> CanvasView {
        self.to_sub_view(Rect::new(Vector2::null(), self.size()))
    }
    fn draw(&mut self, drawable: &dyn Drawable) where Self: Sized {
        drawable.draw(self);
    }
    fn draw_at(&mut self, drawable: &dyn Drawable, position: Vector2) where Self: Sized {
        self.draw_in(drawable, Rect::new(position, self.size() - position));
    }
    fn draw_in(&mut self, drawable: &dyn Drawable, rect: Rect) where Self: Sized {
        let mut view = self.to_sub_view(rect);
        drawable.draw(&mut view);
    }
}

impl <'a> Into<Box<&'a dyn Surface>> for &'a Canvas {
    fn into(self) -> Box<&'a dyn Surface> {
        Box::new(self)
    }
}

impl <'a> Into<Box<&'a dyn Surface>> for &'a mut Canvas {
    fn into(self) -> Box<&'a dyn Surface> {
        Box::new(self)
    }
}

#[derive(Debug)]
pub struct DrawableStr<'a> {
    string: &'a str,
    style: Style
}

impl <'a> DrawableStr<'a> {
    pub fn new(string: &'a str, style: Style) -> Self {
        DrawableStr::<'a> { string, style }
    }
    pub fn size(&self) -> Vector2 {
        Vector2::new(self.string.len() as isize, 1)
    }
}

impl Drawable for DrawableStr<'_> {
    fn draw(&self, canvas: &mut dyn Surface) {
        let str = self.string;
        let chars = str.chars().collect::<Vec<char>>();
        let mut x = 0;
        for c in chars {
            canvas.set_cell((x, 0).into(), Cell::new_styled(c, self.style.clone()));
            x += 1;
        }
    }
}

impl Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Canvas");

        for y in 0..self.size.y {
            let mut row_content = String::new();
            for x in 0..self.size.x {
                row_content += &self.get_cell((x, y).into()).to_string();
            }
            s.field(&format!("row_{}", y), &row_content);
        }

        let mut map = HashMap::new();
        for y in 0..self.size.y {
            for x in 0..self.size.x {
                let cell = self.get_cell((x, y).into());
                let key = (x, y);
                map.insert(key, cell);
            }
        }

        s.finish()
    }
}

impl Surface for Canvas {
    fn iter_mut_cells(&mut self) -> std::slice::IterMut<'_, Cell> {
        self.cells.as_mut_slice().iter_mut()
    }
    fn size(&self) -> Vector2 {
        self.size
    }
    fn set_size(&mut self, size: Vector2) {
        if self.size == size {
            return;
        }
        let old_cells = self.cells.clone();
        let old_size = self.size;
        self.size = size;
        self.cells = vec![Cell::default(); isize::abs(size.x * size.y) as usize];
        for y in 0..isize::abs(size.y.min(old_size.y)) {
            for x in 0..isize::abs(size.x.min(old_size.x)) {
                let index = (y * old_size.x + x) as usize;
                if index >= old_cells.len() {
                    continue;
                }
                let cell = old_cells[index].clone();
                self.set_cell((x, y).into(), cell);
            }
        }
    }
    fn get_cell(&self, position: Vector2) -> Cell {
        let x = position.x;
        let y = position.y;

        if x < 0 || y < 0 {
            return Cell::default();
        }
        if position.x >= self.size.x || position.y >= self.size.y {
            return Cell::default();
        }
        let index = y * self.size.x + x;
        if self.cells.len() <= index as usize {
            return Cell::default();
        }

        self.cells[index as usize].clone()
    }
    fn set_cell(&mut self, position: Vector2, cell: Cell) {
        let x = position.x;
        let y = position.y;

        if x < 0 || y < 0 {
            return;
        }
        if position.x >= self.size.x || position.y >= self.size.y {
            return;
        }
        let index = y * self.size.x + x;
        if self.cells.len() <= index as usize {
            tracing::debug!(
                "Index out of bounds: {:?}, {}, {}, {}",
                cell,
                x,
                y,
                self.cells.len()
            );
            return;
        }

        self.cells[index as usize] = cell;
    }
    fn to_sub_view(&mut self, rect: Rect) -> CanvasView {
        let corner = rect.bottom_right();
        self.set_size(self.size.max(corner));
        CanvasView { canvas: Box::new(self), rect }
    }
}

impl <T: AsRef<str>> Drawable for T {
    fn draw(&self, canvas: &mut dyn Surface) {
        let str = self.as_ref();
        let chars = str.chars().collect::<Vec<char>>();
        let mut x = 0;
        for c in chars {
            canvas.set_cell((x, 0).into(), Cell::new_styled(c, Style::default()));
            x += 1;
        }
    }
}

impl Canvas {
    pub fn new(size: Vector2) -> Self {
        Self::new_filled(size, Cell::default())
    }
    pub fn new_filled(size: Vector2, cell: Cell) -> Self {
        let cells = vec![cell; isize::abs(size.x * size.y) as usize];
        Canvas { cells, size }
    }
}

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
            size,
            parser: vt100::Parser::new(size.y as u16, size.x as u16, 0),
        }
    }
    pub fn set_size(&mut self, size: Vector2) {
        let size = size.max(MIN_TERMINAL_SIZE);
        if self.size == size {
            return;
        }
        self.size = size;
        self.parser.set_size(size.y as u16, size.x as u16);
    }
    pub fn title(&self) -> String {
        self.parser.screen().title().to_string()
    }
    pub fn cursor_position(&self) -> Vector2 {
        let (y, x) = self.parser.screen().cursor_position();
        Vector2::new(x as isize, y as isize)
    }
    pub fn is_cursor_visible(&self) -> bool {
        !self.parser.screen().hide_cursor()
    }
    pub fn draw(&self, canvas: &mut impl Surface) {
        let screen = self.parser.screen();
        let (height, width) = screen.size();
        let size = Vector2::new(width as isize, height as isize);
        canvas.set_size(size);
        for y in 0..height {
            for x in 0..width {
                let position = (x as isize, y as isize).into();
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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Style {
    foreground_color: Color,
    background_color: Color,
    is_bold: bool,
    is_italic: bool,
}

impl Style {
    pub fn background_color(&self) -> Color {
        self.background_color.clone()
    }
    pub fn foreground_color(&self) -> Color {
        self.foreground_color.clone()
    }
    pub fn with_background_color(&self, color: impl Into<Color>) -> Self {
        let mut style = self.clone();
        style.background_color = color.into();
        style
    }
    pub fn with_foreground_color(&self, color: impl Into<Color>) -> Self {
        let mut style = self.clone();
        style.foreground_color = color.into();
        style
    }
}

impl From<Style> for Vec<u8> {
    fn from(val: Style) -> Self {
        let mut bytes = Vec::new();
        let bg = val.background_color();
        let fg = val.foreground_color();
        bytes.extend(bg.to_vec(ColorType::Background));
        bytes.extend(fg.to_vec(ColorType::Foreground));

        bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Color {
    color: ColorEnum,
}

impl From<vt100::Color> for Color {
    fn from(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => Color::default(),
            vt100::Color::Rgb(r, g, b) => Color::new_rgb(r, g, b),
            vt100::Color::Idx(value) => Color::new_one_byte(value),
        }
    }
}

impl Color {
    pub fn new_one_byte(byte: u8) -> Self {
        Color {
            color: ColorEnum::OneByte(byte),
        }
    }
    pub fn new_rgb(r: u8, g: u8, b: u8) -> Self {
        Color {
            color: ColorEnum::Rgb(r, g, b),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorEnum {
    Default,
    OneByte(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorType {
    Foreground,
    Background,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            color: ColorEnum::Default,
        }
    }
}

impl Color {
    pub fn to_vec(&self, color_type: ColorType) -> Vec<u8> {
        let prefix = match color_type {
            ColorType::Foreground => 30,
            ColorType::Background => 40,
        };
        let mut bytes = Vec::new();

        bytes.extend("\x1b[".as_bytes());

        match &self.color {
            ColorEnum::Default => {
                bytes.extend((prefix + 9).to_string().as_bytes());
            }
            ColorEnum::OneByte(value) => {
                if (0..=7).contains(value) {
                    bytes.extend((prefix + value).to_string().as_bytes());
                } else if (8..=15).contains(value) {
                    bytes.extend((60 + prefix + value - 8).to_string().as_bytes());
                } else {
                    bytes.extend((prefix + 8).to_string().as_bytes());
                    bytes.extend(";5;".as_bytes());
                    bytes.extend(value.to_string().as_bytes());
                }
            }
            ColorEnum::Rgb(r, g, b) => {
                bytes.extend((prefix + 8).to_string().as_bytes());
                bytes.extend(";2;".as_bytes());
                bytes.extend(r.to_string().as_bytes());
                bytes.extend(";".as_bytes());
                bytes.extend(g.to_string().as_bytes());
                bytes.extend(";".as_bytes());
                bytes.extend(b.to_string().as_bytes());
            }
            _ => {}
        }
        bytes.extend("m".as_bytes());

        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CellValueEnum {
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellValue {
    value: CellValueEnum,
}

impl CellValue {
    pub fn to_vec(&self) -> Vec<u8> {
        match &self.value {
            CellValueEnum::String(value) => value.as_bytes().to_vec(),
        }
    }
}

impl<T: Into<String>> From<T> for CellValue {
    fn from(value: T) -> Self {
        CellValue {
            value: CellValueEnum::String(value.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub value: CellValue,
    pub style: Style,
}

impl Cell {
    pub fn new(value: impl Into<CellValue>) -> Self {
        Cell {
            value: value.into(),
            style: Style::default(),
        }
    }
    pub fn new_styled(value: impl Into<CellValue>, style: Style) -> Self {
        Cell {
            value: value.into(),
            style,
        }
    }
    pub fn empty_styled(style: Style) -> Self {
        Cell {
            value: " ".into(),
            style,
        }
    }
    pub fn is_empty(&self) -> bool {
        match &self.value.value {
            CellValueEnum::String(value) => value == " ",
        }
    }
    pub fn to_string(&self) -> String {
        match &self.value.value {
            CellValueEnum::String(value) => value.clone(),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            value: " ".into(),
            style: Style::default(),
        }
    }
}

pub enum TerminalCommand {
    String(String),
    Csi(CsiSequence),
    Osc(OscSequence),
}

impl TerminalCommand {
    pub fn string(string: impl Into<String>) -> Self {
        TerminalCommand::String(string.into())
    }
    pub fn osc(osc_sequence: impl Into<OscSequence>) -> Self {
        TerminalCommand::Osc(osc_sequence.into())
    }
    pub fn csi(csi_sequence: impl Into<CsiSequence>) -> Self {
        TerminalCommand::Csi(csi_sequence.into())
    }
}

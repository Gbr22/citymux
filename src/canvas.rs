use std::collections::HashMap;
use std::ops::{Add, Sub};
use std::fmt::Debug;

use crate::encoding::{CsiSequence, OscSequence};

#[derive(Clone, Copy, Default, Debug)]
pub struct Vector2 {
    pub x: isize,
    pub y: isize,
}

impl From<(isize, isize)> for Vector2 {
    fn from(value: (isize, isize)) -> Self {
        Vector2 { x: value.0, y: value.1 }
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
    pub fn new(x: isize, y: isize) -> Self {
        Vector2 { x, y }
    }
}

impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[derive(Clone)]
pub struct Canvas {
    cells: Vec<Cell>,
    size: Vector2,
}

impl Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Canvas");

        for y in 0..self.size.y {
            let mut row_content = String::new();
            for x in 0..self.size.x {
                row_content += &self.get_cell((x, y)).to_string();
            }
            s.field(&format!("row_{}",y), &row_content);
        }

        let mut map = HashMap::new();
        for y in 0..self.size.y {
            for x in 0..self.size.x {
                let cell = self.get_cell((x, y));
                let key = (x,y);
                map.insert(key, cell);
            }
        }

        s.field("map", &format!("{:?}", map));
        s.finish()
    }
}

impl Canvas {
    pub fn iter_mut_cells(&mut self) -> std::slice::IterMut<'_, Cell> {
        self.cells.as_mut_slice().iter_mut()
    }
}

impl Canvas {
    pub fn size(&self) -> Vector2 {
        self.size
    }
    pub fn set_size(&mut self, size: Vector2) {
        let old_cells = self.cells.clone();
        let old_size = self.size;
        self.size = size;
        self.cells = vec![Cell::default(); isize::abs(size.x * size.y) as usize];
        for y in 0..isize::abs(size.y.min(old_size.y)) {
            for x in 0..isize::abs(size.x.min(old_size.x)) {
                let cell = old_cells[(y * old_size.x + x) as usize].clone();
                self.set_cell((x, y), cell);
            }
        }
    }
}

impl PartialEq for Canvas {
    fn eq(&self, other: &Self) -> bool {
        self.cells == other.cells && self.size == other.size
    }
}

impl From<String> for Canvas {
    fn from(value: String) -> Self {
        Canvas::from(value.as_str())
    }
}
impl From<&str> for Canvas {
    fn from(value: &str) -> Self {
        let chars = value.chars().collect::<Vec<char>>();
        let mut canvas = Canvas::new(Vector2 { x: chars.len() as isize, y: 1 });
        let mut x = 0;
        for c in value.chars() {
            canvas.set_cell((x, 0), Cell::new_styled(c, Style::default()));
            x += 1;
        }
        canvas
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
    pub fn put_canvas(&mut self, canvas: &Canvas, position: Vector2) {
        for y in 0..canvas.size.y {
            for x in 0..canvas.size.x {
                let pos = Vector2::new(x, y);
                let cell = canvas.get_cell(pos);
                self.set_cell( pos + position, cell);
            }
        }
    }
    pub fn get_cell(&self, position: impl Into<Vector2>) -> Cell {
        let position = position.into();
        let x = position.x;
        let y = position.y;

        if x < 0 || y < 0 {
            return Cell::default();
        }
        let index = y * self.size.x + x;
        if self.cells.len() <= index as usize {
            return Cell::default();
        }
    
        self.cells[index as usize].clone()
    }
    
    pub fn set_cell(&mut self, position: impl Into<Vector2>, cell: Cell) {
        let position = position.into();
        let x = position.x;
        let y = position.y;

        if x < 0 || y < 0 {
            return;
        }
        let index = y * self.size.x + x;
        if self.cells.len() <= index as usize {
            tracing::debug!("Index out of bounds: {:?}, {}, {}, {}", cell, x, y, self.cells.len());
            return;
        }
    
        self.cells[index as usize] = cell;
    }
}

#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub title: String,
    pub canvas: Canvas,
    pub cursor: Vector2,
    pub current_style: Style,
    pub is_wrap_state_pending: bool,
    pub is_cursor_visible: bool,
    pub is_bracketed_paste_mode_enabled: bool,
    pub is_application_key_mode_enabled: bool,
}

impl TerminalInfo {
    pub fn new(size: Vector2) -> Self {
        TerminalInfo {
            title: String::default(),
            canvas: Canvas::new(size),
            cursor: Vector2 { x: 0, y: 0 },
            current_style: Style::default(),
            is_wrap_state_pending: false,
            is_cursor_visible: true,
            is_bracketed_paste_mode_enabled: true,
            is_application_key_mode_enabled: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
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
    pub fn with_background_color(&self, color: Color) -> Self {
        let mut style = self.clone();
        style.background_color = color;
        style
    }
    pub fn with_foreground_color(&self, color: Color) -> Self {
        let mut style = self.clone();
        style.foreground_color = color;
        style
    }
}

impl Into<Vec<u8>> for Style {
    fn into(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let bg = self.background_color();
        let fg = self.foreground_color();
        bytes.extend(bg.to_vec(ColorType::Background));
        bytes.extend(fg.to_vec(ColorType::Foreground));
        
        return bytes;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    color: ColorEnum,
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

#[derive(Clone, Debug, PartialEq)]
pub enum ColorEnum {
    Default,
    OneByte(u8),
    Rgb(u8, u8, u8),
}

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
            },
            ColorEnum::OneByte(value) => {
                if (0..=7).contains(value) {
                    bytes.extend((prefix + value).to_string().as_bytes());
                }
                else if (8..=15).contains(value) {
                    bytes.extend((60 + prefix + value - 8).to_string().as_bytes());
                }
                else {
                    bytes.extend((prefix+8).to_string().as_bytes());
                    bytes.extend(";5;".as_bytes());
                    bytes.extend(value.to_string().as_bytes());
                }
            },
            ColorEnum::Rgb(r, g, b) => {
                bytes.extend((prefix+8).to_string().as_bytes());
                bytes.extend(";2;".as_bytes());
                bytes.extend(r.to_string().as_bytes());
                bytes.extend(";".as_bytes());
                bytes.extend(g.to_string().as_bytes());
                bytes.extend(";".as_bytes());
                bytes.extend(b.to_string().as_bytes());
            },
            _ => {},
        }
        bytes.extend("m".as_bytes());

        bytes
    }
}

impl Default for Style {
    fn default() -> Self {
        Style {
            foreground_color: Color::default(),
            background_color: Color::default(),
            is_bold: false,
            is_italic: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CellValueEnum {
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
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

impl <T: Into<String>> From<T> for CellValue {
    fn from(value: T) -> Self {
        CellValue { value: CellValueEnum::String(value.into()) }
    }
}

#[derive(Clone, Debug)]
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

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.style == other.style
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

impl TerminalInfo {
    pub fn set_cursor_y_wrap(&mut self, y: isize) {
        if y >= self.canvas.size.y {
            let mut diff = y - self.cursor.y;
            if diff > self.canvas.size.y {
                diff = self.canvas.size.y;
            }
            self.cursor.y = self.canvas.size.y-1;
            for y in 0..self.canvas.size.y-1 {
                for x in 0..self.canvas.size.x {
                    let cell = self.canvas.get_cell((x, y + diff));
                    self.canvas.set_cell((x, y), cell);
                }
            }
            for y in self.canvas.size.y-diff..self.canvas.size.y {
                for x in 0..self.canvas.size.x {
                    self.canvas.set_cell((x, y), Cell::default());
                }
            }
        } else {
            self.cursor.y = y;
        }
    }
    pub fn set_cursor_x_wrap(&mut self, x: isize) {
        self.cursor.x = x;
        if self.cursor.x >= self.canvas.size.x {
            self.cursor.x = 0;
            self.set_cursor_y_wrap(self.cursor.y + 1);
        }
        self.is_wrap_state_pending = false;
    }
    pub fn set_cursor_x_pending_wrap(&mut self, x: isize) {
        if x >= self.canvas.size.x {
            self.cursor.x = self.canvas.size.x-1;
            self.is_wrap_state_pending = true;
        } else {
            self.cursor.x = x;
            self.is_wrap_state_pending = false;
        }
    }
    pub fn set_cursor_x_no_wrap(&mut self, x: isize) {
        let x = isize::min(x, self.canvas.size.x-1);
        self.cursor.x = x;
        self.is_wrap_state_pending = false;
    }
    pub fn set_cursor_y_no_wrap(&mut self, y: isize) {
        let y = isize::min(y, self.canvas.size.y-1);
        self.cursor.y = y;
        self.is_wrap_state_pending = false;
    }
    pub fn execute_command(&mut self, command: TerminalCommand) {
        match command {
            TerminalCommand::String(c) => {
                match c.as_str() {
                    "\r" => {
                        self.set_cursor_x_wrap(0);
                    },
                    "\n" => {
                        self.set_cursor_x_wrap(0);
                        self.set_cursor_y_wrap(self.cursor.y + 1);
                    },
                    "\x08" => {
                        self.set_cursor_x_wrap(self.cursor.x - 1);
                        self.canvas.set_cell(self.cursor, Cell::empty_styled(self.current_style.clone()));
                    },
                    _ => {
                        if self.is_wrap_state_pending {
                            self.set_cursor_x_wrap(0);
                            self.set_cursor_y_wrap(self.cursor.y + 1);
                            self.is_wrap_state_pending = false;
                        }
                        self.canvas.set_cell(
                            self.cursor,
                            Cell::new_styled(c, self.current_style.clone())
                        );
                        self.set_cursor_x_pending_wrap(self.cursor.x + 1);
                    }
                }
            }
            TerminalCommand::Csi(csi_sequence) => {
                let string = csi_sequence.content_as_string();
                let last_char = string.as_bytes().last().unwrap_or(&0).to_owned();
                if "ABCD".as_bytes().contains(&last_char) {
                    let number = string[0..string.len()-1].parse::<usize>().unwrap_or(1);
                    let number = number as isize;
                    let function = string.as_bytes()[string.len()-1];
                    if function == b'A' {
                        self.set_cursor_y_no_wrap(self.cursor.y - number);
                    } else if function == b'B' {
                        self.set_cursor_y_no_wrap(self.cursor.y + number);
                    } else if function == b'C' {
                        self.set_cursor_x_no_wrap(self.cursor.x + number);
                    } else if function == b'D' {
                        self.set_cursor_x_no_wrap(self.cursor.x - number);
                    }
                    return;
                }
                if string.ends_with("G") {
                    let substring = string.trim_end_matches("G");
                    let number: isize = substring.parse::<isize>().unwrap_or(1) - 1;
                    self.set_cursor_x_no_wrap(number);
                    return;
                }
                if string == "K" || string == "0K" {
                    for x in self.cursor.x..self.canvas.size.x {
                        self.canvas.set_cell((x, self.cursor.y), Cell::empty_styled(self.current_style.clone()));
                    }
                    return;
                }
                if string == "2J" || string == "3J" {
                    for y in self.cursor.y..self.canvas.size.y {
                        for x in 0..self.canvas.size.x-1 {
                            self.canvas.set_cell((x, y), Cell::empty_styled(self.current_style.clone()));
                        }
                    }
                    self.set_cursor_x_wrap(0);
                    self.set_cursor_y_wrap(0);
                    return;
                }
                if string.ends_with("H") {
                    let substring = string.trim_end_matches("H");
                    let parts = substring.split(";");
                    let mut parts = parts.collect::<Vec<&str>>();
                    let x = (parts.pop().unwrap_or("1").parse::<usize>().unwrap_or(1) as isize)-1;
                    let y = (parts.pop().unwrap_or("1").parse::<usize>().unwrap_or(1) as isize)-1;
                    self.set_cursor_x_no_wrap(x);
                    self.set_cursor_y_no_wrap(y);
                    return;
                }
                if string.ends_with("X") {
                    let substring = string.trim_end_matches("X");
                    let number = substring.parse::<usize>().unwrap_or(1);
                    for x in 0..number {
                        self.canvas.set_cell(self.cursor+Vector2::new(x.try_into().unwrap_or(0), 0), Cell::empty_styled(self.current_style.clone()));
                    }
                    self.is_wrap_state_pending = false;
                    return;
                }
                if string == "?2004l" {
                    self.is_bracketed_paste_mode_enabled = false;
                    return;
                }
                if string == "?2004h" {
                    self.is_bracketed_paste_mode_enabled = true;
                    return;
                }
                if string == "?25l" {
                    self.is_cursor_visible = false;
                    return;
                }
                if string == "?25h" {
                    self.is_cursor_visible = true;
                    return;
                }
                if string == "?1h" {
                    self.is_application_key_mode_enabled = true;
                    return;
                }
                if string == "?1l" {
                    self.is_application_key_mode_enabled = false;
                    return;
                }
                if string == "4l" {
                    // Disable insert mode
                    return;
                }
                if string.ends_with("m") {
                    let substring = string.trim_end_matches("m");
                    let parts = substring.split(";");
                    let parts = parts.collect::<Vec<&str>>();
                    let arguments = parts.iter().map(|x| x.parse::<usize>().unwrap_or(0)).collect::<Vec<usize>>();
                    let first = arguments[0];
                    let mut style = self.current_style.clone();
                    let is_normal_options = (0..=107).contains(&first) && first != 38 && first != 48 && first != 58;
                    let first = arguments.first().unwrap_or(&0).to_owned();
                    if first == 38 || first == 48 {
                        let second = arguments.get(1).unwrap_or(&0).to_owned();
                        let color = if second == 5 {
                            let color = arguments.get(2).unwrap_or(&0).to_owned();
                            Color::new_one_byte(color.try_into().unwrap_or(0))
                        }
                        else if second == 2 {
                            let r = arguments.get(2).unwrap_or(&0).to_owned().try_into().unwrap_or(0);
                            let g = arguments.get(3).unwrap_or(&0).to_owned().try_into().unwrap_or(0);
                            let b = arguments.get(4).unwrap_or(&0).to_owned().try_into().unwrap_or(0);

                            Color::new_rgb(r, g, b)
                        }
                        else {
                            tracing::debug!("Unknown color, second argument: {:?}", second);
                            Color::default()
                        };
                        if first == 38 {
                            style = style.with_foreground_color(color);
                        } else if first == 48 {
                            style = style.with_background_color(color);
                        }
                        self.current_style = style;
                        return;
                    }
                    if is_normal_options {
                        for argument in arguments {
                            if argument == 0 {
                                style = Style::default();
                                self.current_style = style;
                                return;
                            }
                            if (30..=37).contains(&argument) {
                                style.foreground_color = Color::new_one_byte((argument - 30).try_into().unwrap_or(0));
                                self.current_style = style;
                                return;
                            }
                            if (90..=97).contains(&argument) {
                                style.foreground_color = Color::new_one_byte((argument - 90).try_into().unwrap_or(0));
                                self.current_style = style;
                                return;
                            }
                            if argument == 39 {
                                style.foreground_color = Color::default();
                                self.current_style = style;
                                return;
                            }
                            if (40..=47).contains(&argument) {
                                style.background_color = Color::new_one_byte((argument - 40).try_into().unwrap_or(0));
                                self.current_style = style;
                                return;
                            }
                            if (100..=107).contains(&argument) {
                                style.background_color = Color::new_one_byte((argument - 100).try_into().unwrap_or(0));
                                self.current_style = style;
                                return;
                            }
                            if first == 49 {
                                style.background_color = Color::default();
                                self.current_style = style;
                                return;
                            }
                        }
                        self.current_style = style;
                        return;
                    }
                }
                tracing::debug!("Unknown CSI sequence: {:?}, {:?}", csi_sequence.content(), csi_sequence.content_as_string());
            },
            TerminalCommand::Osc(osc_sequence) => {
                let string = osc_sequence.content_as_string();
                if string.starts_with("0;") {
                    self.title = osc_sequence.content_as_string().split(";").collect::<Vec<&str>>()[1].to_string();
                    return;
                }
                if string == "" {
                    // Ignore empty string
                    return;
                }
                tracing::debug!("Unknown OSC sequence: {:?}, {:?}", osc_sequence.content(), osc_sequence.content_as_string());
            },
        }
    }
}

use std::ops::{Add, Sub};

use crate::encoding::{CsiSequence, OscSequence};

#[derive(Clone, Copy, Default, Debug)]
pub struct Vector2 {
    pub x: isize,
    pub y: isize,
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

pub struct Canvas {
    pub cells: Vec<Cell>,
    size: Vector2,
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
                self.set_cell(x, y, cell);
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
            canvas.set_cell(x, 0, Cell {
                value: format!("{}", c),
                color: Color::default(),
            });
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
                let cell = canvas.get_cell(x, y);
                self.set_cell( x + position.x, y + position.y, cell);
            }
        }
    }
    pub fn get_cell(&self, x: isize, y: isize) -> Cell {
        if x < 0 || y < 0 {
            return Cell::default();
        }
        let index = y * self.size.x + x;
        if self.cells.len() <= index as usize {
            return Cell::default();
        }
    
        self.cells[index as usize].clone()
    }
    
    pub fn set_cell(&mut self, x: isize, y: isize, cell: Cell) {
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

pub struct TerminalInfo {
    pub title: String,
    pub canvas: Canvas,
    pub cursor: Vector2,
}

impl TerminalInfo {
    pub fn new(size: Vector2) -> Self {
        TerminalInfo {
            title: String::default(),
            canvas: Canvas::new(size),
            cursor: Vector2 { x: 0, y: 0 },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Color {}

impl Default for Color {
    fn default() -> Self {
        Color {}
    }
}

impl PartialEq for Color {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub value: String,
    pub color: Color,
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.color == other.color
    }
}

impl Cell {
    pub fn new(value: impl Into<String>) -> Self {
        Cell {
            value: value.into(),
            color: Color::default(),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            value: " ".to_string(),
            color: Color::default(),
        }
    }
}

pub enum TerminalCommand {
    String(String),
    Csi(CsiSequence),
    Osc(OscSequence),
}

impl TerminalInfo {
    pub fn set_cursor_y(&mut self, y: isize) {
        if y >= self.canvas.size.y {
            let mut diff = y - self.cursor.y;
            if diff > self.canvas.size.y {
                diff = self.canvas.size.y;
            }
            self.cursor.y = self.canvas.size.y-1;
            for y in 0..self.canvas.size.y-1 {
                for x in 0..self.canvas.size.x {
                    let cell = self.canvas.get_cell(x, y + diff);
                    self.canvas.set_cell(x, y, cell);
                }
            }
            for y in self.canvas.size.y-diff..self.canvas.size.y {
                for x in 0..self.canvas.size.x {
                    self.canvas.set_cell(x, y, Cell::default());
                }
            }
        } else {
            self.cursor.y = y;
        }
    }
    pub fn set_cursor_x(&mut self, x: isize) {
        self.cursor.x = x;
        if self.cursor.x >= self.canvas.size.x {
            self.cursor.x = 0;
            self.set_cursor_y(self.cursor.y + 1);
        }
    }
    pub fn execute_command(&mut self, command: TerminalCommand) {
        match command {
            TerminalCommand::String(c) => {
                match c.as_str() {
                    "\r" => {
                        self.set_cursor_x(0);
                    },
                    "\n" => {
                        self.set_cursor_x(0);
                        self.set_cursor_y(self.cursor.y + 1);
                    },
                    "\x08" => {
                        self.set_cursor_x(self.cursor.x - 1);
                        self.canvas.set_cell(self.cursor.x, self.cursor.y, Cell {
                            value: " ".to_string(),
                            color: Color::default(),
                        });
                    },
                    _ => {
                        self.canvas.set_cell(self.cursor.x, self.cursor.y, Cell {
                            value: format!("{}", c),
                            color: Color::default(),
                        });

                        self.set_cursor_x(self.cursor.x + 1);
                    }
                }
            }
            TerminalCommand::Csi(csi_sequence) => {
                let string = csi_sequence.content_as_string();
                if "ABCD".as_bytes().contains(&string.as_bytes()[string.len()-1]) {
                    let number = string[0..string.len()-1].parse::<usize>();
                    if let Ok(number) = number {
                        let number = number as isize;
                        let function = string.as_bytes()[string.len()-1];
                        if function == b'A' {
                            self.set_cursor_y(self.cursor.y - number);
                        } else if function == b'B' {
                            self.set_cursor_y(self.cursor.y + number);
                        } else if function == b'C' {
                            self.set_cursor_x(self.cursor.x + number);
                        } else if function == b'D' {
                            self.set_cursor_x(self.cursor.x - number);
                        }
                        return;
                    }
                }
                if string == "K" || string == "0K" {
                    for x in self.cursor.x..self.canvas.size.x-1 {
                        self.canvas.set_cell(x, self.cursor.y, Cell::default());
                    }
                    return;
                }
                if string.ends_with("H") {
                    let substring = string.trim_end_matches("H");
                    let parts = substring.split(";");
                    let mut parts = parts.collect::<Vec<&str>>();
                    let x = (parts.pop().unwrap_or("1").parse::<usize>().unwrap_or(1) as isize)-1;
                    let y = (parts.pop().unwrap_or("1").parse::<usize>().unwrap_or(1) as isize)-1;
                    self.set_cursor_x(x);
                    self.set_cursor_y(y);
                    return;
                }
                tracing::debug!("Unknown CSI sequence: {:?}, {:?}", csi_sequence.content(), csi_sequence.content_as_string());
            },
            TerminalCommand::Osc(osc_sequence) => {
                let string = osc_sequence.content_as_string();
                if string.starts_with("0;") {
                    self.title = osc_sequence.content_as_string().split(";").collect::<Vec<&str>>()[1].to_string();
                    return;
                }
                tracing::debug!("Unknown OSC sequence: {:?}, {:?}", osc_sequence.content(), osc_sequence.content_as_string());
            },
        }
    }
}

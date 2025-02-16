use crate::encoding::CsiSequence;

#[derive(Clone, Copy, Default, Debug)]
pub struct Vector2 {
    pub x: usize,
    pub y: usize,
}

pub struct Canvas {
    pub cells: Vec<Cell>,
    pub size: Vector2,
    pub cursor: Vector2,
}

impl Canvas {
    pub fn new(size: Vector2) -> Self {
        let cells = vec![Cell::default(); size.x * size.y];
        Canvas {
            cells,
            size,
            cursor: Vector2 { x: 0, y: 0 },
        }
    }
}

#[derive(Clone)]
pub struct Color {}

impl Default for Color {
    fn default() -> Self {
        Color {}
    }
}

#[derive(Clone)]
pub struct Cell {
    pub value: String,
    pub color: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            value: " ".to_string(),
            color: Color::default(),
        }
    }
}

pub fn get_cell(state: &Canvas, x: usize, y: usize) -> Cell {
    let index = y * state.size.x + x;
    if state.cells.len() <= index {
        return Cell::default();
    }

    state.cells[index].clone()
}

pub fn set_cell(state: &mut Canvas, x: usize, y: usize, cell: Cell) {
    let index = y * state.size.x + x;
    if state.cells.len() <= index {
        return;
    }

    state.cells[index] = cell;
}

pub enum CanvasCommand {
    String(String),
    Csi(CsiSequence)
}

impl Canvas {
    pub fn set_cursor_y(&mut self, y: usize) {
        if y >= self.size.y {
            let diff = y - self.cursor.y;
            self.cursor.y = self.size.y-1;
            for y in 0..self.size.y-1 {
                for x in 0..self.size.x {
                    let cell = get_cell(self, x, y + diff);
                    set_cell(self, x, y, cell);
                }
            }
            for y in self.size.y-diff..self.size.y {
                for x in 0..self.size.x {
                    set_cell(self, x, y, Cell::default());
                }
            }
        } else {
            self.cursor.y = y;
        }
    }
    pub fn set_cursor_x(&mut self, x: usize) {
        self.cursor.x = x;
        if self.cursor.x >= self.size.x {
            self.cursor.x = 0;
            self.set_cursor_y(self.cursor.y + 1);
        }
    }
    pub fn execute_command(&mut self, command: CanvasCommand) {
        match command {
            CanvasCommand::String(c) => {
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
                        set_cell(self, self.cursor.x, self.cursor.y, Cell {
                            value: " ".to_string(),
                            color: Color::default(),
                        });
                    },
                    _ => {
                        set_cell(self, self.cursor.x, self.cursor.y, Cell {
                            value: format!("{}", c),
                            color: Color::default(),
                        });
                        self.set_cursor_x(self.cursor.x + 1);
                    }
                }
            }
            CanvasCommand::Csi(csi_sequence) => {
                let string = csi_sequence.content_as_string();
                if "ABCD".as_bytes().contains(&string.as_bytes()[string.len()-1]) {
                    let number = string[0..string.len()-1].parse::<usize>();
                    if let Ok(number) = number {
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
                tracing::debug!("Unknown CSI sequence: {:?}, {:?}", csi_sequence.content(), csi_sequence.content_as_string());
            },
        }
    }
}
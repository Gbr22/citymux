use std::collections::HashMap;
use std::fmt::Debug;

use super::cell::Cell;
use super::rect::Rect;
use super::view::SurfaceView;
use super::surface::Surface;
use super::vector::Vector2;

#[derive(Clone, PartialEq, Eq, Default)]
pub struct Canvas {
    cells: Vec<Cell>,
    size: Vector2,
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
            return;
        }

        self.cells[index as usize] = cell;
    }
    fn to_sub_view(&mut self, rect: Rect) -> SurfaceView {
        let corner = rect.bottom_right();
        self.set_size(self.size.max(corner));
        let mut view = SurfaceView::from(Box::new(self as &mut dyn Surface));
        view.set_rect(rect);

        view
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

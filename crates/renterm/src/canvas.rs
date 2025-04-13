use std::collections::HashMap;
use std::fmt::Debug;

use num_traits::AsPrimitive;

use crate::scalar::Scalar;
use crate::DefaultScalar;

use super::cell::Cell;
use super::rect::Rect;
use super::view::SurfaceView;
use super::surface::Surface;
use super::vector::Vector2;

#[derive(Clone, PartialEq, Eq, Default)]
pub struct Canvas<S: Scalar = DefaultScalar> {
    cells: Vec<Cell>,
    size: Vector2<S>,
}

impl <'a, S: Scalar> Into<Box<&'a dyn Surface<S>>> for &'a Canvas<S> {
    fn into(self) -> Box<&'a dyn Surface<S>> {
        Box::new(self)
    }
}

impl <'a, S: Scalar> Into<Box<&'a dyn Surface<S>>> for &'a mut Canvas<S> {
    fn into(self) -> Box<&'a dyn Surface<S>> {
        Box::new(self)
    }
}

impl <S: Scalar> Debug for Canvas<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Canvas");

        for y in 0..self.size.y.as_() {
            let mut row_content = String::new();
            for x in 0..self.size.x.as_() {
                row_content += &self.get_cell(Vector2::new(S::from_usize(x).unwrap(), S::from_usize(y).unwrap())).to_string();
            }
            s.field(&format!("row_{}", y), &row_content);
        }

        let mut map = HashMap::new();
        for y in 0.as_()..self.size.y.as_() {
            for x in 0.as_()..self.size.x.as_() {
                let cell = self.get_cell(Vector2::new(S::from_usize(x).unwrap(), S::from_usize(y).unwrap()));
                let key = (x, y);
                map.insert(key, cell);
            }
        }

        s.finish()
    }
}

impl <S: Scalar> Surface<S> for Canvas<S> {
    fn size(&self) -> Vector2<S> {
        self.size.clone()
    }
    fn set_size(&mut self, size: Vector2<S>) {
        if self.size == size {
            return;
        }
        let old_cells = self.cells.clone();
        let old_size = self.size.clone();
        self.size = size;
        self.cells = vec![Cell::default(); S::abs(self.size.x * self.size.y).as_()];
        for y in 0..S::abs(self.size.y.min(old_size.y)).as_() {
            for x in 0..S::abs(self.size.x.clone().min(old_size.x)).as_() {
                let index = y * old_size.x.as_() + x;
                if index >= old_cells.len() {
                    continue;
                }
                let cell = old_cells[index].clone();
                self.set_cell(Vector2::<S>::new(S::from_usize(x).unwrap(), S::from_usize(y).unwrap()), cell);
            }
        }
    }
    fn get_cell(&self, position: Vector2<S>) -> Cell {
        let x = position.x;
        let y = position.y;

        if x < S::zero() || y < S::zero() {
            return Cell::default();
        }
        if position.x >= self.size.x || position.y >= self.size.y {
            return Cell::default();
        }
        let index = y * self.size.x + x;
        if self.cells.len() <= index.as_() {
            return Cell::default();
        }

        self.cells[index.as_()].clone()
    }
    fn set_cell(&mut self, position: Vector2<S>, cell: Cell) {
        let x = position.x;
        let y = position.y;

        if x < S::zero() || y < S::zero() {
            return;
        }
        if position.x >= self.size.x || position.y >= self.size.y {
            return;
        }
        let index = y * self.size.x + x;
        if self.cells.len() <= index.as_() {
            return;
        }

        self.cells[index.as_()] = cell;
    }
    fn to_sub_view(&mut self, rect: Rect<S>) -> SurfaceView<S> {
        let corner = rect.bottom_right();
        self.set_size(self.size.clone().max(corner));
        let mut view = SurfaceView::from(Box::new(self as &mut dyn Surface<S>));
        view.set_rect(rect);

        view
    }
}



impl <S: Scalar> Canvas<S> {
    pub fn new(size: Vector2<S>) -> Self {
        Self::new_filled(size, Cell::default())
    }
    pub fn new_filled(size: Vector2<S>, cell: Cell) -> Self {
        let cells = vec![cell; S::abs(size.x * size.y).as_()];
        Canvas { cells, size }
    }
}

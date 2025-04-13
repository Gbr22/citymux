use crate::{scalar::Scalar, DefaultScalar};

use super::{cell::Cell, rect::Rect, surface::Surface, vector::Vector2};

pub struct SurfaceView<'a, S: Scalar = DefaultScalar> {
    canvas: Box<&'a mut dyn Surface<S>>,
    rect: Rect<S>,
}

impl <'a, S: Scalar> From<Box<&'a mut dyn Surface<S>>> for SurfaceView<'a, S> {
    fn from(canvas: Box<&'a mut dyn Surface<S>>) -> SurfaceView<'a, S> {
        let rect: Rect<S> = Rect::<S>::new(Vector2::<S>::null(), canvas.size());
        SurfaceView { rect, canvas }
    }
}

impl<'a, S: Scalar> SurfaceView<'a, S> {
    fn is_position_in_rect(&self, position: &Vector2<S>) -> bool {
        if position.x < S::zero() || position.y < S::zero() {
            return false;
        }
        if position.x >= self.rect.size().x || position.y >= self.rect.size().y {
            return false;
        }
        true
    }
    pub fn set_rect(&mut self, rect: Rect<S>) {
        self.rect = rect;
    }
}

impl <'a, S: Scalar> Surface<S> for SurfaceView<'a, S> {
    fn size(&self) -> Vector2<S> {
        self.rect.size()
    }
    fn set_size(&mut self, size: Vector2<S>) {
        self.rect.set_size(size);
    }
    fn get_cell(&self, position: Vector2<S>) -> Cell {
        if !self.is_position_in_rect(&position) {
            return Cell::default();
        }
        let position = position + self.rect.top_left();
        self.canvas.get_cell(position)
    }
    
    fn set_cell(&mut self, position: Vector2<S>, cell: Cell) {
        if !self.is_position_in_rect(&position) {
            return;
        }
        let position = position + self.rect.top_left();
        self.canvas.set_cell(position, cell);
    }

    fn to_sub_view(&mut self, rect: Rect<S>) -> SurfaceView<S> {
        SurfaceView { rect, canvas: Box::new(self) }
    }
}

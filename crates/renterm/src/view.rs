use super::{cell::Cell, rect::Rect, surface::Surface, vector::Vector2};

pub struct SurfaceView<'a> {
    canvas: Box<&'a mut dyn Surface>,
    rect: Rect,
}

impl <'a> From<Box<&'a mut dyn Surface>> for SurfaceView<'a> {
    fn from(canvas: Box<&'a mut dyn Surface>) -> Self {
        SurfaceView { rect: Rect::new(Vector2::null(), canvas.size()), canvas }
    }
}

impl<'a> SurfaceView<'a> {
    fn is_position_in_rect(&self, position: Vector2) -> bool {
        if position.x < 0 || position.y < 0 {
            return false;
        }
        if position.x >= self.rect.size().x || position.y >= self.rect.size().y {
            return false;
        }
        true
    }
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }
}

impl <'a> Surface for SurfaceView<'a> {
    fn size(&self) -> Vector2 {
        self.rect.size()
    }
    fn set_size(&mut self, size: Vector2) {
        self.rect.set_size(size);
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

    fn to_sub_view(&mut self, rect: Rect) -> SurfaceView {
        SurfaceView { rect, canvas: Box::new(self) }
    }
}

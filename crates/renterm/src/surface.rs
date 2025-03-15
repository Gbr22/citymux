use super::{cell::Cell, drawable::Drawable, rect::Rect, vector::Vector2, view::SurfaceView};

pub trait Surface {
    fn size(&self) -> Vector2;
    fn set_size(&mut self, size: Vector2);
    fn get_cell(&self, position: Vector2) -> Cell;
    fn set_cell(&mut self, position: Vector2, cell: Cell);
    fn to_sub_view(&mut self, rect: Rect) -> SurfaceView;
    fn to_view(&mut self) -> SurfaceView {
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

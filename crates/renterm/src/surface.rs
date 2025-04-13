use crate::{scalar::Scalar, DefaultScalar};

use super::{cell::Cell, drawable::Drawable, rect::Rect, vector::Vector2, view::SurfaceView};

pub trait Surface<S: Scalar = DefaultScalar> {
    fn size(&self) -> Vector2<S>;
    fn set_size(&mut self, size: Vector2<S>);
    fn get_cell(&self, position: Vector2<S>) -> Cell;
    fn set_cell(&mut self, position: Vector2<S>, cell: Cell);
    fn to_sub_view(&mut self, rect: Rect<S>) -> SurfaceView<S>;
    fn to_view(&mut self) -> SurfaceView<S> {
        self.to_sub_view(Rect::<S>::new(Vector2::<S>::null(), self.size()))
    }
    fn draw(&mut self, drawable: &dyn Drawable<S>) where Self: Sized {
        drawable.draw(self);
    }
    fn draw_at(&mut self, drawable: &dyn Drawable<S>, position: Vector2<S>) where Self: Sized {
        self.draw_in(drawable, Rect::<S>::new(position.clone(), self.size() - position));
    }
    fn draw_in(&mut self, drawable: &dyn Drawable<S>, rect: Rect<S>) where Self: Sized {
        let mut view = self.to_sub_view(rect);
        drawable.draw(&mut view);
    }
}

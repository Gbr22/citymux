use super::surface::Surface;

pub trait Drawable {
    fn draw(&self, canvas: &mut dyn Surface);
}


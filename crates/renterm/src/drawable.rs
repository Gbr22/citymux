use crate::{scalar::Scalar, DefaultScalar};

use super::surface::Surface;

pub trait Drawable<S: Scalar = DefaultScalar> {
    fn draw(&self, canvas: &mut dyn Surface<S>);
}

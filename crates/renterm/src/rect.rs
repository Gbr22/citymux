use std::ops::{Div, Sub};

use crate::scalar::Scalar;

use super::{border::BorderSize, vector::Vector2};

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Rect<T: Scalar = i32> {
    position: Vector2<T>,
    size: Vector2<T>,
}

impl <S: Scalar> Rect<S> {
    pub fn new(position: Vector2<S>, size: Vector2<S>) -> Rect<S> {
        let size = size.max(Vector2::<S>::null());
        Rect { position, size }
    }
    pub fn contains(&self, vector: Vector2<S>) -> bool {
        vector.x >= self.position().x
            && vector.y >= self.position().y
            && vector.x < self.position().x + self.size().x
            && vector.y < self.position().y + self.size().y
    }
    pub fn position(&self) -> Vector2<S> {
        self.position.clone()
    }
    pub fn top_left(&self) -> Vector2<S> {
        self.position()
    }
    pub fn bottom_right(&self) -> Vector2<S> {
        self.position() + self.size()
    }
    pub fn size(&self) -> Vector2<S> {
        self.size.clone()
    }
    pub fn set_size(&mut self, size: Vector2<S>) {
        self.size = size.max(Vector2::null());
    }
}

impl <S: Scalar> Div<S> for Rect<S> {
    type Output = Rect<S>;

    fn div(mut self, rhs: S) -> Self::Output {
        self.size = self.size.div(rhs);

        self
    }
}

impl Sub<BorderSize> for Rect {
    type Output = Rect;

    fn sub(mut self, rhs: BorderSize) -> Self::Output {
        self.position.x += rhs.size as i32;
        self.position.y += rhs.size as i32;
        self.size.x -= rhs.size as i32;
        self.size.y -= rhs.size as i32;

        self
    }
}

use std::ops::Sub;

use super::{border::BorderSize, vector::Vector2};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Rect {
    position: Vector2,
    size: Vector2,
}

impl Rect {
    pub const fn new(position: Vector2, size: Vector2) -> Self {
        let size = size.max(Vector2::null());
        Rect { position, size }
    }
    pub const fn contains(&self, vector: Vector2) -> bool {
        vector.x >= self.position().x
            && vector.y >= self.position().y
            && vector.x < self.position().x + self.size().x
            && vector.y < self.position().y + self.size().y
    }
    pub const fn position(&self) -> Vector2 {
        self.position
    }
    pub const fn top_left(&self) -> Vector2 {
        self.position()
    }
    pub const fn bottom_right(&self) -> Vector2 {
        self.position().add(self.size())
    }
    pub const fn size(&self) -> Vector2 {
        self.size
    }
    pub const fn set_size(&mut self, size: Vector2) {
        self.size = size.max(Vector2::null());
    }
}

impl Sub<BorderSize> for Rect {
    type Output = Rect;

    fn sub(mut self, rhs: BorderSize) -> Self::Output {
        self.position.x += rhs.size as isize;
        self.position.y += rhs.size as isize;
        self.size.x -= rhs.size as isize;
        self.size.y -= rhs.size as isize;

        self
    }
}

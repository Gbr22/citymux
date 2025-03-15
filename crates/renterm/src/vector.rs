use std::ops::{Add, Deref, Sub};

use super::rect::Rect;

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct Vector2 {
    pub x: isize,
    pub y: isize,
}

const fn const_max(a: isize, b: isize) -> isize {
    if a >= b { a } else { b }
}
const fn const_min(a: isize, b: isize) -> isize {
    if a <= b { a } else { b }
}

impl Vector2 {
    pub const fn new(x: isize, y: isize) -> Self {
        Vector2 { x, y }
    }
    pub const fn null() -> Self {
        Vector2::new(0, 0)
    }
    pub const fn max(self, other: Self) -> Self {
        Vector2 {
            x: const_max(self.x, other.x),
            y: const_max(self.y, other.y),
        }
    }
    pub const fn min(self, other: Self) -> Self {
        Vector2 {
            x: const_min(self.x, other.x),
            y: const_min(self.y, other.y),
        }
    }
}

impl From<Vector2> for Rect {
    fn from(value: Vector2) -> Self {
        Rect::new(Vector2::null(), value)
    }
}

impl From<(isize, isize)> for Vector2 {
    fn from(value: (isize, isize)) -> Self {
        Vector2 {
            x: value.0,
            y: value.1,
        }
    }
}

impl Vector2 {
    pub(crate) const fn add(self, other: Self) -> Self {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
    pub(crate) const fn sub(self, other: Self) -> Self {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Add for Vector2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::add(self, other)
    }
}

impl Sub for Vector2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::sub(self, other)
    }
}

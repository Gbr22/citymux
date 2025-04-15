use std::ops::{Add, Div, Sub};

use crate::{scalar::Scalar, DefaultScalar};

use super::rect::Rect;

#[derive(Default, Debug, Eq, PartialEq)]
pub struct Vector2<S: Scalar = DefaultScalar> {
    pub x: S,
    pub y: S,
}

impl <S: Scalar> Clone for Vector2<S> {
    fn clone(&self) -> Self {
        Vector2 {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }
}

impl <S: Scalar> Vector2<S> {
    pub fn new(x: impl Into<S>, y: impl Into<S>) -> Vector2<S> {
        Vector2 {
            x: x.into(),
            y: y.into(),
        }
    }
    pub fn null() -> Self {
        Vector2::<S>::new(S::zero(), S::zero())
    }
    pub fn max(self, other: Vector2<S>) -> Vector2<S> {
        Vector2 {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }
    pub fn min(self, other: Self) -> Self {
        Vector2 {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }
    pub fn signnum(self) -> Self {
        Vector2 {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }
}

impl <T: Scalar> From<Vector2<T>> for Rect<T> {
    fn from(value: Vector2<T>) -> Self {
        Rect::<T>::new(Vector2::<T>::null(), value)
    }
}

impl <T: Scalar, A: Into<T>, B: Into<T>>
From<(A, B)> for Vector2<T> {
    fn from(value: (A, B)) -> Self {
        Vector2::new(value.0, value.1)
    }
}

impl <S: Scalar> Add for Vector2<S> {
    type Output = Self;

    fn add(self, other: Vector2<S>) -> Vector2<S> {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl <S: Scalar> Sub for Vector2<S> {
    type Output = Self;

    fn sub(self, other: Vector2<S>) -> Vector2<S> {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl <S: Scalar> Div<S> for Vector2<S> {
    type Output = Vector2<S>;

    fn div(mut self, rhs: S) -> Self::Output {
        self.x = self.x.div(rhs);
        self.y = self.y.div(rhs);

        self
    }
}

impl <S: Scalar> Into<(S, S)> for Vector2<S> {
    fn into(self) -> (S, S) {
        (self.x, self.y)
    }
}

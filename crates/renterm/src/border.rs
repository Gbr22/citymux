use crate::{scalar::Scalar, DefaultScalar};

#[derive(Clone, Copy, Default, Debug)]
pub struct BorderSize<S: Scalar = DefaultScalar> {
    pub size: S,
}

impl <S: Scalar> From<S> for BorderSize<S> {
    fn from(value: S) -> Self {
        BorderSize { size: value.abs() }
    }
}

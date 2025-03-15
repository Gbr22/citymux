#[derive(Clone, Copy, Default, Debug)]
pub struct BorderSize {
    pub size: usize,
}

impl From<usize> for BorderSize {
    fn from(value: usize) -> Self {
        BorderSize { size: value }
    }
}

impl From<isize> for BorderSize {
    fn from(value: isize) -> Self {
        BorderSize {
            size: value.unsigned_abs(),
        }
    }
}

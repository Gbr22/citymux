use crate::scalar::Scalar;

use super::{cell::Cell, drawable::Drawable, style::Style, surface::Surface, vector::Vector2};

#[derive(Debug)]
pub struct DrawableStr<'a> {
    string: &'a str,
    style: Style
}

impl <'a> DrawableStr<'a> {
    pub fn new(string: &'a str, style: Style) -> Self {
        DrawableStr::<'a> { string, style }
    }
    pub fn size(&self) -> Vector2<usize> {
        Vector2::new(self.string.len(), 1 as usize)
    }
}

impl <S: Scalar> Drawable<S> for DrawableStr<'_> {
    fn draw(&self, canvas: &mut dyn Surface<S>) {
        let str = self.string;
        let chars = str.chars().collect::<Vec<char>>();
        let mut x: S = S::zero();
        for c in chars {
            canvas.set_cell((x, S::zero()).into(), Cell::new_styled(c, self.style.clone()));
            x = x + S::one();
        }
    }
}

impl <T: AsRef<str>, S: Scalar> Drawable<S> for T {
    fn draw(&self, canvas: &mut dyn Surface<S>) {
        let str = self.as_ref();
        let chars = str.chars().collect::<Vec<char>>();
        let mut x = S::zero();
        for c in chars {
            canvas.set_cell((x, S::zero()).into(), Cell::new_styled(c, Style::default()));
            x = x + S::one();
        }
    }
}

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
    pub fn size(&self) -> Vector2 {
        Vector2::new(self.string.len() as isize, 1)
    }
}

impl Drawable for DrawableStr<'_> {
    fn draw(&self, canvas: &mut dyn Surface) {
        let str = self.string;
        let chars = str.chars().collect::<Vec<char>>();
        let mut x = 0;
        for c in chars {
            canvas.set_cell((x, 0).into(), Cell::new_styled(c, self.style.clone()));
            x += 1;
        }
    }
}

impl <T: AsRef<str>> Drawable for T {
    fn draw(&self, canvas: &mut dyn Surface) {
        let str = self.as_ref();
        let chars = str.chars().collect::<Vec<char>>();
        let mut x = 0;
        for c in chars {
            canvas.set_cell((x, 0).into(), Cell::new_styled(c, Style::default()));
            x += 1;
        }
    }
}

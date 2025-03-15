use super::color::{Color, ColorType};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Style {
    foreground_color: Color,
    background_color: Color,
    is_bold: bool,
    is_italic: bool,
}

impl Style {
    pub fn background_color(&self) -> Color {
        self.background_color.clone()
    }
    pub fn foreground_color(&self) -> Color {
        self.foreground_color.clone()
    }
    pub fn with_background_color(&self, color: impl Into<Color>) -> Self {
        let mut style = self.clone();
        style.background_color = color.into();
        style
    }
    pub fn with_foreground_color(&self, color: impl Into<Color>) -> Self {
        let mut style = self.clone();
        style.foreground_color = color.into();
        style
    }
}

impl From<Style> for Vec<u8> {
    fn from(val: Style) -> Self {
        let mut bytes = Vec::new();
        let bg = val.background_color();
        let fg = val.foreground_color();
        bytes.extend(bg.to_vec(ColorType::Background));
        bytes.extend(fg.to_vec(ColorType::Foreground));

        bytes
    }
}

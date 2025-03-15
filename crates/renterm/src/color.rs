#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Color {
    color: ColorEnum,
}

#[cfg(feature = "vt100")]
impl From<vt100::Color> for Color {
    fn from(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => Color::default(),
            vt100::Color::Rgb(r, g, b) => Color::new_rgb(r, g, b),
            vt100::Color::Idx(value) => Color::new_one_byte(value),
        }
    }
}

impl Color {
    pub fn new_one_byte(byte: u8) -> Self {
        Color {
            color: ColorEnum::OneByte(byte),
        }
    }
    pub fn new_rgb(r: u8, g: u8, b: u8) -> Self {
        Color {
            color: ColorEnum::Rgb(r, g, b),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorEnum {
    Default,
    OneByte(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorType {
    Foreground,
    Background,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            color: ColorEnum::Default,
        }
    }
}

impl Color {
    pub fn to_vec(&self, color_type: ColorType) -> Vec<u8> {
        let prefix = match color_type {
            ColorType::Foreground => 30,
            ColorType::Background => 40,
        };
        let mut bytes = Vec::new();

        bytes.extend("\x1b[".as_bytes());

        match &self.color {
            ColorEnum::Default => {
                bytes.extend((prefix + 9).to_string().as_bytes());
            }
            ColorEnum::OneByte(value) => {
                if (0..=7).contains(value) {
                    bytes.extend((prefix + value).to_string().as_bytes());
                } else if (8..=15).contains(value) {
                    bytes.extend((60 + prefix + value - 8).to_string().as_bytes());
                } else {
                    bytes.extend((prefix + 8).to_string().as_bytes());
                    bytes.extend(";5;".as_bytes());
                    bytes.extend(value.to_string().as_bytes());
                }
            }
            ColorEnum::Rgb(r, g, b) => {
                bytes.extend((prefix + 8).to_string().as_bytes());
                bytes.extend(";2;".as_bytes());
                bytes.extend(r.to_string().as_bytes());
                bytes.extend(";".as_bytes());
                bytes.extend(g.to_string().as_bytes());
                bytes.extend(";".as_bytes());
                bytes.extend(b.to_string().as_bytes());
            }
        }
        bytes.extend("m".as_bytes());

        bytes
    }
}

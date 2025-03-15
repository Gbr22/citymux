use super::style::Style;

#[derive(Debug, Clone, PartialEq, Eq)]
enum CellValueEnum {
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellValue {
    value: CellValueEnum,
}

impl ToString for CellValue {
    fn to_string(&self) -> String {
        match &self.value {
            CellValueEnum::String(value) => value.clone(),
        }
    }
}

impl<T: Into<String>> From<T> for CellValue {
    fn from(value: T) -> Self {
        CellValue {
            value: CellValueEnum::String(value.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub value: CellValue,
    pub style: Style,
}

impl Cell {
    pub fn new(value: impl Into<CellValue>) -> Self {
        Cell {
            value: value.into(),
            style: Style::default(),
        }
    }
    pub fn new_styled(value: impl Into<CellValue>, style: Style) -> Self {
        Cell {
            value: value.into(),
            style,
        }
    }
    pub fn empty_styled(style: Style) -> Self {
        Cell {
            value: " ".into(),
            style,
        }
    }
    pub fn is_empty(&self) -> bool {
        match &self.value.value {
            CellValueEnum::String(value) => value == " ",
        }
    }
    pub fn to_string(&self) -> String {
        match &self.value.value {
            CellValueEnum::String(value) => value.clone(),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            value: " ".into(),
            style: Style::default(),
        }
    }
}

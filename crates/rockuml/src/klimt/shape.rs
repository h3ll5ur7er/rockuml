use super::font::FontConfiguration;

#[derive(Clone, Debug, PartialEq)]
pub enum UShape {
    Text(UText),
}

#[derive(Clone, Debug, PartialEq)]
pub struct UText {
    pub text: String,
    pub font: FontConfiguration,
    pub orientation: i32,
}

impl UText {
    pub fn new(text: &str, font: FontConfiguration) -> Self {
        Self {
            text: crate::jaws::make_newlines_visible(text),
            font,
            orientation: 0,
        }
    }
}

//! PlantUML's drawing layer (`klimt`): fonts, shapes, and the surfaces they are drawn on.

pub(crate) mod blocks;
pub(crate) mod debug;
pub(crate) mod font;
pub(crate) mod geom;
pub(crate) mod group;
pub(crate) mod png;
pub(crate) mod shape;
pub(crate) mod stencil;
pub(crate) mod svg;
pub(crate) mod typeface;
pub(crate) mod ugraphic;
pub(crate) mod url;
pub(crate) mod width_table;
mod width_table_data;

use crate::color::HColor;
use font::StringBounder;
use geom::XDimension2D;
use ugraphic::UGraphic;

/// Something that knows its size and can draw itself.
pub trait TextBlock {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D;

    fn draw_u(&self, ug: &UGraphic);

    /// Draws the block inside a border's padding. Blocks whose separators span them let those span the padding
    /// too.
    fn draw_in_padding(&self, ug: &UGraphic, _left: f64, _right: f64) {
        self.draw_u(ug);
    }

    /// The colour the block asks to be drawn on, which stacking blocks paint behind it.
    fn backcolor(&self) -> Option<HColor> {
        None
    }
}

impl<T: TextBlock + ?Sized> TextBlock for Box<T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        (**self).calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        (**self).draw_u(ug);
    }

    fn draw_in_padding(&self, ug: &UGraphic, left: f64, right: f64) {
        (**self).draw_in_padding(ug, left, right);
    }

    fn backcolor(&self) -> Option<HColor> {
        (**self).backcolor()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum HorizontalAlignment {
    #[default]
    Left,
    Center,
    Right,
}

impl HorizontalAlignment {
    /// Where something `width` wide starts when aligned in `available` space.
    pub(crate) fn offset(self, available: f64, width: f64) -> f64 {
        match self {
            Self::Left => 0.0,
            Self::Center => (available - width) / 2.0,
            Self::Right => available - width,
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "left" => Some(Self::Left),
            "center" => Some(Self::Center),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

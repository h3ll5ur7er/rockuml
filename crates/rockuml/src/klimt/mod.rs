//! PlantUML's drawing layer (`klimt`): fonts, shapes, and the surfaces they are drawn on.

pub mod blocks;
pub mod debug;
pub mod font;
pub mod geom;
pub mod group;
pub mod png;
pub mod shape;
pub mod stencil;
pub mod svg;
pub mod typeface;
pub mod ugraphic;
pub mod width_table;
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
pub enum HorizontalAlignment {
    #[default]
    Left,
    Center,
    Right,
}

impl HorizontalAlignment {
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Left, Self::Center, Self::Right]
            .into_iter()
            .find(|alignment| format!("{alignment:?}").eq_ignore_ascii_case(name))
    }
}

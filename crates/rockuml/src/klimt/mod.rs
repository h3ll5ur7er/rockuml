//! PlantUML's drawing layer (`klimt`): fonts, shapes, and the surfaces they are drawn on.

pub(crate) mod affine;
pub(crate) mod big_frame;
pub(crate) mod blocks;
pub(crate) mod clip;
pub(crate) mod debug;
pub(crate) mod fashion;
pub(crate) mod font;
pub(crate) mod geom;
pub(crate) mod group;
pub(crate) mod image;
pub(crate) mod limit_finder;
pub(crate) mod png;
pub(crate) mod shape;
pub(crate) mod sprite;
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

impl<T: TextBlock + ?Sized> TextBlock for &T {
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

/// Lays a line out at tab stops `tab_width` apart: calls `visit` with each piece between tabulations and its x
/// offset, and returns the width of the whole line.
pub(crate) fn layout_tabulated(
    line: &str,
    tab_width: f64,
    width_of: impl Fn(&str) -> f64,
    mut visit: impl FnMut(&str, f64),
) -> f64 {
    let is_tabulation = |c: char| c == '\t' || c == crate::jaws::BLOCK_E1_REAL_TABULATION;
    let mut x = 0.0;
    for piece in line.split_inclusive(is_tabulation) {
        let (text, tabulated) = match piece.strip_suffix(is_tabulation) {
            Some(text) => (text, true),
            None => (piece, false),
        };
        if !text.is_empty() {
            visit(text, x);
            x += width_of(text);
        }
        if tabulated {
            x += tab_width - x % tab_width;
        }
    }
    x
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabulations_advance_to_the_next_stop() {
        let mut pieces = Vec::new();
        let width = layout_tabulated(
            "	a	bc",
            4.0,
            |text| text.len() as f64,
            |piece, x| {
                pieces.push((piece.to_owned(), x));
            },
        );
        assert_eq!(pieces, [("a".to_owned(), 4.0), ("bc".to_owned(), 8.0)]);
        assert_eq!(width, 10.0);
    }
}

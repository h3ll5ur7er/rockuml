//! PlantUML's drawing layer (`klimt`): fonts, shapes, and the surfaces they are drawn on.

pub mod debug;
pub mod font;
pub mod geom;
pub mod shape;
pub mod ugraphic;

use font::StringBounder;
use geom::XDimension2D;
use ugraphic::UGraphic;

/// Something that knows its size and can draw itself.
pub trait TextBlock {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D;

    fn draw_u(&self, ug: &UGraphic);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HorizontalAlignment {
    #[default]
    Left,
    Center,
    Right,
}

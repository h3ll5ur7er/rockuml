//! A layer drawing tiles and connections back through itself (PlantUML's `UGraphicDispatchDrawable`), as
//! activity groups measure their inside on a `LimitFinder`.

use super::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};

pub(crate) struct UGraphicDispatchDrawable {
    ug: UGraphic,
}

impl UGraphicDispatchDrawable {
    pub(crate) fn create(ug: UGraphic) -> UGraphic {
        UGraphic::from_layer(Self { ug })
    }
}

impl UGraphicLayer for UGraphicDispatchDrawable {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    fn apply(&self, change: UChange) -> UGraphic {
        Self::create(self.ug.apply(change))
    }

    fn draw(&self, this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::Ftile(tile) => tile.draw_u(this),
            AnyShape::Connection(connection) => connection.draw_u(this),
            shape => self.ug.draw(shape),
        }
    }
}

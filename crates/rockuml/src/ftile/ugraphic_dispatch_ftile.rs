//! The layer that draws an activity diagram without swimlanes (PlantUML's `UGraphicDispatchFtile`): tiles
//! and connections drawn on it draw themselves back through it, everything else goes to the layer below. It
//! records where labels are drawn and draws the lines of the `goto`s after them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{Ftile, FtileGoto, FtileLabel, downcast};
use crate::color::HColor;
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};
use crate::svek::UGraphicForSnake;

pub(crate) struct UGraphicDispatchFtile {
    ug: UGraphic,
    /// Where each `label` tile was drawn, for the `goto`s drawn after it; shared by every copy.
    positions: Rc<RefCell<HashMap<String, UTranslate>>>,
    /// What `goto` lines are drawn in.
    goto_color: HColor,
}

impl UGraphicDispatchFtile {
    /// Over `ug`, which is a `UGraphicForSnake`: positions are measured from it.
    pub(crate) fn create(
        ug: UGraphic,
        positions: Rc<RefCell<HashMap<String, UTranslate>>>,
        goto_color: HColor,
    ) -> UGraphic {
        UGraphic::from_layer(Self {
            ug,
            positions,
            goto_color,
        })
    }

    /// How far the layer below has moved since the arrows' layer was set (`getPosition`), which is where a
    /// label tile is drawn.
    pub(crate) fn get_position(&self) -> UTranslate {
        self.ug
            .layer::<UGraphicForSnake>()
            .map_or_else(UTranslate::default, UGraphicForSnake::get_translation)
    }

    /// A line across, then up or down, from the `goto` to its label, if that was drawn before it.
    fn draw_goto(&self, ftile: &FtileGoto) {
        let geom = ftile.calculate_dimension(self.ug.string_bounder());
        let pt = geom.get_point_in();
        let ug_goto = self
            .ug
            .apply(self.goto_color.clone())
            .with_backcolor(self.goto_color.clone())
            .apply(UTranslate::new(pt.x, pt.y));
        let pos_now = self.get_position();
        let Some(dest) = self.positions.borrow().get(ftile.get_name()).copied() else {
            return;
        };
        let dx = dest.dx - pos_now.dx;
        let dy = dest.dy - pos_now.dy;
        ug_goto.draw(&UShape::Line { dx, dy: 0.0 });
        ug_goto
            .apply(UTranslate::new(dx, 0.0))
            .draw(&UShape::Line { dx: 0.0, dy });
    }
}

impl UGraphicLayer for UGraphicDispatchFtile {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    fn apply(&self, change: UChange) -> UGraphic {
        Self::create(
            self.ug.apply(change),
            self.positions.clone(),
            self.goto_color.clone(),
        )
    }

    fn draw(&self, this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::Ftile(tile) => {
                tile.draw_u(this);
                if let Some(label) = downcast::<FtileLabel>(tile) {
                    self.positions
                        .borrow_mut()
                        .insert(label.get_name().to_owned(), self.get_position());
                }
                if let Some(goto) = downcast::<FtileGoto>(tile) {
                    self.draw_goto(goto);
                }
            }
            AnyShape::Connection(connection) => connection.draw_u(this),
            shape => self.ug.draw(shape),
        }
    }
}

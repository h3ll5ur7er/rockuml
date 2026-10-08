//! The layer that draws an activity diagram without swimlanes (PlantUML's `UGraphicDispatchFtile`): tiles
//! and connections drawn on it draw themselves back through it, everything else goes to the layer below.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::color::HColor;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};
use crate::svek::UGraphicForSnake;

pub(crate) struct UGraphicDispatchFtile {
    ug: UGraphic,
    /// Where each `label` tile was drawn, for the `goto`s drawn after it; shared by every copy.
    positions: Rc<RefCell<HashMap<String, UTranslate>>>,
    /// What `goto` lines are drawn in.
    goto_color: HColor,
    /// PlantUML's debug option, which marks the ends of `goto` lines.
    is_debug: bool,
}

impl UGraphicDispatchFtile {
    /// Over `ug`, which is a `UGraphicForSnake`: positions are measured from it.
    pub(crate) fn create(
        ug: UGraphic,
        positions: Rc<RefCell<HashMap<String, UTranslate>>>,
        goto_color: HColor,
        is_debug: bool,
    ) -> UGraphic {
        UGraphic::from_layer(Self {
            ug,
            positions,
            goto_color,
            is_debug,
        })
    }

    /// How far the layer below has moved since the arrows' layer was set (`getPosition`), which is where a
    /// label tile is drawn.
    pub(crate) fn get_position(&self) -> UTranslate {
        self.ug
            .layer::<UGraphicForSnake>()
            .map_or_else(UTranslate::default, UGraphicForSnake::get_translation)
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
            self.is_debug,
        )
    }

    /// Labels and `goto` lines (`FtileLabel`, `FtileGoto`) join here when those tiles are ported.
    fn draw(&self, this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::Ftile(tile) => tile.draw_u(this),
            AnyShape::Connection(connection) => connection.draw_u(this),
            shape => self.ug.draw(shape),
        }
    }
}

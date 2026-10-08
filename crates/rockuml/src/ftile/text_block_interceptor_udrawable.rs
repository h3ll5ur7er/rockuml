//! Draws the tile tree of a diagram without swimlanes (PlantUML's `TextBlockInterceptorUDrawable`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{Ftile, UGraphicDispatchFtile};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct TextBlockInterceptorUDrawable {
    text_block: Rc<dyn Ftile>,
    goto_color: HColor,
    is_debug: bool,
}

impl TextBlockInterceptorUDrawable {
    pub(crate) fn new(text_block: Rc<dyn Ftile>, goto_color: HColor, is_debug: bool) -> Self {
        Self {
            text_block,
            goto_color,
            is_debug,
        }
    }
}

impl UDrawable for TextBlockInterceptorUDrawable {
    /// Draws the tree through a fresh `UGraphicDispatchFtile` over `ug`, which is a `UGraphicForSnake`,
    /// then flushes `ug` so the arrows are drawn.
    fn draw_u(&self, ug: &UGraphic) {
        UGraphicDispatchFtile::create(
            ug.clone(),
            Rc::new(RefCell::new(HashMap::new())),
            self.goto_color.clone(),
            self.is_debug,
        )
        .draw(&self.text_block);
        ug.flush_ug();
    }
}

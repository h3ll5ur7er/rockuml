//! The layer that holds the arrows of activity diagrams back until the end (PlantUML's
//! `svek.UGraphicForSnake`).
//!
//! An arrow drawn on it merges into the first held arrow it continues, or else is held after the others.
//! [`UGraphic::flush_ug`] drops the arrowhead of every arrow ending where a mergeable arrow starts, then
//! draws them all in that order, on the surface below as it was when each was drawn. Everything else
//! passes straight to the layer below, so arrows end up above the boxes.

use std::cell::RefCell;
use std::rc::Rc;

use crate::ftile::Snake;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};

pub(crate) struct UGraphicForSnake {
    ug: UGraphic,
    /// How far this copy has moved since the layer was set; arrows are compared there.
    dx: f64,
    dy: f64,
    snakes: Rc<RefCell<Vec<PendingSnake>>>,
}

struct PendingSnake {
    snake: Snake,
    ug: UGraphic,
    dx: f64,
    dy: f64,
}

impl PendingSnake {
    fn draw_internal(&self) {
        self.snake.draw_internal(&self.ug);
    }

    /// Whether this arrow ends where `other` starts, `other` letting it (`touchesOther`).
    fn touches_other(&self, other: &Self) -> bool {
        if other.snake.cannot_be_touched() {
            return false;
        }
        let this_last = self.snake.get_last();
        let other_first = other.snake.get_first();
        Snake::same(
            XPoint2D::new(this_last.x + self.dx, this_last.y + self.dy),
            XPoint2D::new(other_first.x + other.dx, other_first.y + other.dy),
        )
    }

    /// This arrow and `new_item` as one, kept where this one was drawn.
    fn merge(&self, new_item: &Self) -> Option<Self> {
        let s1 = self.snake.move_by(self.dx, self.dy);
        let s2 = new_item.snake.move_by(new_item.dx, new_item.dy);
        let merge = s1.merge(&s2, self.ug.string_bounder())?;
        Some(Self {
            snake: merge.move_by(-self.dx, -self.dy),
            ug: self.ug.clone(),
            dx: self.dx,
            dy: self.dy,
        })
    }
}

impl UGraphicForSnake {
    pub(crate) fn create(ug: UGraphic) -> UGraphic {
        UGraphic::from_layer(Self {
            ug,
            dx: 0.0,
            dy: 0.0,
            snakes: Rc::new(RefCell::new(Vec::new())),
        })
    }

    pub(crate) fn get_translation(&self) -> UTranslate {
        UTranslate::new(self.dx, self.dy)
    }

    fn add_pending_snake(&self, snake: &Snake) {
        let new_item = PendingSnake {
            snake: snake.clone(),
            ug: self.ug.clone(),
            dx: self.dx,
            dy: self.dy,
        };
        let mut snakes = self.snakes.borrow_mut();
        for pending in snakes.iter_mut() {
            if let Some(merge) = pending.merge(&new_item) {
                *pending = merge;
                return;
            }
        }
        snakes.push(new_item);
    }
}

impl UGraphicLayer for UGraphicForSnake {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    fn apply(&self, change: UChange) -> UGraphic {
        let (mut dx, mut dy) = (self.dx, self.dy);
        if let UChange::Translate(translate) = &change {
            dx += translate.dx;
            dy += translate.dy;
        }
        UGraphic::from_layer(Self {
            ug: self.ug.apply(change),
            dx,
            dy,
            snakes: self.snakes.clone(),
        })
    }

    fn draw(&self, _this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::Snake(snake) => self.add_pending_snake(snake),
            shape => self.ug.draw(shape),
        }
    }

    /// Draws the held arrows; the layer below is not flushed, as in PlantUML.
    fn flush_ug(&self) {
        let snakes = std::mem::take(&mut *self.snakes.borrow_mut());
        for pending in &snakes {
            if snakes.iter().any(|other| pending.touches_other(other)) {
                pending
                    .snake
                    .without_end_decoration()
                    .draw_internal(&pending.ug);
            } else {
                pending.draw_internal();
            }
        }
    }
}

#[cfg(test)]
mod tests;

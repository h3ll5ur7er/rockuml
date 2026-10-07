//! What is drawn on a lifeline: activation boxes, the destroy cross and the dotted line of delays.

use crate::color::HColor;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::component::{Area, Component};

/// An activation box; open ends are left without a horizontal line where a delay cuts the box.
pub(crate) struct ComponentRoseActiveLine {
    fashion: Fashion,
    close_up: bool,
    close_down: bool,
}

impl ComponentRoseActiveLine {
    pub(crate) const WIDTH: f64 = 10.0;

    pub(crate) fn new(fashion: Fashion, close_up: bool, close_down: bool) -> Self {
        Self {
            fashion,
            close_up,
            close_down,
        }
    }
}

impl Component for ComponentRoseActiveLine {
    fn preferred_width(&self, _string_bounder: &dyn StringBounder) -> f64 {
        Self::WIDTH
    }

    fn preferred_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dimension = area.dimension;
        let x = ((dimension.width - Self::WIDTH) / 2.0).trunc();
        if dimension.height == 0.0 {
            return;
        }
        ug.start_group(&UGroup::singleton(UGroupType::Title, ""));
        let rectangle = UShape::Rectangle(URectangle::new(Self::WIDTH, dimension.height));
        let ug = ug.with_color(self.fashion.fore_color.clone());
        let back = self.fashion.back_color.clone();
        if self.close_up && self.close_down {
            ug.with_backcolor(back).translated(x, 0.0).draw(&rectangle);
        } else {
            ug.with_backcolor(back.clone())
                .with_color(back)
                .translated(x, 0.0)
                .draw(&rectangle);
            let vertical = UShape::Line {
                dx: 0.0,
                dy: dimension.height,
            };
            ug.translated(x, 0.0).draw(&vertical);
            ug.translated(x + Self::WIDTH, 0.0).draw(&vertical);
            let horizontal = UShape::Line {
                dx: Self::WIDTH,
                dy: 0.0,
            };
            if self.close_up {
                ug.translated(x, 0.0).draw(&horizontal);
            }
            if self.close_down {
                ug.translated(x, dimension.height).draw(&horizontal);
            }
        }
        ug.close_group();
    }
}

/// The cross that ends a destroyed lifeline.
pub(crate) struct ComponentRoseDestroy {
    color: HColor,
    stroke: UStroke,
}

impl ComponentRoseDestroy {
    const CROSS_SIZE: f64 = 9.0;

    pub(crate) fn new(color: HColor, stroke: UStroke) -> Self {
        Self { color, stroke }
    }
}

impl Component for ComponentRoseDestroy {
    fn preferred_width(&self, _string_bounder: &dyn StringBounder) -> f64 {
        Self::CROSS_SIZE * 2.0
    }

    fn preferred_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        Self::CROSS_SIZE * 2.0
    }

    fn draw_internal(&self, ug: &UGraphic, _area: &Area) {
        let size = 2.0 * Self::CROSS_SIZE;
        let ug = ug.with_stroke(self.stroke).with_color(self.color.clone());
        ug.draw(&UShape::Line { dx: size, dy: size });
        ug.translated(0.0, size).draw(&UShape::Line {
            dx: size,
            dy: -size,
        });
    }
}

/// The lifeline during a delay.
pub(crate) struct ComponentRoseDelayLine {
    color: HColor,
    stroke: UStroke,
}

impl ComponentRoseDelayLine {
    pub(crate) fn new(color: HColor, stroke: UStroke) -> Self {
        Self { color, stroke }
    }
}

impl Component for ComponentRoseDelayLine {
    fn preferred_width(&self, _string_bounder: &dyn StringBounder) -> f64 {
        1.0
    }

    fn preferred_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        20.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let x = (area.dimension.width / 2.0).trunc();
        ug.with_stroke(self.stroke)
            .with_color(self.color.clone())
            .translated(x, 0.0)
            .draw(&UShape::Line {
                dx: 0.0,
                dy: area.dimension.height,
            });
    }
}

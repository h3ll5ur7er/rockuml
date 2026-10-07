//! What runs across the whole diagram: dividers (`== text ==`), delays (`...`) and page breaks
//! (PlantUML's `ComponentRoseDivider`, `ComponentRoseDelayText` and `ComponentRoseNewpage`).

use crate::color::HColor;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::component::{Area, Component, TextualPart};

/// A divider: a double line across the diagram, its text in a box in the middle.
pub(crate) struct ComponentRoseDivider {
    text: TextualPart,
    background: HColor,
    border: HColor,
    stroke: UStroke,
    round_corner: f64,
    empty: bool,
}

impl ComponentRoseDivider {
    pub(crate) fn new(
        text: TextualPart,
        background: HColor,
        border: HColor,
        stroke: UStroke,
        round_corner: f64,
        empty: bool,
    ) -> Self {
        Self {
            text,
            background,
            border,
            stroke,
            round_corner,
            empty,
        }
    }

    fn draw_separator(&self, ug: &UGraphic, width: f64) {
        let ug = ug.with_color(self.background.clone());
        ug.translated(0.0, -1.0)
            .with_stroke(UStroke::SIMPLE)
            .draw(&UShape::Rectangle(
                URectangle::new(width, 3.0).rounded(self.round_corner),
            ));
        let line = UShape::Line { dx: width, dy: 0.0 };
        let ug = ug
            .with_stroke(UStroke::with_thickness(self.stroke.thickness / 2.0))
            .with_color(self.border.clone());
        ug.translated(0.0, -1.0).draw(&line);
        ug.translated(0.0, 2.0).draw(&line);
    }
}

impl Component for ComponentRoseDivider {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width(string_bounder) + 30.0
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + 20.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        const DELTA_X: f64 = 6.0;
        let dimension = area.dimension;
        let ug = ug.with_backcolor(self.background.clone());
        self.draw_separator(&ug.translated(0.0, dimension.height / 2.0), dimension.width);
        if self.empty {
            return;
        }
        let string_bounder = ug.string_bounder();
        let text_width = self.text.text_width(string_bounder);
        let text_height = self.text.text_height(string_bounder);
        let x = (dimension.width - text_width - DELTA_X) / 2.0;
        let y = (dimension.height - text_height) / 2.0;
        let ug = ug.with_color(self.border.clone()).with_stroke(self.stroke);
        ug.translated(x, y).draw(&UShape::Rectangle(
            URectangle::new(text_width + DELTA_X, text_height).rounded(self.round_corner),
        ));
        self.text
            .text_block()
            .draw_u(&ug.translated(x + DELTA_X, y + self.text.padding().top));
    }
}

/// The text of a delay, centred.
pub(crate) struct ComponentRoseDelayText {
    text: TextualPart,
}

impl ComponentRoseDelayText {
    pub(crate) fn new(text: TextualPart) -> Self {
        Self { text }
    }
}

impl Component for ComponentRoseDelayText {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.pure_text_width(string_bounder)
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + 20.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let string_bounder = ug.string_bounder();
        let x = (area.dimension.width - self.text.text_width(string_bounder)) / 2.0;
        let y = (area.dimension.height - self.text.text_height(string_bounder)) / 2.0;
        self.text
            .text_block()
            .draw_u(&ug.translated(x, y + self.text.padding().top));
    }
}

/// The line where a page ends.
pub(crate) struct ComponentRoseNewpage {
    color: HColor,
    stroke: UStroke,
}

impl ComponentRoseNewpage {
    pub(crate) fn new(color: HColor, stroke: UStroke) -> Self {
        Self { color, stroke }
    }
}

impl Component for ComponentRoseNewpage {
    fn preferred_width(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }

    fn preferred_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        1.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        ug.with_stroke(self.stroke)
            .with_color(self.color.clone())
            .draw(&UShape::Line {
                dx: area.dimension.width,
                dy: 0.0,
            });
    }
}

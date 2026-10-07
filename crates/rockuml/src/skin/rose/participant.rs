use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::component::{Area, Component, TextualPart};

/// A participant's box, at the top or bottom of its lifeline; `collections` draws a second box behind it.
pub(crate) struct ComponentRoseParticipant {
    text: TextualPart,
    fashion: Fashion,
    min_width: f64,
    collections: bool,
    margin: ClockwiseTopRightBottomLeft,
}

impl ComponentRoseParticipant {
    pub(crate) fn new(
        text: TextualPart,
        fashion: Fashion,
        min_width: f64,
        collections: bool,
        margin: ClockwiseTopRightBottomLeft,
    ) -> Self {
        Self {
            text,
            fashion,
            min_width,
            collections,
            margin,
        }
    }

    fn delta_collection(&self) -> f64 {
        if self.collections { 4.0 } else { 0.0 }
    }

    fn text_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width_from(
            self.text
                .pure_text_width(string_bounder)
                .max(self.min_width),
        )
    }
}

impl Component for ComponentRoseParticipant {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_width(string_bounder)
            + self.margin.left
            + self.margin.right
            + self.delta_collection()
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder)
            + self.margin.top
            + self.margin.bottom
            + 1.0
            + self.delta_collection()
    }

    fn draw_internal(&self, ug: &UGraphic, _area: &Area) {
        let string_bounder = ug.string_bounder();
        let mut ug = self
            .fashion
            .apply_colors(&ug.translated(self.margin.left, self.margin.top))
            .with_stroke(self.fashion.stroke);
        let rectangle = UShape::Rectangle(
            URectangle::new(
                self.text_width(string_bounder),
                self.text.text_height(string_bounder),
            )
            .rounded(self.fashion.round_corner),
        );
        if self.collections {
            ug.translated(self.delta_collection(), 0.0).draw(&rectangle);
            ug = ug.translated(0.0, self.delta_collection());
        }
        ug.draw(&rectangle);
        let padding = self.text.padding();
        self.text.text_block().draw_u(
            &ug.with_stroke(UStroke::SIMPLE)
                .translated(padding.left, padding.top),
        );
    }
}

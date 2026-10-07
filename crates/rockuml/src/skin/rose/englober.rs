use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::component::{Area, Component, TextualPart};

/// A box around several participants, with its title centred at the top (PlantUML's
/// `ComponentRoseEnglober`). It is all background, so that participants are drawn over it.
pub(crate) struct ComponentRoseEnglober {
    text: TextualPart,
    fashion: Fashion,
    round_corner: f64,
}

impl ComponentRoseEnglober {
    pub(crate) fn new(text: TextualPart, fashion: Fashion, round_corner: f64) -> Self {
        Self {
            text,
            fashion,
            round_corner,
        }
    }
}

impl Component for ComponentRoseEnglober {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width(string_bounder)
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + 3.0
    }

    fn draw_internal(&self, _ug: &UGraphic, _area: &Area) {}

    fn draw_background_internal(&self, ug: &UGraphic, area: &Area) {
        let ug = self.fashion.apply(ug);
        let dimension = area.dimension;
        ug.draw(&UShape::Rectangle(
            URectangle::new(dimension.width, dimension.height).rounded(self.round_corner),
        ));
        let x = (dimension.width - self.text.pure_text_width(ug.string_bounder())) / 2.0;
        self.text.text_block().draw_u(&ug.translated(x, 0.0));
    }
}

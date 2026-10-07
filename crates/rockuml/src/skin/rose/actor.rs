use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::component::{Area, Component, TextualPart};

/// A participant drawn as a symbol with its name below it at the top of its lifeline, and above it at the
/// bottom. PlantUML's `ComponentRoseBoundary`, `ComponentRoseControl`, `ComponentRoseEntity` and
/// `ComponentRoseDatabase` are copies of this one with another symbol.
pub(crate) struct ComponentRoseActor {
    text: TextualPart,
    stickman: Box<dyn TextBlock>,
    head: bool,
}

impl ComponentRoseActor {
    pub(crate) fn new(text: TextualPart, stickman: Box<dyn TextBlock>, head: bool) -> Self {
        Self {
            text,
            stickman,
            head,
        }
    }

    fn text_middle_position(&self, string_bounder: &dyn StringBounder) -> f64 {
        (self.preferred_width(string_bounder) - self.text.text_width(string_bounder)) / 2.0
    }
}

impl Component for ComponentRoseActor {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.stickman
            .calculate_dimension(string_bounder)
            .width
            .max(self.text.text_width(string_bounder))
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.stickman.calculate_dimension(string_bounder).height
            + self.text.text_height(string_bounder)
    }

    fn draw_internal(&self, ug: &UGraphic, _area: &Area) {
        let string_bounder = ug.string_bounder();
        let stickman = self.stickman.calculate_dimension(string_bounder);
        let delta = (self.preferred_width(string_bounder) - stickman.width) / 2.0;
        let text_x = self.text_middle_position(string_bounder);
        let ug = if self.head {
            self.text
                .text_block()
                .draw_u(&ug.translated(text_x, stickman.height));
            ug.translated(delta, 0.0)
        } else {
            self.text.text_block().draw_u(&ug.translated(text_x, 0.0));
            ug.translated(delta, self.text.text_height(string_bounder))
        };
        self.stickman.draw_u(&ug);
    }
}

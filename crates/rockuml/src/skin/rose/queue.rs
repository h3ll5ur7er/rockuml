use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::component::{Area, Component};

/// A queue: its symbol holds the name.
pub(crate) struct ComponentRoseQueue {
    symbol: Box<dyn TextBlock>,
}

impl ComponentRoseQueue {
    pub(crate) fn new(symbol: Box<dyn TextBlock>) -> Self {
        Self { symbol }
    }
}

impl Component for ComponentRoseQueue {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.symbol.calculate_dimension(string_bounder).width
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.symbol.calculate_dimension(string_bounder).height
    }

    fn draw_internal(&self, ug: &UGraphic, _area: &Area) {
        self.symbol.draw_u(ug);
    }
}

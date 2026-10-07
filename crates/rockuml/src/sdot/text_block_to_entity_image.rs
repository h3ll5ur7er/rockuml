//! The drawing of a group's own layout, used as the image of that group (PlantUML's
//! `TextBlockToEntityImage`).

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::svek::{IEntityImage, ShapeType};

pub(crate) struct TextBlockToEntityImage {
    text_block: Box<dyn TextBlock>,
}

impl TextBlockToEntityImage {
    pub(crate) fn new(text_block: Box<dyn TextBlock>) -> Self {
        Self { text_block }
    }
}

impl TextBlock for TextBlockToEntityImage {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.text_block.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.text_block.draw_u(ug);
    }

    fn backcolor(&self) -> Option<HColor> {
        self.text_block.backcolor()
    }
}

impl IEntityImage for TextBlockToEntityImage {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn is_hidden(&self) -> bool {
        false
    }
}

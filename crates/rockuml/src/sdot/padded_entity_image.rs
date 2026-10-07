//! An image with room around it (PlantUML's `PaddedEntityImage`), as concurrent regions get before they are
//! stacked.

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::svek::IEntityImage;

pub(crate) struct PaddedEntityImage {
    image: Box<dyn IEntityImage>,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl PaddedEntityImage {
    pub(crate) fn new(
        image: Box<dyn IEntityImage>,
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
    ) -> Self {
        Self {
            image,
            left,
            top,
            right,
            bottom,
        }
    }

    pub(crate) fn uniform(image: Box<dyn IEntityImage>, padding: f64) -> Self {
        Self::new(image, padding, padding, padding, padding)
    }
}

impl TextBlock for PaddedEntityImage {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim = self.image.calculate_dimension(string_bounder);
        XDimension2D::new(
            dim.width + self.left + self.right,
            dim.height + self.top + self.bottom,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.image.draw_u(&ug.translated(self.left, self.top));
    }

    fn backcolor(&self) -> Option<HColor> {
        self.image.backcolor()
    }
}

impl IEntityImage for PaddedEntityImage {}

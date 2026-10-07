use super::Sprite;
use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::image::PortableImage;
use crate::klimt::shape::{UImage, UShape};
use crate::klimt::ugraphic::UGraphic;

/// A sprite from a raster image, such as a PNG.
pub(crate) struct SpriteImage {
    img: UImage,
}

impl SpriteImage {
    pub(crate) fn new(image: PortableImage) -> Self {
        Self {
            img: UImage::new(image),
        }
    }
}

impl Sprite for SpriteImage {
    fn as_text_block(
        &self,
        font_color: &HColor,
        forced_color: Option<&HColor>,
        scale: f64,
    ) -> Box<dyn TextBlock + '_> {
        Box::new(ImageBlock {
            img: &self.img,
            used_color: forced_color.unwrap_or(font_color).clone(),
            scale,
        })
    }
}

struct ImageBlock<'a> {
    img: &'a UImage,
    used_color: HColor,
    scale: f64,
}

impl TextBlock for ImageBlock<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.img.width() * self.scale,
            self.img.height() * self.scale,
        )
    }

    /// The image's darkest colour takes the text colour.
    fn draw_u(&self, ug: &UGraphic) {
        let image = self
            .img
            .mute_color(self.used_color.as_xcolor())
            .scale(self.scale);
        ug.draw(&UShape::Image(image));
    }
}

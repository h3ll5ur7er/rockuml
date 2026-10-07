use super::Sprite;
use crate::color::{HColor, XColor};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::image::PortableImage;
use crate::klimt::shape::{UImage, UShape};
use crate::klimt::ugraphic::UGraphic;

/// A sprite of opaque colours, drawn as written whatever the text colour.
pub(crate) struct SpriteColor {
    image: PortableImage,
}

impl Sprite for SpriteColor {
    fn as_text_block(
        &self,
        _font_color: &HColor,
        _forced_color: Option<&HColor>,
        scale: f64,
    ) -> Box<dyn TextBlock + '_> {
        Box::new(ColorBlock {
            image: &self.image,
            scale,
        })
    }
}

struct ColorBlock<'a> {
    image: &'a PortableImage,
    scale: f64,
}

impl TextBlock for ColorBlock<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.image.width() as f64 * self.scale,
            self.image.height() as f64 * self.scale,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let image = UImage::new(self.image.clone()).scale(self.scale);
        ug.draw(&UShape::Image(image));
    }
}

/// `[WxH/color]` sprites: two characters of [`ColorPalette4096`] per pixel.
pub(crate) struct SpriteColorBuilder4096;

impl SpriteColorBuilder4096 {
    /// The first line gives the width; pixels a line leaves out are black.
    pub(crate) fn build_sprite(strings: &[String]) -> SpriteColor {
        let width = strings.first().map_or(0, |first| first.chars().count() / 2);
        let mut image = PortableImage::new(width, strings.len());
        for (line, text) in strings.iter().enumerate() {
            let chars: Vec<char> = text.chars().collect();
            let mut pairs = chars.as_chunks::<2>().0.iter();
            for col in 0..width {
                let color = pairs
                    .next()
                    .and_then(|&[c1, c2]| ColorPalette4096::get_color_for(c1, c2))
                    .unwrap_or(XColor::rgb(0, 0, 0));
                image.set_rgb(col, line, color.argb());
            }
        }
        SpriteColor { image }
    }
}

/// 4096 colours, 16 levels per channel, each named by two characters.
struct ColorPalette4096;

impl ColorPalette4096 {
    const COLOR_VALUE: &str = ".,$%&*+-:;<=>?@^_~GHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    fn get_color_for(c1: char, c2: char) -> Option<XColor> {
        let value = |c: char| {
            let c = match c {
                '!' => '.',
                '#' => ',',
                other => other,
            };
            Self::COLOR_VALUE.find(c)
        };
        let code = value(c1)? * 64 + value(c2)?;
        let dup = |level: usize| (level * 16 + level) as u8;
        Some(XColor::rgb(
            dup(code / 256 % 16),
            dup(code / 16 % 16),
            dup(code % 16),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_characters_name_each_colour() {
        let sprite = SpriteColorBuilder4096::build_sprite(&["..zz!zW.".to_owned()]);
        let pixels: Vec<u32> = (0..4).map(|x| sprite.image.get_rgb(x, 0)).collect();
        // `z` is 63 and `W` 34: `zz` is code 4095, `.z` 63 and `W.` 2176.
        assert_eq!(pixels, [0xff000000, 0xffffffff, 0xff0033ff, 0xff888800]);
    }
}

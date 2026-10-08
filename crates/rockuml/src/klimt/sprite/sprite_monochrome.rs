use super::Sprite;
use crate::color::{ColorMapper, HColor, XColor};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::image::PortableImage;
use crate::klimt::shape::{UImage, UShape};
use crate::klimt::ugraphic::UGraphic;

/// A sprite of gray levels, drawn as shades between the background and the text colour.
pub(crate) struct SpriteMonochrome {
    width: usize,
    height: usize,
    gray_level: usize,
    /// Row by row.
    gray: Vec<usize>,
}

impl SpriteMonochrome {
    pub(crate) fn new(width: usize, height: usize, gray_level: usize) -> Self {
        Self {
            width,
            height,
            gray_level,
            gray: vec![0; width * height],
        }
    }

    /// Levels outside the sprite are ignored.
    pub(crate) fn set_gray(&mut self, x: usize, y: usize, level: usize) {
        if x < self.width && y < self.height {
            self.gray[y * self.width + x] = level;
        }
    }

    #[cfg(test)]
    pub(crate) fn get_gray(&self, x: usize, y: usize) -> usize {
        self.gray[y * self.width + x]
    }

    #[cfg(test)]
    pub(super) fn width(&self) -> usize {
        self.width
    }

    #[cfg(test)]
    pub(super) fn height(&self) -> usize {
        self.height
    }

    /// Faint levels fade out: a level below a quarter of the darkest is drawn translucent.
    fn to_uimage(&self, mapper: ColorMapper, backcolor: &HColor, color: &HColor) -> UImage {
        let back = if backcolor.is_transparent() {
            HColor::WHITE
        } else {
            backcolor.clone()
        };
        let color = if color.is_transparent() {
            HColor::BLACK
        } else {
            color.clone()
        };
        let coef = |gray: usize| gray as f64 / (self.gray_level - 1) as f64;
        let max_coef = self.gray.iter().map(|&gray| coef(gray)).fold(0.0, f64::max);
        let mut image = PortableImage::new(self.width, self.height);
        for col in 0..self.width {
            for line in 0..self.height {
                let coef = coef(self.gray[line * self.width + col]);
                let alpha = if coef > max_coef / 4.0 {
                    255
                } else {
                    (255.0 * (coef * 4.0 / max_coef)) as i32
                };
                let pixel =
                    gradient_color(back.to_color(mapper), color.to_color(mapper), coef, alpha);
                image.set_rgb(col, line, pixel);
            }
        }
        UImage::new(image)
    }
}

/// `HColorGradient.getColor`: each channel `coef` of the way from `from` to `to`, rounded towards `from`.
fn gradient_color(from: XColor, to: XColor, coef: f64, alpha: i32) -> u32 {
    let channel = |from: u8, to: u8| {
        let from = i32::from(from);
        from + (coef * f64::from(i32::from(to) - from)) as i32
    };
    let [red, green, blue] = [
        channel(from.red, to.red),
        channel(from.green, to.green),
        channel(from.blue, to.blue),
    ];
    u32::from_be_bytes([alpha as u8, red as u8, green as u8, blue as u8])
}

impl Sprite for SpriteMonochrome {
    fn as_text_block(
        &self,
        font_color: &HColor,
        forced_color: Option<&HColor>,
        scale: f64,
        _back_color: Option<&HColor>,
    ) -> Box<dyn TextBlock + '_> {
        Box::new(MonochromeBlock {
            sprite: self,
            color: forced_color.unwrap_or(font_color).clone(),
            scale,
        })
    }
}

struct MonochromeBlock<'a> {
    sprite: &'a SpriteMonochrome,
    color: HColor,
    scale: f64,
}

impl TextBlock for MonochromeBlock<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.sprite.width as f64 * self.scale,
            self.sprite.height as f64 * self.scale,
        )
    }

    /// Shaded towards the colour the surface fills with.
    fn draw_u(&self, ug: &UGraphic) {
        let image = self
            .sprite
            .to_uimage(ug.color_mapper(), &ug.param().backcolor, &self.color)
            .scale(self.scale);
        ug.draw(&UShape::Image(image));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_shade_from_the_background_to_the_colour() {
        let mut sprite = SpriteMonochrome::new(4, 1, 16);
        sprite.set_gray(1, 0, 3);
        sprite.set_gray(2, 0, 4);
        sprite.set_gray(3, 0, 15);
        let image = sprite.to_uimage(ColorMapper::Identity, &HColor::NONE, &HColor::BLUE);
        let pixels: Vec<u32> = (0..4)
            .map(|x| image.image().unwrap().get_rgb(x, 0))
            .collect();
        assert_eq!(pixels, [0x00ffffff, 0xccccccff, 0xffbbbbff, 0xff0000ff]);
    }
}

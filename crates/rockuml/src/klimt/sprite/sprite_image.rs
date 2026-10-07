use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;
use std::sync::LazyLock;

use super::Sprite;
use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::image::PortableImage;
use crate::klimt::shape::{UImage, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::svg_parser::SvgNanoParser;

/// The bundle `tools/bundle-sprites.sh` builds: per file, its path, a newline, its length, a newline, its bytes.
static INTERNAL_SPRITES: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let compressed = crate::assets::get("sprites/sprites.br").expect("the sprites are bundled");
    let mut bundle = Vec::new();
    brotli_decompressor::Decompressor::new(compressed, 4096)
        .read_to_end(&mut bundle)
        .expect("the bundle is valid Brotli");
    bundle
});

/// The built-in sprites by path, such as `archimate/actor.svg`.
static INTERNAL_SPRITE_FILES: LazyLock<HashMap<&'static str, &'static [u8]>> =
    LazyLock::new(|| {
        let mut files = HashMap::new();
        let mut rest = INTERNAL_SPRITES.as_slice();
        while !rest.is_empty() {
            let mut line = || {
                let end = rest
                    .iter()
                    .position(|&byte| byte == b'\n')
                    .expect("a header line");
                let line = std::str::from_utf8(&rest[..end]).expect("a UTF-8 header");
                rest = &rest[end + 1..];
                line
            };
            let path = line();
            let length: usize = line().parse().expect("a length");
            let (file, tail) = rest.split_at(length);
            files.insert(path, file);
            rest = tail;
        }
        files
    });

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

    /// One of PlantUML's built-in sprites, such as `archimate/actor`: an SVG one if there is, else a PNG.
    pub(crate) fn from_internal(name: &str) -> Option<Rc<dyn Sprite>> {
        // Per thread, as sprites are `Rc`s.
        thread_local! {
            static LOADED: RefCell<HashMap<String, Option<Rc<dyn Sprite>>>> = RefCell::default();
        }
        if let Some(loaded) = LOADED.with_borrow(|loaded| loaded.get(name).cloned()) {
            return loaded;
        }
        let sprite = Self::load_internal(name);
        LOADED.with_borrow_mut(|loaded| loaded.insert(name.to_owned(), sprite.clone()));
        sprite
    }

    fn load_internal(name: &str) -> Option<Rc<dyn Sprite>> {
        if let Some(svg) = INTERNAL_SPRITE_FILES.get(format!("{name}.svg").as_str()) {
            let svg = std::str::from_utf8(svg).expect("the bundled SVG sprites are UTF-8");
            return Some(Rc::new(SvgNanoParser::new(svg)));
        }
        let png = INTERNAL_SPRITE_FILES.get(format!("{name}.png").as_str())?;
        let image = PortableImage::from_png(png).expect("the bundled PNG sprites decode");
        Some(Rc::new(SpriteImage::new(image)))
    }
}

impl Sprite for SpriteImage {
    fn as_text_block(
        &self,
        font_color: &HColor,
        forced_color: Option<&HColor>,
        scale: f64,
        _back_color: Option<&HColor>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::debug::StringBounderDebug;

    fn dimension(name: &str) -> Option<XDimension2D> {
        SpriteImage::from_internal(name).map(|sprite| {
            sprite
                .as_text_block(&HColor::BLACK, None, 1.0, None)
                .calculate_dimension(&StringBounderDebug)
        })
    }

    #[test]
    fn built_in_sprites_are_svg_first_then_png() {
        assert_eq!(INTERNAL_SPRITE_FILES.len(), 139);
        assert_eq!(
            dimension("archimate/actor"),
            Some(XDimension2D::new(20.0, 20.0))
        );
        assert_eq!(
            dimension("archimate/access"),
            Some(XDimension2D::new(15.0, 15.0)),
            "PlantUML measures the 16-pixel image one pixel short"
        );
        assert!(dimension("archimate/no-such-sprite").is_none());
        assert!(dimension("archimate/actor.svg").is_none());
    }
}

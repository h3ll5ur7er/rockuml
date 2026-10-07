//! Small images a diagram defines with `sprite` and draws with `<$name>` (PlantUML's `klimt.sprite`).

mod sprite_color;
mod sprite_gray_level;
mod sprite_image;
mod sprite_monochrome;

pub(crate) use sprite_color::SpriteColorBuilder4096;
pub(crate) use sprite_gray_level::SpriteGrayLevel;
pub(crate) use sprite_image::SpriteImage;
pub(crate) use sprite_monochrome::SpriteMonochrome;

use std::rc::Rc;

use super::TextBlock;
use crate::color::HColor;

pub(crate) trait Sprite {
    /// The sprite drawn in `forced_color`, or else `font_color`, at `scale`.
    fn as_text_block(
        &self,
        font_color: &HColor,
        forced_color: Option<&HColor>,
        scale: f64,
    ) -> Box<dyn TextBlock + '_>;
}

/// Where creole text finds the sprites it names.
pub(crate) trait SpriteContainer {
    fn get_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>>;
}

/// For text outside any diagram that defines sprites, which still finds the built-in ones.
pub(crate) struct SpriteContainerEmpty;

impl SpriteContainer for SpriteContainerEmpty {
    fn get_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>> {
        SpriteImage::from_internal(name)
    }
}

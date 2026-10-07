//! Small images a diagram defines with `sprite` and draws with `<$name>` (PlantUML's `klimt.sprite`).

mod sprite_color;
mod sprite_gray_level;
mod sprite_image;
mod sprite_monochrome;

pub(crate) use sprite_color::SpriteColorBuilder4096;
pub(crate) use sprite_gray_level::SpriteGrayLevel;
pub(crate) use sprite_image::SpriteImage;

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

/// Where creole text finds the sprites and images it names.
pub(crate) trait SpriteContainer {
    fn get_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>>;

    /// The base64 data of a PNG the source refers to by MD5.
    fn get_from_md5(&self, md5: &str) -> Option<&str>;

    /// The content of a file or URL an `<img>` names, read when the diagram was created.
    fn image_file(&self, src: &str) -> Option<&[u8]>;
}

/// For text outside any diagram that defines sprites.
pub(crate) struct SpriteContainerEmpty;

impl SpriteContainer for SpriteContainerEmpty {
    fn get_sprite(&self, _name: &str) -> Option<Rc<dyn Sprite>> {
        None
    }

    fn get_from_md5(&self, _md5: &str) -> Option<&str> {
        None
    }

    fn image_file(&self, _src: &str) -> Option<&[u8]> {
        None
    }
}

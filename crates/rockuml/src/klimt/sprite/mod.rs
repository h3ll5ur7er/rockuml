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
use super::geom::ClockwiseTopRightBottomLeft;
use crate::color::HColor;

/// A `sprite` definition drawing the PNG, GIF or JPEG `image` in 4, 8 or 16 grays (`SpriteUtils.encode`);
/// `None` when the image cannot be read or the number of grays is another.
pub fn encode(image: &[u8], name: &str, grays: u32) -> Option<String> {
    let level = SpriteGrayLevel::get(grays)?;
    let image = super::image::PortableImage::read(image)?;
    let mut result = format!(
        "sprite ${name} [{}x{}/{grays}] {{\n",
        image.width(),
        image.height()
    );
    for line in level.encode(&image) {
        result.push_str(&line);
        result.push('\n');
    }
    result.push_str("}\n");
    Some(result)
}

pub(crate) trait Sprite {
    /// The sprite drawn in `forced_color`, or else `font_color`, at `scale`. Only SVG sprites paint the
    /// text's `back_color` behind them.
    fn as_text_block(
        &self,
        font_color: &HColor,
        forced_color: Option<&HColor>,
        scale: f64,
        back_color: Option<&HColor>,
    ) -> Box<dyn TextBlock + '_>;
}

/// Where creole text finds the sprites and images it names.
pub(crate) trait SpriteContainer {
    fn get_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>>;

    /// The base64 data of a PNG the source refers to by MD5.
    fn get_from_md5(&self, md5: &str) -> Option<&str>;

    /// The content of a file or URL an `<img>` names, read when the diagram was created.
    fn image_file(&self, src: &str) -> Option<&[u8]>;

    /// The room around creole text laid out with this container (`ISkinSimple.getPadding`).
    fn get_padding(&self) -> ClockwiseTopRightBottomLeft {
        ClockwiseTopRightBottomLeft::none()
    }
}

/// For text outside any diagram that defines sprites, which still finds the built-in ones.
pub(crate) struct SpriteContainerEmpty;

impl SpriteContainer for SpriteContainerEmpty {
    fn get_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>> {
        SpriteImage::from_internal(name)
    }

    fn get_from_md5(&self, _md5: &str) -> Option<&str> {
        None
    }

    fn image_file(&self, _src: &str) -> Option<&[u8]> {
        None
    }
}

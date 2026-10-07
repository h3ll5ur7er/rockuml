//! Images in creole text: `<img:...>` (PlantUML's `AtomImg` and `AtomImgSvg`).

use std::path::Path;
use std::rc::Rc;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;

use super::Atom;
use super::atom_text::AtomText;
use crate::diagram::{BASE64_TAG_REPLACEMENT, BASE64_TAG_START};
use crate::klimt::TextBlock;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::XDimension2D;
use crate::klimt::image::PortableImage;
use crate::klimt::shape::{UImage, UImageSvg, UShape};
use crate::klimt::sprite::SpriteContainer;
use crate::klimt::ugraphic::UGraphic;

const DATA_IMAGE_SVG_BASE64: &str = "data:image/svg+xml;base64,";

/// A raster image.
pub(super) struct AtomImg {
    /// The pixels drawn, scaled.
    image: UImage,
    /// The unrounded scaled size, which the scaled pixels round.
    dimension: XDimension2D,
}

impl AtomImg {
    /// The image `src` names (a PNG by MD5, an SVG data URI, a URL or a file), or a text saying why there
    /// is none. Files and URLs come from what the diagram read when it was created.
    pub(super) fn create(sprites: &dyn SpriteContainer, src: &str, scale: f64) -> Rc<dyn Atom> {
        if let Some(md5) = src.strip_prefix(BASE64_TAG_REPLACEMENT) {
            let Some(base64) = sprites.get_from_md5(md5) else {
                return text(&format!("[md5:{md5}]"));
            };
            return PortableImage::read_base64(base64).map_or_else(
                || text(&format!("(Cannot decode: {BASE64_TAG_START}{base64})")),
                |image| raster(image, scale),
            );
        }
        if let Some(base64) = src.strip_prefix(DATA_IMAGE_SVG_BASE64) {
            return match BASE64_STANDARD.decode(base64) {
                Ok(svg) => svg_image(String::from_utf8_lossy(&svg).into_owned(), scale),
                Err(_) => cannot_decode(),
            };
        }
        let content = sprites.image_file(src);
        if src.starts_with("http:") || src.starts_with("https:") {
            if is_svg(src) {
                return content.map_or_else(
                    || text(&format!("(Cannot decode SVG: {src})")),
                    |content| svg_image(String::from_utf8_lossy(content).into_owned(), scale),
                );
            }
            return content.and_then(read_raster_image).map_or_else(
                || text(&format!("(Cannot decode: {src})")),
                |image| raster(image, scale),
            );
        }
        let Some(content) = content else {
            return cannot_decode();
        };
        if is_svg(src) {
            // PlantUML reads SVG files line by line and joins the lines without separators.
            let svg = String::from_utf8_lossy(content).replace(['\r', '\n'], "");
            return svg_image(svg, scale);
        }
        read_raster_image(content).map_or_else(cannot_decode, |image| raster(image, scale))
    }
}

impl TextBlock for AtomImg {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.dimension
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&UShape::Image(self.image.clone()));
    }
}

impl Atom for AtomImg {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}

/// An SVG image, which only SVG output draws.
struct AtomImgSvg {
    image: UImageSvg,
}

impl TextBlock for AtomImgSvg {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(self.image.width(), self.image.height())
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&UShape::ImageSvg(self.image.clone()));
    }
}

impl Atom for AtomImgSvg {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}

fn raster(image: PortableImage, scale: f64) -> Rc<dyn Atom> {
    let dimension = XDimension2D::new(image.width() as f64 * scale, image.height() as f64 * scale);
    Rc::new(AtomImg {
        image: UImage::new(image).scale(scale),
        dimension,
    })
}

fn svg_image(svg: String, scale: f64) -> Rc<dyn Atom> {
    Rc::new(AtomImgSvg {
        image: UImageSvg::new(svg, scale),
    })
}

/// PlantUML tells SVGs from raster images by their name.
fn is_svg(src: &str) -> bool {
    Path::new(src)
        .extension()
        .is_some_and(|extension| extension == "svg")
}

/// Files and URLs, unlike data URIs, go through an AWT image first (`SecurityUtils.readRasterImage`).
fn read_raster_image(content: &[u8]) -> Option<PortableImage> {
    PortableImage::read(content).map(|image| image.drawn_on_transparent())
}

/// PlantUML names the missing file only under its INSECURE security profile.
fn cannot_decode() -> Rc<dyn Atom> {
    text("(Cannot decode)")
}

fn text(message: &str) -> Rc<dyn Atom> {
    Rc::new(AtomText::legacy(
        message,
        FontConfiguration::black_blue_true(UFont::monospace(14)),
    ))
}

use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::font::{FontConfiguration, UFont};
use super::geom::XDimension2D;
use super::image::PortableImage;
use crate::color::XColor;

#[derive(Clone, Debug, PartialEq)]
pub enum UShape {
    Text(UText),
    Ellipse(UEllipse),
    Rectangle(URectangle),
    /// A straight line from the current position by this offset.
    Line {
        dx: f64,
        dy: f64,
    },
    /// A closed shape through these points, relative to the current position.
    Polygon(Vec<(f64, f64)>),
    /// Segments relative to the current position. Closing a path adds nothing, as in PlantUML.
    Path(Vec<USegment>),
    Image(UImage),
    ImageSvg(UImageSvg),
    /// A letter centred on the current position, as in a stereotype's spot.
    CenteredCharacter(UCenteredCharacter),
    /// Takes up space without drawing anything.
    Empty(XDimension2D),
    /// A separator across whatever contains it; only containers that know their width can draw it.
    HorizontalLine,
    /// A text block that formats drawing it themselves never pass on, and measuring surfaces skip.
    SpecialText,
}

impl UShape {
    /// The name of the Java class PlantUML draws this shape with.
    pub fn java_class_name(&self) -> &'static str {
        match self {
            Self::Text(_) => "UText",
            Self::Ellipse(_) => "UEllipse",
            Self::Rectangle(_) => "URectangle",
            Self::Line { .. } => "ULine",
            Self::Polygon(_) => "UPolygon",
            Self::Path(_) => "UPath",
            Self::Empty(_) => "UEmpty",
            Self::Image(_) => "UImage",
            Self::ImageSvg(_) => "UImageSvg",
            Self::CenteredCharacter(_) => "UCenteredCharacter",
            Self::HorizontalLine => "UHorizontalLine",
            Self::SpecialText => "SpecialText",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(
    clippy::enum_variant_names,
    reason = "PlantUML's names: SEG_MOVETO, SEG_LINETO, SEG_CUBICTO, SEG_ARCTO"
)]
pub enum USegment {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    CubicTo {
        ctrl1: (f64, f64),
        ctrl2: (f64, f64),
        end: (f64, f64),
    },
    ArcTo {
        radius: (f64, f64),
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        end: (f64, f64),
    },
}

impl USegment {
    /// An arc's radii and rotation do not move with it.
    #[must_use]
    pub fn translate(self, dx: f64, dy: f64) -> Self {
        let moved = |(x, y): (f64, f64)| (x + dx, y + dy);
        match self {
            Self::MoveTo(x, y) => Self::MoveTo(x + dx, y + dy),
            Self::LineTo(x, y) => Self::LineTo(x + dx, y + dy),
            Self::CubicTo { ctrl1, ctrl2, end } => Self::CubicTo {
                ctrl1: moved(ctrl1),
                ctrl2: moved(ctrl2),
                end: moved(end),
            },
            Self::ArcTo {
                radius,
                x_axis_rotation,
                large_arc,
                sweep,
                end,
            } => Self::ArcTo {
                radius,
                x_axis_rotation,
                large_arc,
                sweep,
                end: moved(end),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UText {
    pub text: String,
    pub font: FontConfiguration,
}

impl UText {
    pub fn new(text: &str, font: FontConfiguration) -> Self {
        Self {
            text: crate::jaws::make_newlines_visible(text),
            font,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UCenteredCharacter {
    pub character: char,
    pub font: UFont,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UEllipse {
    pub width: f64,
    pub height: f64,
}

impl UEllipse {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// A bitmap: the image as given and the scale it is drawn at (PlantUML's `UImage` over a `PixelImage`).
#[derive(Clone, Debug)]
pub struct UImage {
    image_scale1: Rc<PortableImage>,
    scale: f64,
    /// Resampled once, when first drawn, as `PixelImage` caches it.
    scaled: OnceCell<Option<Rc<PortableImage>>>,
}

impl UImage {
    pub(crate) fn new(image: PortableImage) -> Self {
        Self::with_scale(Rc::new(image), 1.0)
    }

    fn with_scale(image_scale1: Rc<PortableImage>, scale: f64) -> Self {
        Self {
            image_scale1,
            scale,
            scaled: OnceCell::new(),
        }
    }

    /// Scaled from the image as given, so that scaling twice resamples once.
    #[must_use]
    pub(crate) fn scale(&self, scale: f64) -> Self {
        Self::with_scale(self.image_scale1.clone(), self.scale * scale)
    }

    /// The image with its darkest opaque colour replaced by `new_color` (`PixelImage.muteColor`).
    #[must_use]
    pub(crate) fn mute_color(&self, new_color: XColor) -> Self {
        let original = &self.image_scale1;
        let opaque = (0..original.width())
            .flat_map(|x| (0..original.height()).map(move |y| original.get_rgb(x, y)))
            .filter(|argb| argb >> 24 == 0xFF);
        let mut darker_rgb = None;
        for argb in opaque {
            let rgb = argb & 0x00FF_FFFF;
            if darker_rgb.is_none_or(|darker| gray_scale(rgb) < gray_scale(darker)) {
                darker_rgb = Some(rgb);
            }
        }
        let mut copy = PortableImage::clone(original);
        for x in 0..original.width() {
            for y in 0..original.height() {
                let argb = original.get_rgb(x, y);
                let alpha = argb & 0xFF00_0000;
                if alpha != 0 && Some(argb & 0x00FF_FFFF) == darker_rgb {
                    // Java adds the alpha to an opaque colour, which carries into the alpha byte.
                    copy.set_rgb(x, y, new_color.argb().wrapping_add(alpha));
                }
            }
        }
        Self::with_scale(Rc::new(copy), self.scale)
    }

    /// `None` where Java fails to draw the image: when it has no pixels or cannot be scaled.
    pub(crate) fn image(&self) -> Option<&PortableImage> {
        let image = if self.scale == 1.0 {
            Some(self.image_scale1.as_ref())
        } else {
            self.scaled
                .get_or_init(|| self.image_scale1.scale(self.scale).map(Rc::new))
                .as_deref()
        };
        image.filter(|image| image.width() > 0 && image.height() > 0)
    }

    /// One less than the pixels drawn across, as PlantUML measures images.
    pub(crate) fn width(&self) -> f64 {
        self.image_scale1.scaled_size(self.scale).0 as f64 - 1.0
    }

    pub(crate) fn height(&self) -> f64 {
        self.image_scale1.scaled_size(self.scale).1 as f64 - 1.0
    }
}

/// The same image at the same scale, whether or not it was resampled yet.
impl PartialEq for UImage {
    fn eq(&self, other: &Self) -> bool {
        self.image_scale1 == other.image_scale1 && self.scale == other.scale
    }
}

/// An SVG document drawn as an image at a scale (PlantUML's `UImageSvg`). Only SVG output draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct UImageSvg {
    svg: String,
    scale: f64,
    /// The size the document declares, before scaling.
    data_width: u32,
    data_height: u32,
}

impl UImageSvg {
    pub(crate) fn new(svg: String, scale: f64) -> Self {
        Self {
            data_width: declared_size(&svg, "width"),
            data_height: declared_size(&svg, "height"),
            svg,
            scale,
        }
    }

    pub(crate) fn contains_xlink(&self) -> bool {
        self.svg
            .contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\"")
    }

    /// The document starting with a bare `<svg>`, ready for another root element: without its XML
    /// declaration and root attributes, a background its root's style gives painted by a rectangle
    /// (`getSvg(false)`). `None` for a document whose root is not `<svg>`, on which PlantUML fails.
    pub(crate) fn svg(&self) -> Option<String> {
        static STYLE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"(?i)<svg[^>]+style="([^">]+)""#).unwrap());
        static BACKGROUND: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"background:([^;]+)").unwrap());

        let mut result = self.svg.as_str();
        if result.starts_with("<?xml")
            && let Some(start) = result.find("<svg")
        {
            result = &result[start..];
        }
        if !result.starts_with("<svg") {
            return None;
        }
        let end = result.find('>')?;
        let mut result = format!("<svg>{}", &result[end + 1..]);
        if let Some(style) = STYLE
            .captures(&self.svg)
            .map(|captures| captures[1].to_owned())
            && let Some(background) = BACKGROUND.captures(&style)
        {
            let rect = format!(
                "<g><rect fill=\"{}\" style=\"{style}\" width=\"{}\" height=\"{}\"/> ",
                &background[1], self.data_width, self.data_height
            );
            result = result.replacen("<g>", &rect, 1);
        }
        Some(result)
    }

    pub(crate) fn data_width(&self) -> u32 {
        self.data_width
    }

    pub(crate) fn data_height(&self) -> u32 {
        self.data_height
    }

    pub(crate) fn width(&self) -> f64 {
        f64::from(self.data_width) * self.scale
    }

    pub(crate) fn height(&self) -> f64 {
        f64::from(self.data_height) * self.scale
    }

    pub(crate) fn scale(&self) -> f64 {
        self.scale
    }
}

/// The viewBox's size rounded up, else the root's `width` or `height` attribute. A document declaring
/// neither, which PlantUML refuses, takes no room.
fn declared_size(svg: &str, name: &str) -> u32 {
    static VIEWBOX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"viewBox[= "']+([0-9.]+)[\s,]+([0-9.]+)[\s,]+([0-9.]+)[\s,]+([0-9.]+)"#)
            .unwrap()
    });
    if let Some(captures) = VIEWBOX.captures(svg) {
        let group = if name == "width" { 3 } else { 4 };
        return captures[group]
            .parse::<f64>()
            .map_or(0, |size| size.ceil() as u32);
    }
    Regex::new(&format!(r"(?i)<svg[^>]+{name}\W+(\d+)"))
        .unwrap()
        .captures(svg)
        .and_then(|captures| captures[1].parse().ok())
        .unwrap_or(0)
}

fn gray_scale(rgb: u32) -> u32 {
    XColor::from_rgb(rgb).gray_scale()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct URectangle {
    pub width: f64,
    pub height: f64,
    pub rx: f64,
    pub ry: f64,
}

impl URectangle {
    pub const fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            rx: 0.0,
            ry: 0.0,
        }
    }

    #[must_use]
    pub const fn rounded(self, corner: f64) -> Self {
        Self {
            rx: corner,
            ry: corner,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_images_measure_their_view_box_rounded_up_or_else_their_size_attributes() {
        let image = UImageSvg::new(r#"<svg viewBox="0 0 10.5 7" width="99">"#.to_owned(), 2.0);
        assert_eq!((image.width(), image.height()), (22.0, 14.0));
        let image = UImageSvg::new(r#"<svg width="30px" height="20px">"#.to_owned(), 1.0);
        assert_eq!((image.data_width(), image.data_height()), (30, 20));
    }

    #[test]
    fn embedded_svg_loses_its_declaration_and_root_attributes() {
        let image = UImageSvg::new(
            r#"<?xml version="1.0"?><svg width="1" height="1"><g/></svg>"#.to_owned(),
            1.0,
        );
        assert_eq!(image.svg().unwrap(), "<svg><g/></svg>");
    }

    #[test]
    fn documents_whose_root_is_not_svg_are_not_embedded() {
        for document in [r#"<html><svg width="1" height="1"/></html>"#, "<svg"] {
            assert_eq!(UImageSvg::new(document.to_owned(), 1.0).svg(), None);
        }
    }
}

use std::cell::OnceCell;
use std::fmt;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::TextBlock;
use super::compress::CompressionMode;
use super::font::{FontConfiguration, UFont};
use super::geom::{MinMax, XDimension2D, XPoint2D};
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
    Polygon(UPolygon),
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
    /// A note for whoever reads the document, which only SVG and the debug listing keep.
    Comment(String),
    /// A swimlane title centred in its lane; only the compression layer of activity diagrams draws it.
    CenteredText(CenteredText),
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
            Self::Comment(_) => "UComment",
            Self::CenteredText(_) => "CenteredText",
        }
    }

    /// A closed shape through `points`, relative to the current position.
    pub(crate) fn polygon(points: Vec<(f64, f64)>) -> Self {
        Self::Polygon(UPolygon::new(points))
    }
}

/// A closed shape through points relative to the current position, with their bounding box.
#[derive(Clone, Debug, PartialEq)]
pub struct UPolygon {
    points: Vec<(f64, f64)>,
    min_max: MinMax,
    /// The compression pass that skips the polygon: the arrowheads of arrows that take no room
    /// (`compressionMode`).
    compression_mode: Option<CompressionMode>,
}

impl UPolygon {
    pub(crate) fn new(points: Vec<(f64, f64)>) -> Self {
        let min_max = points
            .iter()
            .fold(MinMax::empty(), |min_max, &(x, y)| min_max.add_point(x, y));
        Self {
            points,
            min_max,
            compression_mode: None,
        }
    }

    pub(crate) fn points(&self) -> &[(f64, f64)] {
        &self.points
    }

    /// The bounding box of the points; empty, with minimums above maximums, when there are none.
    pub(crate) fn min_max(&self) -> MinMax {
        self.min_max
    }

    #[allow(dead_code, reason = "read by SlotFinder, Phase 6 stage E1")]
    pub(crate) fn get_compression_mode(&self) -> Option<CompressionMode> {
        self.compression_mode
    }

    /// PlantUML sets this on an arrow's own arrowhead, so it stays set for every later drawing of that
    /// arrow.
    pub(crate) fn set_compression_mode(&mut self, compression_mode: CompressionMode) {
        self.compression_mode = Some(compression_mode);
    }
}

/// A title drawn centred in `total_width` (PlantUML's `activitydiagram3.ftile.CenteredText`). Compressing
/// across changes that width, so only the compression layer can centre it; output formats and measuring
/// surfaces skip it.
#[derive(Clone)]
pub struct CenteredText {
    pub(crate) text: Rc<dyn TextBlock>,
    pub(crate) total_width: f64,
}

/// The same title, as PlantUML compares shapes: by identity.
impl PartialEq for CenteredText {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.text, &other.text) && self.total_width == other.total_width
    }
}

impl fmt::Debug for CenteredText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CenteredText")
            .field("total_width", &self.total_width)
            .finish_non_exhaustive()
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
    /// `UPath.arcTo(end, radius, 0, sweep)`: the shorter arc of a circle to `end`, clockwise when `sweep`.
    pub(crate) fn arc_to(end: (f64, f64), radius: f64, sweep: bool) -> Self {
        Self::ArcTo {
            radius: (radius, radius),
            x_axis_rotation: 0.0,
            large_arc: false,
            sweep,
            end,
        }
    }

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

/// A whole ellipse, or only its arc from `start` over `extend` degrees when either is non-zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UEllipse {
    pub width: f64,
    pub height: f64,
    pub start: f64,
    pub extend: f64,
}

impl UEllipse {
    pub const fn new(width: f64, height: f64) -> Self {
        Self::arc(width, height, 0.0, 0.0)
    }

    pub const fn arc(width: f64, height: f64, start: f64, extend: f64) -> Self {
        Self {
            width,
            height,
            start,
            extend,
        }
    }

    pub(crate) fn is_arc(&self) -> bool {
        self.start != 0.0 || self.extend != 0.0
    }

    #[must_use]
    pub(crate) fn bigger(self, more: f64) -> Self {
        Self::new(self.width + more, self.height + more)
    }

    #[must_use]
    pub(crate) fn scale(self, factor: f64) -> Self {
        Self::new(self.width * factor, self.height * factor)
    }

    /// Where the ellipse's outline starts on the line at height `y`.
    pub(crate) fn get_starting_x(self, y: f64) -> f64 {
        let y = y / self.height * 2.0;
        let x = 1.0 - (1.0 - (y - 1.0) * (y - 1.0)).sqrt();
        x * self.width / 2.0
    }

    /// Where the ellipse's outline ends on the line at height `y`.
    pub(crate) fn get_ending_x(self, y: f64) -> f64 {
        let y = y / self.height * 2.0;
        let x = 1.0 + (1.0 - (y - 1.0) * (y - 1.0)).sqrt();
        x * self.width / 2.0
    }

    pub(crate) fn get_point_at_angle(self, alpha: f64) -> XPoint2D {
        let x = self.width / 2.0 + self.width / 2.0 * alpha.cos();
        let y = self.height / 2.0 + self.height / 2.0 * alpha.sin();
        XPoint2D::new(x, y)
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
        let (data_width, data_height) = svg_declared_size(&svg);
        Self {
            svg,
            scale,
            data_width,
            data_height,
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

/// The width and height an SVG document declares: its viewBox's size rounded up, else its root's `width`
/// and `height` attributes. A document declaring neither, which PlantUML refuses, takes no room.
pub(crate) fn svg_declared_size(svg: &str) -> (u32, u32) {
    static VIEWBOX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"viewBox[= "']+([0-9.]+)[\s,]+([0-9.]+)[\s,]+([0-9.]+)[\s,]+([0-9.]+)"#)
            .unwrap()
    });
    static WIDTH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)<svg[^>]+width\W+(\d+)").unwrap());
    static HEIGHT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)<svg[^>]+height\W+(\d+)").unwrap());

    if let Some(captures) = VIEWBOX.captures(svg) {
        let rounded_up = |group: usize| {
            captures[group]
                .parse::<f64>()
                .map_or(0, |size| size.ceil() as u32)
        };
        return (rounded_up(3), rounded_up(4));
    }
    let attribute = |pattern: &Regex| {
        pattern
            .captures(svg)
            .and_then(|captures| captures[1].parse().ok())
            .unwrap_or(0)
    };
    (attribute(&WIDTH), attribute(&HEIGHT))
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
    /// Frames around content take no room of their own when activity diagrams compress across or down.
    ignore_for_compression_on_x: bool,
    ignore_for_compression_on_y: bool,
}

impl URectangle {
    pub const fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            rx: 0.0,
            ry: 0.0,
            ignore_for_compression_on_x: false,
            ignore_for_compression_on_y: false,
        }
    }

    #[must_use]
    pub(crate) const fn ignore_for_compression_on_x(self) -> Self {
        Self {
            ignore_for_compression_on_x: true,
            ..self
        }
    }

    #[must_use]
    pub(crate) const fn ignore_for_compression_on_y(self) -> Self {
        Self {
            ignore_for_compression_on_y: true,
            ..self
        }
    }

    #[allow(dead_code, reason = "read by SlotFinder, Phase 6 stage E1")]
    pub(crate) fn is_ignore_for_compression_on(self, mode: CompressionMode) -> bool {
        match mode {
            CompressionMode::OnX => self.ignore_for_compression_on_x,
            CompressionMode::OnY => self.ignore_for_compression_on_y,
        }
    }

    /// The same rectangle `width` wide, as compressing across makes it (`withWidth`).
    #[allow(dead_code, reason = "used by UGraphicCompressOnXorY, Phase 6 stage E1")]
    #[must_use]
    pub(crate) const fn with_width(self, width: f64) -> Self {
        Self { width, ..self }
    }

    /// The same rectangle `height` high, as compressing down makes it (`withHeight`).
    #[allow(dead_code, reason = "used by UGraphicCompressOnXorY, Phase 6 stage E1")]
    #[must_use]
    pub(crate) const fn with_height(self, height: f64) -> Self {
        Self { height, ..self }
    }

    #[must_use]
    pub const fn rounded(self, corner: f64) -> Self {
        Self {
            rx: corner,
            ry: corner,
            ..self
        }
    }

    /// `halfRounded`: the rectangle with only its top corners rounded.
    pub(crate) fn half_rounded(self, round_corner: f64) -> UShape {
        if round_corner == 0.0 {
            return UShape::Rectangle(self);
        }
        let (width, height, r) = (self.width, self.height, round_corner / 2.0);
        UShape::Path(vec![
            USegment::MoveTo(r, 0.0),
            USegment::LineTo(width - r, 0.0),
            USegment::arc_to((width, r), r, true),
            USegment::LineTo(width, height),
            USegment::LineTo(0.0, height),
            USegment::LineTo(0.0, r),
            USegment::arc_to((r, 0.0), r, true),
        ])
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

use std::rc::Rc;

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
#[derive(Clone, Debug, PartialEq)]
pub struct UImage {
    image_scale1: Rc<PortableImage>,
    scale: f64,
    /// The pixels drawn, scaled from `image_scale1`.
    image: Rc<PortableImage>,
}

impl UImage {
    pub(crate) fn new(image: PortableImage) -> Self {
        let image = Rc::new(image);
        Self {
            image_scale1: image.clone(),
            scale: 1.0,
            image,
        }
    }

    /// Scaled from the image as given, so that scaling twice resamples once.
    #[must_use]
    pub(crate) fn scale(&self, scale: f64) -> Self {
        let scale = self.scale * scale;
        Self {
            image_scale1: self.image_scale1.clone(),
            scale,
            image: Rc::new(self.image_scale1.scale(scale)),
        }
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
        Self {
            image: Rc::new(copy.scale(self.scale)),
            image_scale1: Rc::new(copy),
            scale: self.scale,
        }
    }

    /// The pixels drawn.
    pub(crate) fn image(&self) -> &PortableImage {
        &self.image
    }

    /// One less than the pixels drawn across, as PlantUML measures images.
    pub(crate) fn width(&self) -> f64 {
        self.image.width() as f64 - 1.0
    }

    pub(crate) fn height(&self) -> f64 {
        self.image.height() as f64 - 1.0
    }
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

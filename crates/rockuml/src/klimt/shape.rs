use super::font::FontConfiguration;
use super::geom::XDimension2D;

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
    /// A bitmap of this many pixels.
    Image {
        width: f64,
        height: f64,
    },
    /// Takes up space without drawing anything.
    Empty(XDimension2D),
    /// A separator across whatever contains it; only containers that know their width can draw it.
    HorizontalLine,
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
            Self::Empty(_) => "UEmpty",
            Self::Image { .. } => "UImage",
            Self::HorizontalLine => "UHorizontalLine",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UText {
    pub text: String,
    pub font: FontConfiguration,
    pub orientation: i32,
}

impl UText {
    pub fn new(text: &str, font: FontConfiguration) -> Self {
        Self {
            text: crate::jaws::make_newlines_visible(text),
            font,
            orientation: 0,
        }
    }
}

/// An ellipse, or an arc of one when `extend` is non-zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UEllipse {
    pub width: f64,
    pub height: f64,
    pub start: f64,
    pub extend: f64,
}

impl UEllipse {
    pub const fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            start: 0.0,
            extend: 0.0,
        }
    }
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

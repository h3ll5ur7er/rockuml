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
    /// Segments relative to the current position. Closing a path adds nothing, as in PlantUML.
    Path(Vec<USegment>),
    Image(UImage),
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
            Self::Path(_) => "UPath",
            Self::Empty(_) => "UEmpty",
            Self::Image(_) => "UImage",
            Self::HorizontalLine => "UHorizontalLine",
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

/// A bitmap, kept as PNG data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UImage {
    pub png: &'static [u8],
    pub width: f64,
    pub height: f64,
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

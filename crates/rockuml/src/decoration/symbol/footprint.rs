//! The smallest ellipse of given proportions around what a text block draws (PlantUML's `svek.image.Footprint`,
//! `ContainingEllipse`, `YTransformer`, `SmallestEnclosingCircle` and `Circle`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::clip::path_bounds;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UGraphicBackend, UParam};

/// The ellipse `alpha` times as high as wide around the corners of what `drawable` draws.
pub(super) fn get_ellipse(
    string_bounder: &dyn StringBounder,
    drawable: &dyn TextBlock,
    alpha: f64,
) -> ContainingEllipse {
    let footprint = Rc::new(RefCell::new(Footprint {
        string_bounder: string_bounder.shared(),
        all: Vec::new(),
    }));
    let ug = UGraphic::new(footprint.clone(), string_bounder.shared(), HColor::WHITE);
    drawable.draw_u(&ug);
    let mut circle = ContainingEllipse::new(alpha);
    for &pt in &footprint.borrow().all {
        circle.append(pt);
    }
    circle
}

/// Records the corners of the shapes drawn, separators aside (`Footprint.MyUGraphic`).
struct Footprint {
    string_bounder: Rc<dyn StringBounder>,
    all: Vec<XPoint2D>,
}

impl Footprint {
    fn add_point(&mut self, x: f64, y: f64) {
        self.all.push(XPoint2D::new(x, y));
    }
}

impl UGraphicBackend for Footprint {
    fn draw(&mut self, shape: &UShape, at: UTranslate, _param: &UParam) {
        let (x, y) = (at.dx, at.dy);
        match shape {
            UShape::Text(text) => {
                let dim = self
                    .string_bounder
                    .calculate_dimension(&text.font.font(), &text.text);
                let y = y - (dim.height - 1.5);
                self.add_point(x, y);
                self.add_point(x, y + dim.height);
                self.add_point(x + dim.width, y);
                self.add_point(x + dim.width, y + dim.height);
            }
            UShape::Image(image) => {
                self.add_point(x, y);
                self.add_point(x, y + image.height());
                self.add_point(x + image.width(), y);
                self.add_point(x + image.width(), y + image.height());
            }
            UShape::Path(segments) => {
                if let Some((min_x, min_y, max_x, max_y)) = path_bounds(segments) {
                    self.add_point(x + min_x, y + min_y);
                    self.add_point(x + max_x, y + max_y);
                }
            }
            UShape::Rectangle(rectangle) => {
                self.add_point(x, y);
                self.add_point(x + rectangle.width, y + rectangle.height);
            }
            UShape::Ellipse(ellipse) => {
                self.add_point(x, y);
                self.add_point(x + ellipse.width, y + ellipse.height);
            }
            UShape::Empty(dimension) => {
                self.add_point(x, y);
                self.add_point(x + dimension.width, y + dimension.height);
            }
            // Lines are separators, which the ellipse need not hold; PlantUML fails on the other shapes.
            UShape::Line { .. }
            | UShape::HorizontalLine
            | UShape::Polygon(_)
            | UShape::ImageSvg(_)
            | UShape::CenteredCharacter(_)
            | UShape::SpecialText => {}
        }
    }
}

/// The smallest circle around points squeezed vertically by `alpha`, stretched back into an ellipse.
pub(super) struct ContainingEllipse {
    sec: SmallestEnclosingCircle,
    ytransformer: YTransformer,
}

impl ContainingEllipse {
    fn new(coef_y: f64) -> Self {
        Self {
            sec: SmallestEnclosingCircle::default(),
            ytransformer: YTransformer::new(coef_y),
        }
    }

    fn append(&mut self, pt: XPoint2D) {
        self.sec.append(self.ytransformer.get_reverse_point_2d(pt));
    }

    pub(super) fn as_u_ellipse(&self) -> UEllipse {
        let radius = self.sec.get_circle().radius;
        UEllipse::new(2.0 * radius, 2.0 * radius * self.ytransformer.alpha)
    }

    pub(super) fn get_center(&self) -> XPoint2D {
        self.ytransformer.get_point_2d(self.sec.get_circle().center)
    }
}

struct YTransformer {
    alpha: f64,
}

impl YTransformer {
    /// A degenerate block, measuring 0 by 0, keeps its proportions.
    fn new(alpha: f64) -> Self {
        Self {
            alpha: if alpha.is_nan() { 1.0 } else { alpha },
        }
    }

    fn get_point_2d(&self, pt: XPoint2D) -> XPoint2D {
        XPoint2D::new(pt.x, pt.y * self.alpha)
    }

    fn get_reverse_point_2d(&self, pt: XPoint2D) -> XPoint2D {
        XPoint2D::new(pt.x, pt.y / self.alpha)
    }
}

/// Welzl's algorithm over distinct points, in the order they came.
#[derive(Default)]
struct SmallestEnclosingCircle {
    all: Vec<XPoint2D>,
}

impl SmallestEnclosingCircle {
    fn append(&mut self, pt: XPoint2D) {
        if !self.all.contains(&pt) {
            self.all.push(pt);
        }
    }

    fn get_circle(&self) -> Circle {
        Self::find_sec(self.all.len(), &self.all, 0, &mut self.all.clone())
    }

    /// The smallest circle around the first `n` points of `p` with the first `m` points of `b` on it.
    fn find_sec(n: usize, p: &[XPoint2D], m: usize, b: &mut [XPoint2D]) -> Circle {
        let mut sec = match m {
            1 => Circle::new(b[0]),
            2 => Circle::through(b[0], b[1]),
            3 => return Circle::get_circle(b[0], b[1], b[2]),
            _ => Circle::new(XPoint2D::new(0.0, 0.0)),
        };
        for i in 0..n {
            if sec.is_outside(p[i]) {
                b[m] = p[i];
                sec = Self::find_sec(i, p, m + 1, b);
            }
        }
        sec
    }
}

#[derive(Clone, Copy)]
struct Circle {
    center: XPoint2D,
    radius: f64,
}

impl Circle {
    fn new(center: XPoint2D) -> Self {
        Self {
            center,
            radius: 0.0,
        }
    }

    /// The circle with `p1` and `p2` at the ends of a diameter.
    fn through(p1: XPoint2D, p2: XPoint2D) -> Self {
        let center = XPoint2D::new(f64::midpoint(p1.x, p2.x), f64::midpoint(p1.y, p2.y));
        Self {
            center,
            radius: distance(p1, center),
        }
    }

    /// The circle through three points, ordered so that the formula never divides by zero.
    fn get_circle(p1: XPoint2D, p2: XPoint2D, p3: XPoint2D) -> Self {
        if p3.y == p2.y {
            Self::circumscribed(p2, p1, p3)
        } else {
            Self::circumscribed(p1, p2, p3)
        }
    }

    fn circumscribed(p1: XPoint2D, p2: XPoint2D, p3: XPoint2D) -> Self {
        let num1 = p3.x * p3.x * (p1.y - p2.y)
            + (p1.x * p1.x + (p1.y - p2.y) * (p1.y - p3.y)) * (p2.y - p3.y)
            + p2.x * p2.x * (-p1.y + p3.y);
        let den1 = 2.0 * (p3.x * (p1.y - p2.y) + p1.x * (p2.y - p3.y) + p2.x * (-p1.y + p3.y));
        let x = num1 / den1;
        let den2 = p3.y - p2.y;
        let y = f64::midpoint(p2.y, p3.y) - (p3.x - p2.x) / den2 * (x - f64::midpoint(p2.x, p3.x));
        let center = XPoint2D::new(x, y);
        Self {
            center,
            radius: distance(center, p1),
        }
    }

    fn is_outside(&self, point: XPoint2D) -> bool {
        distance(self.center, point) > self.radius
    }
}

/// `XPoint2D.distance`.
fn distance(from: XPoint2D, to: XPoint2D) -> f64 {
    let px = to.x - from.x;
    let py = to.y - from.y;
    (px * px + py * py).sqrt()
}

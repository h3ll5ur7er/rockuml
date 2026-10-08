//! The hollow triangles of redefinition (`<||`, crossed by a bar) and definition (`<|:`, followed by two
//! dots).

use super::{Extremity, ExtremityFactory, change_back, manageround};
use crate::color::HColor;
use crate::klimt::UDrawable;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct ExtremityFactoryExtendsLike {
    background_color: HColor,
    defined_by: bool,
}

impl ExtremityFactoryExtendsLike {
    pub(crate) fn new(background_color: HColor, defined_by: bool) -> Self {
        Self {
            background_color,
            defined_by,
        }
    }
}

impl ExtremityFactory for ExtremityFactoryExtendsLike {
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity> {
        let background_color = self.background_color.clone();
        Box::new(if self.defined_by {
            ExtremityExtendsLike::defined_by(p0, angle, background_color)
        } else {
            ExtremityExtendsLike::redefines(p0, angle, background_color)
        })
    }
}

const XLEN: f64 = -19.0;
const HALF_WIDTH: f64 = 7.0;

struct ExtremityExtendsLike {
    trig: UShape,
    back: HColor,
    mark: Mark,
}

/// What follows the triangle.
enum Mark {
    Redefines { pos: UTranslate, bar: UShape },
    DefinedBy { pos1: UTranslate, pos2: UTranslate },
}

impl ExtremityExtendsLike {
    fn new(porig: XPoint2D, angle: f64, background_color: HColor, mark: Mark) -> Self {
        let angle = manageround(angle);
        let trig_point = |x: f64, y: f64| {
            let p = Point::new(x, y).rotate(angle);
            (p.x + porig.x, p.y + porig.y)
        };
        let trig = UShape::polygon(vec![
            (porig.x, porig.y),
            trig_point(XLEN, -HALF_WIDTH),
            trig_point(XLEN, HALF_WIDTH),
            (porig.x, porig.y),
        ]);
        Self {
            trig,
            back: background_color,
            mark,
        }
    }

    /// The bar is placed along the angle as given, not the rounded one the triangle follows.
    fn redefines(porig: XPoint2D, angle: f64, background_color: HColor) -> Self {
        const XSUFFIX: f64 = XLEN * 1.2;
        let p1 = Point::new(XSUFFIX, -HALF_WIDTH).rotate(angle);
        let p2 = Point::new(XSUFFIX, HALF_WIDTH).rotate(angle);
        let mark = Mark::Redefines {
            pos: p1.pos(porig),
            bar: UShape::Line {
                dx: p2.x - p1.x,
                dy: p2.y - p1.y,
            },
        };
        Self::new(porig, angle, background_color, mark)
    }

    /// The dots are placed along the angle as given, not the rounded one the triangle follows.
    fn defined_by(porig: XPoint2D, angle: f64, background_color: HColor) -> Self {
        const XSUFFIX: f64 = XLEN * 1.3;
        let w = HALF_WIDTH - DOT_HALF_SIZE;
        let dot_pos = |y: f64| {
            let mut p = Point::new(XSUFFIX, y).rotate(angle);
            p.x -= DOT_HALF_SIZE;
            p.y -= DOT_HALF_SIZE;
            p.pos(porig)
        };
        let mark = Mark::DefinedBy {
            pos1: dot_pos(-w),
            pos2: dot_pos(w),
        };
        Self::new(porig, angle, background_color, mark)
    }
}

const DOT_HALF_SIZE: f64 = 2.0;

impl UDrawable for ExtremityExtendsLike {
    fn draw_u(&self, ug: &UGraphic) {
        ug.with_backcolor(self.back.clone()).draw(&self.trig);
        match &self.mark {
            Mark::Redefines { pos, bar } => ug
                .with_stroke(UStroke::with_thickness(2.0))
                .translated(pos.dx, pos.dy)
                .draw(bar),
            Mark::DefinedBy { pos1, pos2 } => {
                let size = DOT_HALF_SIZE + DOT_HALF_SIZE;
                let dot = UShape::Ellipse(UEllipse::new(size, size));
                let ug = change_back(ug);
                ug.translated(pos1.dx, pos1.dy).draw(&dot);
                ug.translated(pos2.dx, pos2.dy).draw(&dot);
            }
        }
    }
}

impl Extremity for ExtremityExtendsLike {
    fn decoration_length(&self) -> f64 {
        18.0
    }
}

/// A point turned the way these decorations turn it, which mirrors it across the x axis as well.
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn rotate(self, theta: f64) -> Self {
        let ct = theta.cos();
        let st = -theta.sin();
        Self::new(self.x * ct - self.y * st, -self.x * st - self.y * ct)
    }

    fn pos(self, pt: XPoint2D) -> UTranslate {
        UTranslate::new(self.x + pt.x, self.y + pt.y)
    }
}

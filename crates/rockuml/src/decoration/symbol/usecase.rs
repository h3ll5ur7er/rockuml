//! An ellipse around the text (PlantUML's `USymbolUsecase`, with `klimt.shape.TextBlockInEllipse` and
//! `svek.image.RotatedEllipse`). A business use case's ellipse has a chord across its right end. Separators in
//! the text span the ellipse.

use std::f64::consts::PI;
use std::rc::Rc;

use super::SmallContent;
use super::footprint::{self, ContainingEllipse};
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XPoint2D};
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::stencil::{HorizontalLineDrawer, Stencil, UHorizontalLine};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};

pub(super) fn as_small(business: bool, content: SmallContent) -> Box<dyn TextBlock> {
    Box::new(SmallUsecase { business, content })
}

struct SmallUsecase {
    business: bool,
    content: SmallContent,
}

impl SmallUsecase {
    /// The stereotype above the label, with room for the chord of a business use case.
    fn desc(&self) -> Box<dyn TextBlock + '_> {
        let tmp = self.content.text(HorizontalAlignment::Center);
        if self.business {
            Box::new(TextBlockMarged::new(
                tmp,
                ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 7.0, 0.0, 7.0),
            ))
        } else {
            Box::new(tmp)
        }
    }
}

impl TextBlock for SmallUsecase {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        TextBlockInEllipse::new(self.desc(), string_bounder).calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = self.content.fashion.apply(ug);
        let ellipse = TextBlockInEllipse::new(self.desc(), ug.string_bounder());
        let frontier = ellipse.get_u_ellipse();
        ellipse
            .draw_u(&ug.with_horizontal_line_drawer(Rc::new(EllipseLines { ellipse: frontier })));
        if self.business {
            special_business(&ug, frontier);
        }
    }
}

/// The chord between the points 20 degrees above the right end and its mirror on an ellipse turned by 45
/// degrees, slightly inside the outline.
fn special_business(ug: &UGraphic, frontier: UEllipse) {
    let rotated_ellipse = RotatedEllipse {
        ellipse: frontier,
        beta: PI / 4.0,
    };
    let theta1 = 20.0 * PI / 180.0;
    let theta2 = rotated_ellipse.get_other_theta(theta1);
    let frontier2 = frontier.scale(0.99);
    let p1 = frontier2.get_point_at_angle(-theta1);
    let p2 = frontier2.get_point_at_angle(-theta2);
    ug.translated(p1.x, p1.y).draw(&UShape::Line {
        dx: p2.x - p1.x,
        dy: p2.y - p1.y,
    });
}

/// Separators span the ellipse at their height (`MyUGraphicEllipse`).
struct EllipseLines {
    ellipse: UEllipse,
}

/// The ellipse's extent, `dy` lower.
struct EllipseStencil {
    ellipse: UEllipse,
    dy: f64,
}

impl Stencil for EllipseStencil {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, y: f64) -> f64 {
        self.ellipse.get_starting_x(y + self.dy)
    }

    fn ending_x(&self, _string_bounder: &dyn StringBounder, y: f64) -> f64 {
        self.ellipse.get_ending_x(y + self.dy)
    }
}

impl HorizontalLineDrawer for EllipseLines {
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, y: f64) {
        let stencil = EllipseStencil {
            ellipse: self.ellipse,
            dy: y,
        };
        line.draw_line_internal(&ug.translated(0.0, y), &stencil, 0.0);
    }
}

/// A text block in the smallest ellipse around it, a little bigger, about as much higher than wide as the text
/// (PlantUML's `TextBlockInEllipse`).
struct TextBlockInEllipse<T> {
    text: T,
    ellipse: ContainingEllipse,
}

impl<T: TextBlock> TextBlockInEllipse<T> {
    fn new(text: T, string_bounder: &dyn StringBounder) -> Self {
        let text_dim = text.calculate_dimension(string_bounder);
        let alpha = (text_dim.height / text_dim.width).clamp(0.2, 0.8);
        let ellipse = footprint::get_ellipse(string_bounder, &text, alpha);
        Self { text, ellipse }
    }

    fn get_u_ellipse(&self) -> UEllipse {
        self.ellipse.as_u_ellipse().bigger(6.0)
    }
}

impl<T: TextBlock> TextBlock for TextBlockInEllipse<T> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        let ellipse = self.get_u_ellipse();
        XDimension2D::new(ellipse.width, ellipse.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sh = self.get_u_ellipse();
        let center = self.ellipse.get_center();
        let dx = sh.width / 2.0 - center.x;
        let dy = sh.height / 2.0 - center.y;
        ug.draw(&UShape::Ellipse(sh));
        self.text.draw_u(&ug.translated(dx, dy - 2.0));
    }
}

/// An ellipse turned by `beta` around its centre.
struct RotatedEllipse {
    ellipse: UEllipse,
    beta: f64,
}

impl RotatedEllipse {
    fn get_a(&self) -> f64 {
        self.ellipse.width / 2.0
    }

    fn get_b(&self) -> f64 {
        self.ellipse.height / 2.0
    }

    fn get_point(&self, theta: f64) -> XPoint2D {
        let x = self.get_a() * theta.cos();
        let y = self.get_b() * theta.sin();
        let xp = x * self.beta.cos() - y * self.beta.sin();
        let yp = x * self.beta.sin() + y * self.beta.cos();
        XPoint2D::new(xp, yp)
    }

    /// The other angle whose point has the same x as `theta1`'s.
    fn get_other_theta(&self, theta1: f64) -> f64 {
        let z = self.get_point(theta1).x;
        let a = self.get_a() * self.beta.cos();
        let b = self.get_b() * self.beta.sin();
        let sum = 2.0 * a * z / (a * a + b * b);
        let other = sum - theta1.cos();
        -other.acos()
    }
}

//! A cloud of bubbles (PlantUML's `USymbolCloud`), randomised by a generator seeded with the cloud's size so
//! that a cloud always looks the same.

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::java::Random;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::{XDimension2D, XPoint2D};
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolCloud;

fn draw_cloud(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(&UShape::Path(get_specific_frontier_for_cloud(
        width, height,
    )));
}

fn get_specific_frontier_for_cloud(width: f64, height: f64) -> Vec<USegment> {
    let mut rnd = Random::new(width as i64 + 7919 * height as i64);
    let mut points = Vec::new();
    let mut bubble_size = 11.0;
    if width.max(height) / bubble_size > 16.0 {
        bubble_size = width.max(height) / 16.0;
    }
    let margin1 = 8.0;
    let point_a = XPoint2D::new(margin1, margin1);
    let point_b = XPoint2D::new(width - margin1, margin1);
    let point_c = XPoint2D::new(width - margin1, height - margin1);
    let point_d = XPoint2D::new(margin1, height - margin1);
    let corners = [point_a, point_b, point_c, point_d];
    if width > 100.0 && height > 100.0 {
        complex(&mut rnd, &mut points, bubble_size, corners);
    } else {
        simple(&mut rnd, &mut points, bubble_size, corners);
    }
    points.push(points[0]);
    let mut result = vec![USegment::MoveTo(points[0].x, points[0].y)];
    for pair in points.windows(2) {
        result.push(add_curve(&mut rnd, pair[0], pair[1]));
    }
    result
}

/// Bubbles along the sides, the corners cut by straight lines.
fn complex(
    rnd: &mut Random,
    points: &mut Vec<XPoint2D>,
    bubble_size: f64,
    [point_a, point_b, point_c, point_d]: [XPoint2D; 4],
) {
    let margin2 = 7.0;
    special_line(
        bubble_size,
        rnd,
        points,
        mv_x(point_a, margin2),
        mv_x(point_b, -margin2),
    );
    points.push(mv_y(point_b, margin2));
    special_line(
        bubble_size,
        rnd,
        points,
        mv_y(point_b, margin2),
        mv_y(point_c, -margin2),
    );
    points.push(mv_x(point_c, -margin2));
    special_line(
        bubble_size,
        rnd,
        points,
        mv_x(point_c, -margin2),
        mv_x(point_d, margin2),
    );
    points.push(mv_y(point_d, -margin2));
    special_line(
        bubble_size,
        rnd,
        points,
        mv_y(point_d, -margin2),
        mv_y(point_a, margin2),
    );
    points.push(mv_x(point_a, margin2));
}

fn simple(rnd: &mut Random, points: &mut Vec<XPoint2D>, bubble_size: f64, corners: [XPoint2D; 4]) {
    for side in 0..4 {
        special_line(
            bubble_size,
            rnd,
            points,
            corners[side],
            corners[(side + 1) % 4],
        );
    }
}

fn mv_x(pt: XPoint2D, dx: f64) -> XPoint2D {
    XPoint2D::new(pt.x + dx, pt.y)
}

fn mv_y(pt: XPoint2D, dy: f64) -> XPoint2D {
    XPoint2D::new(pt.x, pt.y + dy)
}

/// Bubbles from `p1` to `p2` through a point pushed out of the middle.
fn special_line(
    bubble_size: f64,
    rnd: &mut Random,
    points: &mut Vec<XPoint2D>,
    p1: XPoint2D,
    p2: XPoint2D,
) {
    let change = CoordinateChange::create(p1, p2);
    let length = change.len;
    let middle = change.get_true_coordinate(
        length / 2.0,
        -rnd_between(rnd, 1.0, 1.0 + 12.0_f64.min(bubble_size * 0.8)),
    );
    bubble_line(rnd, points, p1, middle, bubble_size);
    bubble_line(rnd, points, middle, p2, bubble_size);
}

fn bubble_line(
    rnd: &mut Random,
    points: &mut Vec<XPoint2D>,
    p1: XPoint2D,
    p2: XPoint2D,
    mut bubble_size: f64,
) {
    let change = CoordinateChange::create(p1, p2);
    let length = change.len;
    let mut nb = (length / bubble_size) as i32;
    if nb == 0 {
        bubble_size = length / 2.0;
        nb = (length / bubble_size) as i32;
    }
    for i in 0..nb {
        let on_line = change.get_true_coordinate(f64::from(i) * length / f64::from(nb), 0.0);
        points.push(rnd_point(rnd, on_line, bubble_size * 0.2));
    }
}

fn add_curve(rnd: &mut Random, p1: XPoint2D, p2: XPoint2D) -> USegment {
    let change = CoordinateChange::create(p1, p2);
    let length = change.len;
    let coef = rnd_between(rnd, 0.25, 0.35);
    let middle = change.get_true_coordinate(length * coef, -length * rnd_between(rnd, 0.4, 0.55));
    let middle2 =
        change.get_true_coordinate(length * (1.0 - coef), -length * rnd_between(rnd, 0.4, 0.55));
    USegment::CubicTo {
        ctrl1: (middle.x, middle.y),
        ctrl2: (middle2.x, middle2.y),
        end: (p2.x, p2.y),
    }
}

fn rnd_between(rnd: &mut Random, a: f64, b: f64) -> f64 {
    rnd.next_double() * (b - a) + a
}

fn rnd_point(rnd: &mut Random, pt: XPoint2D, v: f64) -> XPoint2D {
    let x = pt.x + v * rnd.next_double();
    let y = pt.y + v * rnd.next_double();
    XPoint2D::new(x, y)
}

/// Coordinates along the line from one point to another and across it (PlantUML's `CoordinateChange`).
struct CoordinateChange {
    x1: f64,
    y1: f64,
    vect_u_x: f64,
    vect_u_y: f64,
    vect_v_x: f64,
    vect_v_y: f64,
    len: f64,
}

impl CoordinateChange {
    fn create(p1: XPoint2D, p2: XPoint2D) -> Self {
        let (dx, dy) = (p1.x - p2.x, p1.y - p2.y);
        let len = (dx * dx + dy * dy).sqrt();
        let vect_u_x = (p2.x - p1.x) / len;
        let vect_u_y = (p2.y - p1.y) / len;
        Self {
            x1: p1.x,
            y1: p1.y,
            vect_u_x,
            vect_u_y,
            vect_v_x: -vect_u_y,
            vect_v_y: vect_u_x,
            len,
        }
    }

    /// The point `a` along the line and `b` to its left.
    fn get_true_coordinate(&self, a: f64, b: f64) -> XPoint2D {
        let x = a * self.vect_u_x + b * self.vect_v_x;
        let y = a * self.vect_u_y + b * self.vect_v_y;
        XPoint2D::new(self.x1 + x, self.y1 + y)
    }
}

impl SmallShape for USymbolCloud {
    fn margin(&self) -> Margin {
        Margin::new(15.0, 15.0, 15.0, 15.0)
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, _fashion: &Fashion) {
        draw_cloud(ug, dimension.width, dimension.height);
    }
}

impl BigShape for USymbolCloud {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_cloud(ug, content.width, content.height);
        content.draw_centered_stereotype_and_title(ug, 13.0);
    }
}

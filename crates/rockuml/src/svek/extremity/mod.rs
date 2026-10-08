//! The decorations drawn where links meet their entities (PlantUML's `svek.extremity`), as Smetana layouts
//! draw them.

mod arrow;
mod arrow_and_circle;
mod circle;
mod circle_connect;
mod circle_cross;
mod circle_crowfoot;
mod circle_line;
mod crowfoot;
mod diamond;
mod double_line;
mod extends_like;
mod half_arrow;
mod line_crowfoot;
mod not_navigable;
mod parenthesis;
mod plus;
mod square;
#[cfg(test)]
mod tests;
mod triangle;

use std::f64::consts::PI;

pub(crate) use arrow::ExtremityFactoryArrow;
pub(crate) use arrow_and_circle::ExtremityFactoryArrowAndCircle;
pub(crate) use circle::ExtremityFactoryCircle;
pub(crate) use circle_connect::ExtremityFactoryCircleConnect;
pub(crate) use circle_cross::ExtremityFactoryCircleCross;
pub(crate) use circle_crowfoot::ExtremityFactoryCircleCrowfoot;
pub(crate) use circle_line::ExtremityFactoryCircleLine;
pub(crate) use crowfoot::ExtremityFactoryCrowfoot;
pub(crate) use diamond::ExtremityFactoryDiamond;
pub(crate) use double_line::ExtremityFactoryDoubleLine;
pub(crate) use extends_like::ExtremityFactoryExtendsLike;
pub(crate) use half_arrow::ExtremityFactoryHalfArrow;
pub(crate) use line_crowfoot::ExtremityFactoryLineCrowfoot;
pub(crate) use not_navigable::ExtremityFactoryNotNavigable;
pub(crate) use parenthesis::ExtremityFactoryParenthesis;
pub(crate) use plus::ExtremityFactoryPlus;
pub(crate) use square::ExtremityFactorySquare;
pub(crate) use triangle::ExtremityFactoryTriangle;

use crate::color::HColor;
use crate::decoration::LinkDecor;
use crate::klimt::UDrawable;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;

/// The decoration at one end of a link.
pub(crate) trait Extremity: UDrawable {
    /// How far the decoration reaches back along the link from its end.
    fn decoration_length(&self) -> f64 {
        8.0
    }
}

/// Makes the decoration of one kind of link end.
pub(crate) trait ExtremityFactory {
    /// The decoration at `p0`, the end of a link arriving there along `angle` radians.
    fn create_udrawable(&self, p0: XPoint2D, angle: f64) -> Box<dyn Extremity>;
}

impl LinkDecor {
    /// What draws the decoration; `None` draws nothing. Hollow decorations are filled with `background_color`
    /// (`getExtremityFactoryComplete`).
    pub(crate) fn get_extremity_factory_complete(
        self,
        background_color: HColor,
    ) -> Option<Box<dyn ExtremityFactory>> {
        let bg = background_color;
        Some(match self {
            Self::Extends => Box::new(ExtremityFactoryTriangle::new(18.0, 6.0, 18.0)),
            Self::Plus => Box::new(ExtremityFactoryPlus::new(bg)),
            Self::Redefines => Box::new(ExtremityFactoryExtendsLike::new(bg, false)),
            Self::DefinedBy => Box::new(ExtremityFactoryExtendsLike::new(bg, true)),
            Self::HalfArrowUp => Box::new(ExtremityFactoryHalfArrow::new(1)),
            Self::HalfArrowDown => Box::new(ExtremityFactoryHalfArrow::new(-1)),
            Self::ArrowTriangle => Box::new(ExtremityFactoryTriangle::new(8.0, 3.0, 8.0)),
            Self::Crowfoot => Box::new(ExtremityFactoryCrowfoot),
            Self::CircleCrowfoot => Box::new(ExtremityFactoryCircleCrowfoot),
            Self::LineCrowfoot => Box::new(ExtremityFactoryLineCrowfoot),
            Self::CircleLine => Box::new(ExtremityFactoryCircleLine),
            Self::DoubleLine => Box::new(ExtremityFactoryDoubleLine),
            Self::CircleCross => Box::new(ExtremityFactoryCircleCross::new(bg)),
            Self::Arrow => Box::new(ExtremityFactoryArrow),
            Self::ArrowAndCircle => Box::new(ExtremityFactoryArrowAndCircle::new(bg)),
            Self::NotNavigable => Box::new(ExtremityFactoryNotNavigable),
            Self::Aggregation => Box::new(ExtremityFactoryDiamond::new(false)),
            Self::Composition => Box::new(ExtremityFactoryDiamond::new(true)),
            Self::Circle => Box::new(ExtremityFactoryCircle::new(false, bg)),
            Self::CircleFill => Box::new(ExtremityFactoryCircle::new(true, bg)),
            Self::Square => Box::new(ExtremityFactorySquare::new(bg)),
            Self::Parenthesis => Box::new(ExtremityFactoryParenthesis),
            Self::CircleConnect => Box::new(ExtremityFactoryCircleConnect::new(bg)),
            Self::None => return None,
        })
    }
}

/// Snaps an angle within a twentieth of a degree of a right angle onto it, so that links along an axis get
/// decorations along it.
fn manageround(angle: f64) -> f64 {
    let deg = angle * 180.0 / PI;
    let is_close_to = |value: f64| (value - deg).abs() < 0.05;
    if is_close_to(0.0) {
        return 0.0;
    }
    if is_close_to(90.0) {
        return 90.0 * PI / 180.0;
    }
    if is_close_to(180.0) {
        return 180.0 * PI / 180.0;
    }
    if is_close_to(270.0) {
        return 270.0 * PI / 180.0;
    }
    if is_close_to(360.0) {
        return 0.0;
    }
    angle
}

/// `HColors.changeBack`: shapes are filled with the line colour.
fn change_back(ug: &UGraphic) -> UGraphic {
    ug.with_backcolor(ug.param().color.clone())
}

/// The point at `angle` on the circle of `radius` whose bounding square starts at (`px`, `py`).
fn point_on_circle(px: f64, py: f64, radius: f64, angle: f64) -> XPoint2D {
    XPoint2D::new(
        px + radius + radius * angle.cos(),
        py + radius + radius * angle.sin(),
    )
}

/// A line from `p1` to `p2`, both relative to (`x`, `y`).
fn draw_line(ug: &UGraphic, x: f64, y: f64, p1: XPoint2D, p2: XPoint2D) {
    let (dx, dy) = (p2.x - p1.x, p2.y - p1.y);
    ug.translated(x + p1.x, y + p1.y)
        .draw(&UShape::Line { dx, dy });
}

/// The polygon through `points` turned by `theta` radians around the origin, then moved to `to`, as
/// `UPolygon.rotate` and `translate` do it: turning by exactly 0 leaves the points alone.
fn rotated_polygon(points: &[(f64, f64)], theta: f64, to: XPoint2D) -> UShape {
    let rotate = (theta != 0.0).then(|| XAffineTransform::rotate_instance(theta));
    let points = points
        .iter()
        .map(|&point| rotate.map_or(point, |rotate| rotate.transform(point)))
        .map(|(x, y)| (x + to.x, y + to.y))
        .collect();
    UShape::polygon(points)
}

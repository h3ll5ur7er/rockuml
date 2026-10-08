//! The figures an actor is drawn as, chosen with `skinparam actorStyle` (PlantUML's `ActorStyle`,
//! `ActorStickMan`, `ActorAwesome` and `ActorHollow`).

use std::f64::consts::PI;

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UEllipse, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ActorStyle {
    #[default]
    Stickman,
    StickmanBusiness,
    Awesome,
    Hollow,
}

impl ActorStyle {
    pub(crate) fn named(name: &str) -> Self {
        if name.eq_ignore_ascii_case("awesome") {
            Self::Awesome
        } else if name.eq_ignore_ascii_case("hollow") {
            Self::Hollow
        } else {
            Self::Stickman
        }
    }

    pub(crate) fn text_block(self, fashion: Fashion) -> Box<dyn TextBlock> {
        match self {
            Self::Stickman => Box::new(ActorStickMan {
                fashion,
                actor_business: false,
            }),
            Self::StickmanBusiness => Box::new(ActorStickMan {
                fashion,
                actor_business: true,
            }),
            Self::Awesome => Box::new(ActorAwesome { fashion }),
            Self::Hollow => Box::new(ActorHollow { fashion }),
        }
    }
}

pub(crate) struct ActorStickMan {
    fashion: Fashion,
    /// A business actor's head is crossed by a chord.
    actor_business: bool,
}

impl ActorStickMan {
    const ARMS_Y: f64 = 8.0;
    const ARMS_LENGTH: f64 = 13.0;
    const BODY_LENGTH: f64 = 27.0;
    const LEGS_X: f64 = 13.0;
    const LEGS_Y: f64 = 15.0;
    const HEAD_DIAM: f64 = 16.0;

    fn thickness(&self) -> f64 {
        self.fashion.stroke.thickness
    }

    /// Draws the chord on a surface centred on the head.
    fn special_business(ug: &UGraphic) {
        let alpha = 21.0 * PI / 64.0;
        let on_circle = |alpha: f64| {
            (
                Self::HEAD_DIAM / 2.0 * alpha.cos(),
                Self::HEAD_DIAM / 2.0 * alpha.sin(),
            )
        };
        let (x1, y1) = on_circle(PI / 4.0 + alpha);
        let (x2, y2) = on_circle(PI / 4.0 - alpha);
        ug.translated(x1, y1).draw(&UShape::Line {
            dx: x2 - x1,
            dy: y2 - y1,
        });
    }
}

impl TextBlock for ActorStickMan {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            Self::ARMS_LENGTH.max(Self::LEGS_X) * 2.0 + 2.0 * self.thickness(),
            Self::HEAD_DIAM + Self::BODY_LENGTH + Self::LEGS_Y + 2.0 * self.thickness() + 1.0,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let start_x =
            Self::ARMS_LENGTH.max(Self::LEGS_X) - Self::HEAD_DIAM / 2.0 + self.thickness();
        let center_x = start_x + Self::HEAD_DIAM / 2.0;
        let body = vec![
            USegment::MoveTo(0.0, 0.0),
            USegment::LineTo(0.0, Self::BODY_LENGTH),
            USegment::MoveTo(-Self::ARMS_LENGTH, Self::ARMS_Y),
            USegment::LineTo(Self::ARMS_LENGTH, Self::ARMS_Y),
            USegment::MoveTo(0.0, Self::BODY_LENGTH),
            USegment::LineTo(-Self::LEGS_X, Self::BODY_LENGTH + Self::LEGS_Y),
            USegment::MoveTo(0.0, Self::BODY_LENGTH),
            USegment::LineTo(Self::LEGS_X, Self::BODY_LENGTH + Self::LEGS_Y),
        ];
        let ug = self.fashion.apply(ug);
        ug.translated(start_x, self.thickness())
            .draw(&UShape::Ellipse(UEllipse::new(
                Self::HEAD_DIAM,
                Self::HEAD_DIAM,
            )));
        if self.actor_business {
            Self::special_business(&ug.translated(
                start_x + Self::HEAD_DIAM / 2.0,
                self.thickness() + Self::HEAD_DIAM / 2.0,
            ));
        }
        ug.translated(center_x, Self::HEAD_DIAM + self.thickness())
            .with_backcolor(HColor::NONE)
            .draw(&UShape::path(body));
    }
}

pub(crate) struct ActorAwesome {
    fashion: Fashion,
}

impl ActorAwesome {
    const HEAD_DIAM: f64 = 32.0;
    const BODY_WIDTH: f64 = 54.0;
    const SHOULDER: f64 = 16.0;
    const COLLAR: f64 = 4.0;
    const RADIUS: f64 = 8.0;
    const BODY_HEIGHT: f64 = 28.0;

    fn thickness(&self) -> f64 {
        self.fashion.stroke.thickness
    }

    fn preferred_width(&self) -> f64 {
        Self::BODY_WIDTH + self.thickness() * 2.0
    }

    fn body() -> Vec<USegment> {
        let collar = Self::COLLAR;
        let shoulder = Self::SHOULDER;
        let radius = Self::RADIUS;
        let half_width = Self::BODY_WIDTH / 2.0;
        let height = Self::BODY_HEIGHT;
        let cubic = |ctrl1, ctrl2, end| USegment::CubicTo { ctrl1, ctrl2, end };
        vec![
            USegment::MoveTo(0.0, collar),
            cubic(
                (collar, collar),
                (half_width - shoulder - collar, collar),
                (half_width - shoulder, 0.0),
            ),
            cubic(
                (half_width - shoulder / 2.0, 0.0),
                (half_width, shoulder / 2.0),
                (half_width, shoulder),
            ),
            USegment::LineTo(half_width, height - radius),
            cubic(
                (half_width, height - radius / 2.0),
                (half_width - radius / 2.0, height),
                (half_width - radius, height),
            ),
            USegment::LineTo(-half_width + radius, height),
            cubic(
                (-half_width + radius / 2.0, height),
                (-half_width, height - radius / 2.0),
                (-half_width, height - radius),
            ),
            USegment::LineTo(-half_width, shoulder),
            cubic(
                (-half_width, shoulder / 2.0),
                (-half_width + shoulder / 2.0, 0.0),
                (-half_width + shoulder, 0.0),
            ),
            cubic(
                (-half_width + shoulder + collar, collar),
                (-collar, collar),
                (0.0, collar),
            ),
        ]
    }
}

impl TextBlock for ActorAwesome {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.preferred_width(),
            Self::HEAD_DIAM + Self::BODY_HEIGHT + self.thickness() * 2.0,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let center_x = self.preferred_width() / 2.0;
        let ug = self.fashion.apply(ug);
        ug.translated(center_x - Self::HEAD_DIAM / 2.0, self.thickness())
            .draw(&UShape::Ellipse(UEllipse::new(
                Self::HEAD_DIAM,
                Self::HEAD_DIAM,
            )));
        ug.translated(center_x, Self::HEAD_DIAM + self.thickness())
            .draw(&UShape::path(Self::body()));
    }
}

pub(crate) struct ActorHollow {
    fashion: Fashion,
}

impl ActorHollow {
    const HEAD_DIAM: f64 = 9.0;
    const BODY_WIDTH: f64 = 25.0;
    const BODY_HEIGHT: f64 = 21.0;
    const NECK_HEIGHT: f64 = 2.0;
    const ARM_THICKNESS: f64 = 5.0;
    const BODY_THICKNESS: f64 = 6.0;
    const LEG_THICKNESS: f64 = 6.0;

    fn thickness(&self) -> f64 {
        self.fashion.stroke.thickness
    }

    fn preferred_width(&self) -> f64 {
        Self::BODY_WIDTH + self.thickness() * 2.0
    }

    fn body() -> Vec<USegment> {
        let half_width = Self::BODY_WIDTH / 2.0;
        let height = Self::BODY_HEIGHT;
        let arm = Self::ARM_THICKNESS;
        let half_body = Self::BODY_THICKNESS / 2.0;
        let leg_diagonal = Self::LEG_THICKNESS * 2.0_f64.sqrt();
        let crotch_top = height - (Self::BODY_WIDTH + leg_diagonal - Self::BODY_THICKNESS) / 2.0;
        vec![
            USegment::MoveTo(-half_width, 0.0),
            USegment::LineTo(-half_width, arm),
            USegment::LineTo(-half_body, arm),
            USegment::LineTo(-half_body, crotch_top),
            USegment::LineTo(-half_width, height - leg_diagonal / 2.0),
            USegment::LineTo(-(half_width - leg_diagonal / 2.0), height),
            USegment::LineTo(0.0, height - (half_width - leg_diagonal / 2.0)),
            USegment::LineTo(half_width - leg_diagonal / 2.0, height),
            USegment::LineTo(half_width, height - leg_diagonal / 2.0),
            USegment::LineTo(half_body, crotch_top),
            USegment::LineTo(half_body, arm),
            USegment::LineTo(half_width, arm),
            USegment::LineTo(half_width, 0.0),
            USegment::LineTo(-half_width, 0.0),
        ]
    }
}

impl TextBlock for ActorHollow {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.preferred_width(),
            Self::HEAD_DIAM + Self::NECK_HEIGHT + Self::BODY_HEIGHT + self.thickness() * 2.0,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let center_x = self.preferred_width() / 2.0;
        let ug = self.fashion.apply(ug);
        ug.translated(center_x - Self::HEAD_DIAM / 2.0, self.thickness())
            .draw(&UShape::Ellipse(UEllipse::new(
                Self::HEAD_DIAM,
                Self::HEAD_DIAM,
            )));
        ug.translated(
            center_x,
            Self::HEAD_DIAM + self.thickness() + Self::NECK_HEIGHT,
        )
        .draw(&UShape::path(Self::body()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::debug::StringBounderDebug;
    use crate::klimt::ugraphic::UStroke;

    fn dimension(style: ActorStyle) -> XDimension2D {
        let fashion =
            Fashion::new(HColor::WHITE, HColor::BLACK).with_stroke(UStroke::with_thickness(0.5));
        style
            .text_block(fashion)
            .calculate_dimension(&StringBounderDebug)
    }

    #[test]
    fn each_style_has_its_size_plus_the_stroke() {
        assert_eq!(
            dimension(ActorStyle::Stickman),
            XDimension2D::new(27.0, 60.0)
        );
        assert_eq!(
            dimension(ActorStyle::Awesome),
            XDimension2D::new(55.0, 61.0)
        );
        assert_eq!(dimension(ActorStyle::Hollow), XDimension2D::new(26.0, 33.0));
    }

    #[test]
    fn unknown_styles_are_stickmen() {
        assert_eq!(ActorStyle::named("AWESOME"), ActorStyle::Awesome);
        assert_eq!(ActorStyle::named("hollow"), ActorStyle::Hollow);
        assert_eq!(ActorStyle::named("business"), ActorStyle::Stickman);
    }
}

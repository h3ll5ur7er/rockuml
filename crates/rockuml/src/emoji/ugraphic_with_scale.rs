use super::ColorResolver;
use crate::color::HColor;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};

/// A surface for an SVG picture: where it is drawn, with the transform its groups and shapes add up to.
#[derive(Clone)]
pub(crate) struct UGraphicWithScale<'a> {
    ug: UGraphic,
    color_resolver: &'a ColorResolver,
    at: XAffineTransform,
    /// The rotation in degrees, which arcs turn by.
    angle: f64,
    scale: f64,
}

impl<'a> UGraphicWithScale<'a> {
    pub(crate) fn new(ug: &UGraphic, color_resolver: &'a ColorResolver, scale: f64) -> Self {
        let color = color_resolver.default_color();
        Self {
            ug: ug.with_color(color.clone()).with_backcolor(color),
            color_resolver,
            at: XAffineTransform::scale_instance(scale, scale),
            angle: 0.0,
            scale,
        }
    }

    pub(crate) fn ug(&self) -> &UGraphic {
        &self.ug
    }

    #[must_use]
    pub(crate) fn with_color(&self, color: HColor) -> Self {
        Self {
            ug: self.ug.with_color(color),
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_backcolor(&self, color: HColor) -> Self {
        Self {
            ug: self.ug.with_backcolor(color),
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_stroke(&self, stroke: UStroke) -> Self {
        Self {
            ug: self.ug.with_stroke(stroke),
            ..self.clone()
        }
    }

    pub(crate) fn true_color(&self, code: &str) -> HColor {
        self.color_resolver.true_color(code)
    }

    pub(crate) fn default_color(&self) -> HColor {
        self.color_resolver.default_color()
    }

    /// PlantUML replaces the initial scale by this one, rather than multiplying them.
    #[must_use]
    pub(crate) fn apply_scale(&self, change_x: f64, change_y: f64) -> Self {
        let mut at = self.at;
        at.scale(change_x, change_y);
        Self {
            at,
            scale: change_x,
            ..self.clone()
        }
    }

    /// Rotates by `delta_angle` degrees around `(x, y)`.
    #[must_use]
    pub(crate) fn apply_rotate(&self, delta_angle: f64, x: f64, y: f64) -> Self {
        let mut at = self.at;
        at.rotate(delta_angle * std::f64::consts::PI / 180.0, x, y);
        Self {
            at,
            angle: self.angle + delta_angle,
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn apply_translate(&self, x: f64, y: f64) -> Self {
        let mut at = self.at;
        at.translate(x, y);
        Self { at, ..self.clone() }
    }

    #[must_use]
    pub(crate) fn apply_matrix(&self, matrix: &XAffineTransform) -> Self {
        let mut at = self.at;
        at.concatenate(matrix);
        Self { at, ..self.clone() }
    }

    pub(crate) fn affine_transform(&self) -> &XAffineTransform {
        &self.at
    }

    pub(crate) fn angle(&self) -> f64 {
        self.angle
    }

    pub(crate) fn initial_scale(&self) -> f64 {
        self.scale
    }

    pub(crate) fn draw(&self, shape: &UShape) {
        self.ug.draw(shape);
    }
}

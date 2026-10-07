//! PlantUML's own affine transform (`klimt.awt.XAffineTransform`), which differs from Java AWT's in how it scales.

/// The matrix `[[m00, m01, m02], [m10, m11, m12]]` mapping `(x, y)` to `(m00·x + m01·y + m02, m10·x + m11·y + m12)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct XAffineTransform {
    m00: f64,
    m10: f64,
    m01: f64,
    m11: f64,
    m02: f64,
    m12: f64,
}

impl XAffineTransform {
    /// The values in SVG's `matrix(a b c d e f)` order.
    pub(crate) const fn new(m00: f64, m10: f64, m01: f64, m11: f64, m02: f64, m12: f64) -> Self {
        Self {
            m00,
            m10,
            m01,
            m11,
            m02,
            m12,
        }
    }

    pub(crate) const fn scale_instance(sx: f64, sy: f64) -> Self {
        Self::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    pub(crate) fn scale_x(&self) -> f64 {
        self.m00
    }

    pub(crate) fn scale_y(&self) -> f64 {
        self.m11
    }

    pub(crate) fn translate_x(&self) -> f64 {
        self.m02
    }

    pub(crate) fn translate_y(&self) -> f64 {
        self.m12
    }

    /// Scales the result of the transform, translation included, unlike AWT which scales what it transforms.
    pub(crate) fn scale(&mut self, sx: f64, sy: f64) {
        self.m00 *= sx;
        self.m01 *= sx;
        self.m02 *= sx;
        self.m10 *= sy;
        self.m11 *= sy;
        self.m12 *= sy;
    }

    pub(crate) fn transform(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            self.m00 * x + self.m01 * y + self.m02,
            self.m10 * x + self.m11 * y + self.m12,
        )
    }

    /// `other` is applied first, then this transform.
    pub(crate) fn concatenate(&mut self, other: &Self) {
        *self = Self {
            m00: self.m00 * other.m00 + self.m01 * other.m10,
            m01: self.m00 * other.m01 + self.m01 * other.m11,
            m02: self.m00 * other.m02 + self.m01 * other.m12 + self.m02,
            m10: self.m10 * other.m00 + self.m11 * other.m10,
            m11: self.m10 * other.m01 + self.m11 * other.m11,
            m12: self.m10 * other.m02 + self.m11 * other.m12 + self.m12,
        };
    }

    pub(crate) fn translate(&mut self, tx: f64, ty: f64) {
        self.m02 += self.m00 * tx + self.m01 * ty;
        self.m12 += self.m10 * tx + self.m11 * ty;
    }

    /// Rotates by `theta` radians around `(anchor_x, anchor_y)`.
    pub(crate) fn rotate(&mut self, theta: f64, anchor_x: f64, anchor_y: f64) {
        self.translate(anchor_x, anchor_y);
        let (sin, cos) = theta.sin_cos();
        *self = Self {
            m00: self.m00 * cos + self.m01 * sin,
            m01: self.m00 * -sin + self.m01 * cos,
            m10: self.m10 * cos + self.m11 * sin,
            m11: self.m10 * -sin + self.m11 * cos,
            ..*self
        };
        self.translate(-anchor_x, -anchor_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_also_scales_the_translation() {
        let mut transform = XAffineTransform::scale_instance(2.0, 2.0);
        transform.translate(1.0, 3.0);
        assert_eq!(transform.transform((1.0, 1.0)), (4.0, 8.0));
        transform.scale(0.5, 0.5);
        assert_eq!(transform.transform((1.0, 1.0)), (2.0, 4.0));
    }

    #[test]
    fn a_concatenated_transform_applies_first() {
        let mut transform = XAffineTransform::scale_instance(2.0, 3.0);
        transform.concatenate(&XAffineTransform::new(1.0, 0.0, 0.0, 1.0, 5.0, 7.0));
        assert_eq!(transform.transform((1.0, 1.0)), (12.0, 24.0));
    }

    #[test]
    fn rotation_turns_around_the_anchor() {
        let mut transform = XAffineTransform::scale_instance(1.0, 1.0);
        transform.rotate(std::f64::consts::FRAC_PI_2, 1.0, 1.0);
        let close = |(x, y): (f64, f64), (ex, ey): (f64, f64)| {
            assert!((x - ex).abs() < 1e-12 && (y - ey).abs() < 1e-12, "{x}, {y}");
        };
        close(transform.transform((2.0, 1.0)), (1.0, 2.0));
        close(transform.transform((1.0, 1.0)), (1.0, 1.0));
    }
}

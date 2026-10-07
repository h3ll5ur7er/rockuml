//! Java's arithmetic as Smetana relies on it: `Macro.ROUND`/`POINTS`/`hypot`, `Math.min`/`max`, and the
//! transcendental functions.
//!
//! Java evaluates `double` arithmetic in strict IEEE 754 without fused multiply-add, like Rust's `f64` operators,
//! so ported expressions keep their operand order and never use `mul_add`. `(int) d` truncates towards zero,
//! saturates and maps NaN to 0, exactly as Rust's `d as i32`.
//!
//! The transcendental functions are the part Rust's `std` does not reproduce (it calls the platform's C library, so
//! results also differ between Windows, Linux and wasm). Measured on 200,000 samples against Java 21 on x86-64:
//!
//! | Java | here | differs from Java in |
//! |---|---|---|
//! | `Math.atan2` (fdlibm) | `libm::atan2` (an fdlibm port) | none |
//! | `Math.sin`, `Math.cos` (Intel LIBM intrinsics) | [`sin`], [`cos`]: correctly rounded | about 0.1%, by 1 ulp |
//! | `Math.pow` (Intel LIBM intrinsic) | [`pow`]: correctly rounded | about 0.05%, by 1 ulp |
//!
//! The intrinsics are correctly rounded nearly always, so correct rounding is the closest portable match; fdlibm's
//! `sin`/`cos` (and Rust's `std`) differ in about 3% of cases, its `pow` in 12%. Bit-exact results would need a
//! port of the JVM's assembly stubs. Smetana's layouts call `sin`/`cos` in `poly_init` on constant angles, which
//! match Java exactly (tested below), and in pathplan's `solve3`, with `pow`, only to locate spline/line
//! intersections.

#![allow(non_snake_case)]

/// `Macro.ROUND`: rounds half away from zero through `int`, which is not `f64::round` for values beyond `i32`.
pub(crate) fn ROUND(f: f64) -> i32 {
    if f >= 0.0 {
        (f + 0.5) as i32
    } else {
        (f - 0.5) as i32
    }
}

pub(crate) const POINTS_PER_INCH: i32 = 72;

/// `Macro.POINTS`: inches to whole points.
pub(crate) fn POINTS(a_inches: f64) -> i32 {
    ROUND(a_inches * f64::from(POINTS_PER_INCH))
}

pub(crate) fn INCH2PS(a_inches: f64) -> f64 {
    a_inches * f64::from(POINTS_PER_INCH)
}

pub(crate) fn PS2INCH(a_points: f64) -> f64 {
    a_points / f64::from(POINTS_PER_INCH)
}

/// `Macro.hypot`, Graphviz's own formula. It is not `Math.hypot` and rounds differently from `f64::hypot`.
pub(crate) fn hypot(x: f64, y: f64) -> f64 {
    let x = x.abs();
    let y = y.abs();
    let t = min(x, y);
    let x = max(x, y);
    let t = t / x;
    x * (1.0 + t * t).sqrt()
}

/// `Math.min`: NaN wins, and -0.0 is smaller than 0.0 (`f64::min` returns the non-NaN operand).
pub(crate) fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && b.is_sign_negative() {
        return b;
    }
    if a <= b { a } else { b }
}

/// `Math.max`: NaN wins, and 0.0 is greater than -0.0.
pub(crate) fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && a.is_sign_negative() {
        return b;
    }
    if a >= b { a } else { b }
}

/// `Math.sin`, correctly rounded for finite `|x| < 2^30`.
pub(crate) fn sin(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() || x.abs() >= dd::MAX_REDUCIBLE {
        return libm::sin(x);
    }
    let (r, quadrant) = dd::reduce(x);
    match quadrant {
        0 => dd::sin_series(r).round(),
        1 => dd::cos_series(r).round(),
        2 => -dd::sin_series(r).round(),
        _ => -dd::cos_series(r).round(),
    }
}

/// `Math.cos`, correctly rounded for finite `|x| < 2^30`.
pub(crate) fn cos(x: f64) -> f64 {
    if !x.is_finite() || x.abs() >= dd::MAX_REDUCIBLE {
        return libm::cos(x);
    }
    let (r, quadrant) = dd::reduce(x);
    match quadrant {
        0 => dd::cos_series(r).round(),
        1 => -dd::sin_series(r).round(),
        2 => -dd::cos_series(r).round(),
        _ => dd::sin_series(r).round(),
    }
}

/// `Math.atan2`, which is fdlibm's in Java too.
pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// `Math.pow`, correctly rounded (`exp(y·ln x)` in double-double) for positive normal `x`.
pub(crate) fn pow(x: f64, y: f64) -> f64 {
    if !(x > 0.0 && x.is_finite() && y.is_finite()) || x.is_subnormal() {
        return libm::pow(x, y);
    }
    let r = dd::exp(dd::ln(x).mul_f64(y));
    if r.is_finite() && r != 0.0 {
        r
    } else {
        libm::pow(x, y)
    }
}

/// Double-double arithmetic (a value is `hi + lo` with `|lo| <= ulp(hi) / 2`), precise to about 2^-104, which makes
/// the functions above correctly rounded except in astronomically rare cases. It is built on `libm::fma` so that
/// it gives the same results on every platform.
mod dd {
    #[derive(Clone, Copy)]
    pub(super) struct Dd(f64, f64);

    fn two_sum(a: f64, b: f64) -> Dd {
        let s = a + b;
        let bb = s - a;
        Dd(s, (a - (s - bb)) + (b - bb))
    }

    fn quick_two_sum(a: f64, b: f64) -> Dd {
        let s = a + b;
        Dd(s, b - (s - a))
    }

    /// The exact product. The fused multiply-add is an error-free transformation here, not a step of ported code.
    fn two_prod(a: f64, b: f64) -> Dd {
        let p = a * b;
        Dd(p, libm::fma(a, b, -p))
    }

    impl Dd {
        fn add(self, o: Dd) -> Dd {
            let s = two_sum(self.0, o.0);
            let t = two_sum(self.1, o.1);
            let s = quick_two_sum(s.0, s.1 + t.0);
            quick_two_sum(s.0, s.1 + t.1)
        }

        fn mul(self, o: Dd) -> Dd {
            let p = two_prod(self.0, o.0);
            quick_two_sum(p.0, p.1 + (self.0 * o.1 + self.1 * o.0))
        }

        pub(super) fn mul_f64(self, b: f64) -> Dd {
            let p = two_prod(self.0, b);
            quick_two_sum(p.0, p.1 + self.1 * b)
        }

        fn div(self, o: Dd) -> Dd {
            let q1 = self.0 / o.0;
            let r = self.add(o.mul_f64(-q1));
            let q2 = r.0 / o.0;
            let r = r.add(o.mul_f64(-q2));
            let q3 = r.0 / o.0;
            quick_two_sum(q1, q2).add(Dd(q3, 0.0))
        }

        /// The nearest double.
        pub(super) fn round(self) -> f64 {
            self.0 + self.1
        }
    }

    const LN2: Dd = Dd(std::f64::consts::LN_2, 2.319_046_813_846_299_6e-17);

    /// π/2 as the sum of three doubles.
    const PIO2: [f64; 3] = [
        std::f64::consts::FRAC_PI_2,
        6.123_233_995_736_766e-17,
        -1.497_384_904_859_169_8e-33,
    ];

    /// Beyond this, three doubles of π/2 no longer reduce arguments precisely enough.
    pub(super) const MAX_REDUCIBLE: f64 = 1_073_741_824.0;

    /// `x = k·π/2 + r` with `|r| <= π/4`; returns `r` and `k mod 4`.
    pub(super) fn reduce(x: f64) -> (Dd, i64) {
        let k = (x / PIO2[0]).round();
        let p1 = two_prod(k, PIO2[0]);
        let p2 = two_prod(k, PIO2[1]);
        let r = two_sum(x, -p1.0)
            .add(Dd(-p1.1, 0.0))
            .add(Dd(-p2.0, -p2.1))
            .add(Dd(-k * PIO2[2], 0.0));
        (r, (k as i64).rem_euclid(4))
    }

    /// `sin r` by its Taylor series, for `|r| <= π/4`.
    pub(super) fn sin_series(r: Dd) -> Dd {
        let r2 = r.mul(r);
        let mut term = r;
        let mut sum = r;
        for i in 1..30 {
            let i = f64::from(i);
            term = term.mul(r2).div(Dd(-(2.0 * i) * (2.0 * i + 1.0), 0.0));
            sum = sum.add(term);
        }
        sum
    }

    /// `cos r` by its Taylor series, for `|r| <= π/4`.
    pub(super) fn cos_series(r: Dd) -> Dd {
        let r2 = r.mul(r);
        let mut term = Dd(1.0, 0.0);
        let mut sum = term;
        for i in 1..30 {
            let i = f64::from(i);
            term = term.mul(r2).div(Dd(-(2.0 * i - 1.0) * (2.0 * i), 0.0));
            sum = sum.add(term);
        }
        sum
    }

    /// Natural logarithm of a positive normal `x`: `e·ln 2 + 2·atanh((m - 1) / (m + 1))` for `x = m·2^e`.
    pub(super) fn ln(x: f64) -> Dd {
        let bits = x.to_bits();
        let mut e = ((bits >> 52) & 0x7ff) as i32 - 1023;
        let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000);
        if m > std::f64::consts::SQRT_2 {
            m /= 2.0;
            e += 1;
        }
        let s = two_sum(m, -1.0).div(two_sum(m, 1.0));
        let s2 = s.mul(s);
        let mut term = s;
        let mut sum = s;
        let mut k = 3.0;
        while k < 80.0 {
            term = term.mul(s2);
            sum = sum.add(term.div(Dd(k, 0.0)));
            k += 2.0;
        }
        sum.mul_f64(2.0).add(LN2.mul_f64(f64::from(e)))
    }

    /// `e^t`, rounded to the nearest double: `2^k · e^r` with `|r| <= ln 2 / 2`.
    pub(super) fn exp(t: Dd) -> f64 {
        let k = (t.0 / LN2.0).round();
        let r = t.add(LN2.mul_f64(-k));
        let mut term = Dd(1.0, 0.0);
        let mut sum = Dd(1.0, 0.0);
        for i in 1..40 {
            term = term.mul(r).div(Dd(f64::from(i), 0.0));
            sum = sum.add(term);
        }
        libm::scalbn(sum.round(), k as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(x: f64) -> u64 {
        x.to_bits()
    }

    #[test]
    fn round_is_half_away_from_zero_through_int() {
        assert_eq!(ROUND(2.5), 3);
        assert_eq!(ROUND(-2.5), -3);
        assert_eq!(ROUND(-0.4), 0);
        assert_eq!(ROUND(1e20), i32::MAX);
        assert_eq!(ROUND(f64::NAN), 0);
        assert_eq!(POINTS(0.5), 36);
    }

    #[test]
    fn min_and_max_follow_java() {
        assert!(min(f64::NAN, 1.0).is_nan());
        assert!(min(1.0, f64::NAN).is_nan());
        assert!(max(1.0, f64::NAN).is_nan());
        assert!(min(0.0, -0.0).is_sign_negative());
        assert!(min(-0.0, 0.0).is_sign_negative());
        assert!(max(-0.0, 0.0).is_sign_positive());
        assert!(max(0.0, -0.0).is_sign_positive());
    }

    #[test]
    fn hypot_is_graphviz_formula() {
        assert_eq!(hypot(3.0, -4.0), 5.0);
        assert_eq!(bits(hypot(0.5, 0.5)), bits(0.5 * 2f64.sqrt()));
    }

    /// Every sin/cos/atan2 that `poly_init` evaluates for a box, with the results Java 21 printed.
    #[test]
    fn poly_init_trig_matches_java() {
        type Unary = fn(f64) -> f64;
        let cases: [(Unary, u64, u64); 9] = [
            (sin, 0x3fe921fb54442d18, 0x3fe6a09e667f3bcc),
            (cos, 0x3fe921fb54442d18, 0x3fe6a09e667f3bcd),
            (sin, 0xbfe921fb54442d18, 0xbfe6a09e667f3bcc),
            (cos, 0xbfe921fb54442d18, 0x3fe6a09e667f3bcd),
            (sin, 0x3ff921fb54442d18, 0x3ff0000000000000),
            (cos, 0x3ff921fb54442d18, 0x3c91a62633145c07),
            (sin, 0x3fe921fb54442d17, 0x3fe6a09e667f3bcc),
            (cos, 0x3fe921fb54442d17, 0x3fe6a09e667f3bcd),
            (cos, 0x3fe921fb54442d18, 0x3fe6a09e667f3bcd),
        ];
        for (f, x, want) in cases {
            assert_eq!(bits(f(f64::from_bits(x))), want, "input {x:x}");
        }
        let y = f64::from_bits(0x3fd6a09e667f3bcc);
        let x = f64::from_bits(0x3fd6a09e667f3bce);
        assert_eq!(bits(atan2(y, x)), 0x3fe921fb54442d17);
    }

    /// Samples of `Math.atan2` and `Math.pow(x, 1.0 / 3.0)` from Java 21 on x86-64.
    #[test]
    fn atan2_and_pow_match_java_samples() {
        // (y, x, Math.atan2(y, x))
        let atan2_cases = [
            (
                0x4072c6e147ae147b_u64,
                0xc08b0e6666666666_u64,
                0x400675f4b0c05e33_u64,
            ),
            (0xbff0000000000000, 0x3ff0000000000000, 0xbfe921fb54442d18),
            (0x3ff0000000000000, 0x0000000000000000, 0x3ff921fb54442d18),
            (0xbf46f0068db8bac7, 0x400a000000000000, 0xbf2c3b1bbacb795c),
        ];
        for (y, x, want) in atan2_cases {
            assert_eq!(bits(atan2(f64::from_bits(y), f64::from_bits(x))), want);
        }
        // (x, Math.pow(x, 1.0 / 3.0))
        let pow_cases = [
            (0x412e848000000000_u64, 0x4058fffffffffffe_u64),
            (0x4000000000000000, 0x3ff428a2f98d728b),
            (0x3fe0000000000000, 0x3fe965fea53d6e3d),
            (0x405edd2f1a9fbe77, 0x4013ead4f488ee35),
            (0x3f489374bc6a7efa, 0x3fb742573bf48200),
        ];
        for (x, want) in pow_cases {
            assert_eq!(
                bits(pow(f64::from_bits(x), 1.0 / 3.0)),
                want,
                "pow({x:x}, 1/3)"
            );
        }
        assert_eq!(pow(0.0, 1.0 / 3.0), 0.0);
    }
}

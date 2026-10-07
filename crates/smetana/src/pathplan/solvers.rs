//! `solvers.c`: real roots of polynomials up to degree three.
//!
//! Smetana calls Java's `Math.atan2`, `Math.cos` and `Math.pow`. `atan2` is fdlibm there, which the `libm` crate
//! reproduces exactly. `cos` and `pow` are `HotSpot` intrinsics that neither `libm` nor the platform's C library
//! reproduces in the last bit; `libm` at least gives the same result on every platform. The roots only feed
//! comparisons in route.c, so a last-bit difference changes a route only in the rarest of cases.

const EPS: f64 = 1E-7;

fn AEQ0(x: f64) -> bool {
    x < EPS && x > -EPS
}

/// Stores the real roots of `coeff[3]·x³ + coeff[2]·x² + coeff[1]·x + coeff[0]` in `roots` and returns how many
/// there are, or 4 if every `x` is a root.
pub fn solve3(coeff: &[f64; 4], roots: &mut [f64; 3]) -> usize {
    let [d, c, b, a] = *coeff;
    if AEQ0(a) {
        return solve2(coeff, roots);
    }
    let b_over_3a = b / (3.0 * a);
    let c_over_a = c / a;
    let d_over_a = d / a;

    let mut p = b_over_3a * b_over_3a;
    let q = 2.0 * b_over_3a * p - b_over_3a * c_over_a + d_over_a;
    p = c_over_a / 3.0 - p;
    let disc = q * q + 4.0 * p * p * p;

    let rootn = if disc < 0.0 {
        let r = 0.5 * (-disc + q * q).sqrt();
        let theta = libm::atan2((-disc).sqrt(), -q);
        let temp = 2.0 * cbrt(r);
        roots[0] = temp * libm::cos(theta / 3.0);
        roots[1] = temp * libm::cos((theta + std::f64::consts::PI + std::f64::consts::PI) / 3.0);
        roots[2] = temp * libm::cos((theta - std::f64::consts::PI - std::f64::consts::PI) / 3.0);
        3
    } else {
        let alpha = 0.5 * (disc.sqrt() - q);
        let beta = -q - alpha;
        roots[0] = cbrt(alpha) + cbrt(beta);
        if disc > 0.0 {
            1
        } else {
            roots[1] = -0.5 * roots[0];
            roots[2] = roots[1];
            3
        }
    };

    for root in &mut roots[..rootn] {
        *root -= b_over_3a;
    }
    rootn
}

fn solve2(coeff: &[f64; 4], roots: &mut [f64; 3]) -> usize {
    let [c, b, a, _] = *coeff;
    if AEQ0(a) {
        return solve1(coeff, roots);
    }
    let b_over_2a = b / (2.0 * a);
    let c_over_a = c / a;

    let disc = b_over_2a * b_over_2a - c_over_a;
    if disc < 0.0 {
        0
    } else if disc == 0.0 {
        roots[0] = -b_over_2a;
        1
    } else {
        roots[0] = -b_over_2a + disc.sqrt();
        roots[1] = -2.0 * b_over_2a - roots[0];
        2
    }
}

fn solve1(coeff: &[f64; 4], roots: &mut [f64; 3]) -> usize {
    let [b, a, _, _] = *coeff;
    if AEQ0(a) {
        return if AEQ0(b) { 4 } else { 0 };
    }
    roots[0] = -b / a;
    1
}

/// Graphviz's `cbrt` macro: `pow(x, 1/3)` with the sign of `x`.
fn cbrt(x: f64) -> f64 {
    if x < 0.0 {
        -libm::pow(-x, 1.0 / 3.0)
    } else {
        libm::pow(x, 1.0 / 3.0)
    }
}

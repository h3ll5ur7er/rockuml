//! The part of `utils.c` that spline routing uses: Bézier evaluation, the closest point of a spline, and
//! `late_double` for `arrowsize`.

use crate::cgraph::Agobj;
use crate::cgraph::attr::agxget;
use crate::common::geom::DIST2;
use crate::core::Globals;
use crate::core::ids::{SplinesId, SymId};
use crate::core::jutils::atof;
use crate::h::pointf;

const W_DEGREE: usize = 5;

/// `Bezier`: the point at `t` of the Bézier curve with control points `V[0..=degree]`, filling in the control
/// points of the two halves split there if `Left` or `Right` is given.
pub fn Bezier(
    V: &[pointf],
    degree: usize,
    t: f64,
    Left: Option<&mut [pointf]>,
    Right: Option<&mut [pointf]>,
) -> pointf {
    let mut Vtemp = [[pointf::default(); W_DEGREE + 1]; W_DEGREE + 1];
    Vtemp[0][..=degree].copy_from_slice(&V[..=degree]);
    for i in 1..=degree {
        for j in 0..=degree - i {
            Vtemp[i][j].x = (1.0 - t) * Vtemp[i - 1][j].x + t * Vtemp[i - 1][j + 1].x;
            Vtemp[i][j].y = (1.0 - t) * Vtemp[i - 1][j].y + t * Vtemp[i - 1][j + 1].y;
        }
    }
    if let Some(Left) = Left {
        for j in 0..=degree {
            Left[j] = Vtemp[j][0];
        }
    }
    if let Some(Right) = Right {
        for j in 0..=degree {
            Right[j] = Vtemp[degree - j][j];
        }
    }
    Vtemp[degree][0]
}

/// `dotneato_closest`: the point of spline `spl` nearest to `pt`, found by bisecting the Bézier piece whose
/// control point is nearest.
pub fn dotneato_closest(zz: &Globals, spl: SplinesId, pt: pointf) -> pointf {
    let spl = zz.splines[spl];
    let list = spl.list.expect("spline without beziers");
    let mut besti = -1;
    let mut bestj = -1;
    let mut bestdist2 = 1e+38;
    for i in 0..spl.size {
        let bz = zz.beziers.get(list, i);
        let points = bz.list.expect("bezier without points");
        for j in 0..bz.size {
            let b = zz.pointfs.get(points, j);
            let d2 = DIST2(b, pt);
            if bestj == -1 || d2 < bestdist2 {
                besti = i;
                bestj = j;
                bestdist2 = d2;
            }
        }
    }
    let bz = zz.beziers.get(list, besti);
    let points = bz.list.expect("bezier without points");
    if bestj == bz.size - 1 {
        bestj -= 1;
    }
    let j = 3 * (bestj / 3);
    let c: Vec<pointf> = (0..4).map(|k| zz.pointfs.get(points, j + k)).collect();
    let dlow2 = DIST2(c[0], pt);
    let dhigh2 = DIST2(c[3], pt);
    let pt2 = Bezier(&c, 3, 0.5, None, None);
    // Smetana stops the bisection after its first step or fails.
    if (dlow2 - dhigh2).abs() < 1.0 {
        return pt2;
    }
    unimplemented!("6apa9aoby9j8a0eanbfhy5mn2: dotneato_closest bisection")
}

/// `late_double`: `obj`'s value of attribute `attr` as a number, `def` if it is unset or empty, at least `low`.
pub fn late_double(
    zz: &mut Globals,
    obj: impl Into<Agobj>,
    attr: Option<SymId>,
    def: f64,
    low: f64,
) -> f64 {
    let Some(attr) = attr else {
        return def;
    };
    let Some(p) = agxget(zz, obj, attr) else {
        return def;
    };
    let p = zz.agstr(p);
    if p.is_empty() {
        return def;
    }
    let rv = atof(p);
    if rv < low { low } else { rv }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> pointf {
        pointf { x, y }
    }

    #[test]
    fn bezier_splits_at_t() {
        let v = [p(0.0, 0.0), p(0.0, 3.0), p(3.0, 3.0), p(3.0, 0.0)];
        let mut left = [pointf::default(); 4];
        let mut right = [pointf::default(); 4];
        let mid = Bezier(&v, 3, 0.5, Some(&mut left), Some(&mut right));
        assert_eq!(mid, p(1.5, 2.25));
        assert_eq!(left, [p(0.0, 0.0), p(0.0, 1.5), p(0.75, 2.25), mid]);
        assert_eq!(right, [mid, p(2.25, 2.25), p(3.0, 1.5), p(3.0, 0.0)]);
    }
}

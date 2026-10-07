//! `util.c`: polyline helpers.

use super::Ppolyline_t;

/// Stores in `sline` the piecewise Bézier spline that draws the polyline `line` with straight segments: every
/// inner point three times, the end points twice.
///
/// # Panics
/// If `line` has fewer than two points.
pub fn make_polyline(line: &Ppolyline_t, sline: &mut Ppolyline_t) {
    let (first, rest) = line.ps.split_first().expect("a polyline has points");
    let (last, inner) = rest.split_last().expect("a polyline has two ends");
    sline.ps.clear();
    sline.ps.extend([*first, *first]);
    for p in inner {
        sline.ps.extend([*p, *p, *p]);
    }
    sline.ps.extend([*last, *last]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathplan::Ppoint_t;

    fn p(x: f64, y: f64) -> Ppoint_t {
        Ppoint_t { x, y }
    }

    #[test]
    fn repeats_inner_points_three_times_and_ends_twice() {
        let line = Ppolyline_t {
            ps: vec![p(0.0, 0.0), p(1.0, 2.0), p(3.0, 4.0), p(5.0, 6.0)],
        };
        let mut sline = Ppolyline_t {
            ps: vec![p(9.0, 9.0)],
        };
        make_polyline(&line, &mut sline);
        let expected = [(0, 2), (1, 3), (2, 3), (3, 2)]
            .iter()
            .flat_map(|&(i, n)| std::iter::repeat_n(line.ps[i], n))
            .collect::<Vec<_>>();
        assert_eq!(sline.ps, expected);
        assert_eq!(sline.ps.len(), 4 + 3 * (line.ps.len() - 2));
    }
}

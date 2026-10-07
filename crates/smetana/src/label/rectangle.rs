//! `rectangle.c`: the R-tree's integer rectangles.

use super::Rect_t;

/// `InitRect`.
pub(crate) fn InitRect(r: &mut Rect_t) {
    for i in 0..2 * 2 {
        r.boundary[i] = 0;
    }
}

/// `NullRect`: an inverted rectangle that [`CombineRect`] ignores.
pub(crate) fn NullRect() -> Rect_t {
    let mut r = Rect_t::default();
    r.boundary[0] = 1;
    r.boundary[2] = -1;
    for i in 1..2 {
        r.boundary[i] = 0;
        r.boundary[i + 2] = 0;
    }
    r
}

/// `RectArea`. Java's `int` arithmetic wraps, and a side of length 0 divides by zero, which throws in Java and
/// panics here.
pub(crate) fn RectArea(r: &Rect_t) -> i32 {
    if r.boundary[0] > r.boundary[2] {
        return 0;
    }
    let mut area: i32 = 1;
    let mut a: i32 = 1;
    for i in 0..2 {
        let b = r.boundary[i + 2].wrapping_sub(r.boundary[i]);
        a = a.wrapping_mul(b);
        assert!(a.wrapping_div(b) == area, "label: area too large for rtree");
        area = a;
    }
    area
}

/// `CombineRect`: the smallest rectangle containing both.
pub(crate) fn CombineRect(r: &Rect_t, rr: &Rect_t) -> Rect_t {
    if r.boundary[0] > r.boundary[2] {
        return *rr;
    }
    if rr.boundary[0] > rr.boundary[2] {
        return *r;
    }
    let mut new_ = Rect_t::default();
    for i in 0..2 {
        new_.boundary[i] = r.boundary[i].min(rr.boundary[i]);
        let j = i + 2;
        new_.boundary[j] = r.boundary[j].max(rr.boundary[j]);
    }
    new_
}

/// `Overlap`: whether the rectangles share a point (touching counts).
pub(crate) fn Overlap(r: &Rect_t, s: &Rect_t) -> bool {
    for i in 0..2 {
        let j = i + 2;
        if r.boundary[i] > s.boundary[j] || s.boundary[i] > r.boundary[j] {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(b: [i32; 4]) -> Rect_t {
        Rect_t { boundary: b }
    }

    #[test]
    fn area_wraps_like_java_int_and_rejects_overflow() {
        assert_eq!(RectArea(&rect([0, 0, 3, 4])), 12);
        assert_eq!(RectArea(&NullRect()), 0);
        let huge = rect([0, 0, 65_536, 65_536]);
        assert!(std::panic::catch_unwind(|| RectArea(&huge)).is_err());
    }

    #[test]
    #[should_panic(expected = "divide by zero")]
    fn flat_rectangles_divide_by_zero() {
        RectArea(&rect([0, 0, 0, 5]));
    }

    #[test]
    fn combine_ignores_null_rectangles() {
        let a = rect([1, 2, 3, 4]);
        assert_eq!(CombineRect(&NullRect(), &a), a);
        assert_eq!(CombineRect(&a, &NullRect()), a);
        assert_eq!(CombineRect(&a, &rect([-1, 3, 2, 9])), rect([-1, 2, 3, 9]));
        assert!(Overlap(&a, &rect([3, 4, 5, 5])));
        assert!(!Overlap(&a, &rect([4, 4, 5, 5])));
    }
}

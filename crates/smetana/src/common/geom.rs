//! `geom.c` and the macros of `geom.h`: point rotations by multiples of 90 degrees, distances and containment.

use crate::h::{boxf, pointf};

/// `BETWEEN(a, b, c)`: `a <= b <= c`.
pub(crate) fn BETWEEN(a: f64, b: f64, c: f64) -> bool {
    a <= b && b <= c
}

/// `INSIDE(p, b)`: whether `p` lies in `b`, borders included.
pub(crate) fn INSIDE(p: pointf, b: boxf) -> bool {
    BETWEEN(b.LL.x, p.x, b.UR.x) && BETWEEN(b.LL.y, p.y, b.UR.y)
}

/// `DIST2(p, q)`: the squared distance.
pub(crate) fn DIST2(p: pointf, q: pointf) -> f64 {
    let (dx, dy) = (p.x - q.x, p.y - q.y);
    dx * dx + dy * dy
}

/// `APPROXEQPT(p, q, tol)`.
pub(crate) fn APPROXEQPT(p: pointf, q: pointf, tol: f64) -> bool {
    DIST2(p, q) < tol * tol
}

/// `rotatepf`, rotation by an arbitrary angle, which Smetana does not implement.
fn rotatepf(_p: pointf, _cwrot: i32) -> pointf {
    unimplemented!("adzi0wztceimu4ni3aonznmq7: rotatepf")
}

/// `cwrotatepf`: `p` rotated clockwise by `cwrot` degrees.
pub(crate) fn cwrotatepf(p: pointf, cwrot: i32) -> pointf {
    let (x, y) = (p.x, p.y);
    match cwrot {
        0 => p,
        90 => pointf { x: y, y: -x },
        180 => pointf { x, y: -y },
        270 => pointf { x: y, y: x },
        _ if cwrot < 0 => ccwrotatepf(p, -cwrot),
        _ if cwrot > 360 => cwrotatepf(p, cwrot % 360),
        _ => rotatepf(p, cwrot),
    }
}

/// `ccwrotatepf`: `p` rotated counter-clockwise by `ccwrot` degrees.
pub(crate) fn ccwrotatepf(p: pointf, ccwrot: i32) -> pointf {
    let (x, y) = (p.x, p.y);
    match ccwrot {
        0 => p,
        90 => pointf { x: -y, y: x },
        180 => pointf { x, y: -y },
        270 => pointf { x: y, y: x },
        _ if ccwrot < 0 => cwrotatepf(p, -ccwrot),
        _ if ccwrot > 360 => ccwrotatepf(p, ccwrot % 360),
        _ => rotatepf(p, 360 - ccwrot),
    }
}

/// `ptToLine2`: the squared distance from `p` to the line through `a` and `b`.
pub(crate) fn ptToLine2(a: pointf, b: pointf, p: pointf) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let mut a2 = (p.y - a.y) * dx - (p.x - a.x) * dy;
    a2 *= a2;
    if a2 < 0.0000000001 {
        return 0.0;
    }
    a2 / (dx * dx + dy * dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_by_right_angles_are_graphviz_s() {
        let p = pointf { x: 1.0, y: 2.0 };
        assert_eq!(cwrotatepf(p, 90), pointf { x: 2.0, y: -1.0 });
        assert_eq!(ccwrotatepf(p, 90), pointf { x: -2.0, y: 1.0 });
        // Graphviz's 180 and 270 degree cases are reflections, not rotations.
        assert_eq!(cwrotatepf(p, 180), pointf { x: 1.0, y: -2.0 });
        assert_eq!(ccwrotatepf(p, 270), pointf { x: 2.0, y: 1.0 });
        assert_eq!(cwrotatepf(p, -90), ccwrotatepf(p, 90));
        assert_eq!(ccwrotatepf(p, 450), ccwrotatepf(p, 90));
    }

    #[test]
    fn distance_to_a_line_is_squared() {
        let a = pointf { x: 0.0, y: 0.0 };
        let b = pointf { x: 4.0, y: 0.0 };
        assert_eq!(ptToLine2(a, b, pointf { x: 1.0, y: 3.0 }), 9.0);
        assert_eq!(ptToLine2(a, b, pointf { x: 7.0, y: 0.0 }), 0.0);
    }
}

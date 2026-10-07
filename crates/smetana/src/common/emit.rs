//! The part of `emit.c` that layout uses: growing a bounding box around a Bézier segment.

use crate::common::geom::ptToLine2;
use crate::common::utils::Bezier;
use crate::h::{boxf, pointf};

/// `check_control_points`: whether the inner control points lie within 2 points of the chord.
fn check_control_points(cp: &[pointf; 4]) -> bool {
    let dis1 = ptToLine2(cp[0], cp[3], cp[1]);
    let dis2 = ptToLine2(cp[0], cp[3], cp[2]);
    dis1 < 2.0 * 2.0 && dis2 < 2.0 * 2.0
}

/// `update_bb_bz`: grows `bb` to hold the Bézier segment `cp`, splitting the segment until it is nearly straight.
pub fn update_bb_bz(bb: &mut boxf, cp: &[pointf; 4]) {
    let outside = cp
        .iter()
        .any(|p| p.x > bb.UR.x || p.x < bb.LL.x || p.y > bb.UR.y || p.y < bb.LL.y);
    if !outside {
        return;
    }
    if check_control_points(cp) {
        for p in cp {
            if p.x > bb.UR.x {
                bb.UR.x = p.x;
            } else if p.x < bb.LL.x {
                bb.LL.x = p.x;
            }
            if p.y > bb.UR.y {
                bb.UR.y = p.y;
            } else if p.y < bb.LL.y {
                bb.LL.y = p.y;
            }
        }
    } else {
        let mut left = [pointf::default(); 4];
        let mut right = [pointf::default(); 4];
        Bezier(cp, 3, 0.5, Some(&mut left), Some(&mut right));
        update_bb_bz(bb, &left);
        update_bb_bz(bb, &right);
    }
}

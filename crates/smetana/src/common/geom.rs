//! `common/geom.c`: point and box geometry.

use crate::h::pointf;

/// `cwrotatepf`: rotates `p` clockwise by a multiple of 90 degrees. Like Graphviz, 180 degrees only mirrors y.
pub fn cwrotatepf(mut p: pointf, cwrot: i32) -> pointf {
    let (x, y) = (p.x, p.y);
    match cwrot {
        0 => {}
        90 => {
            p.x = y;
            p.y = -x;
        }
        180 => {
            p.x = x;
            p.y = -y;
        }
        270 => {
            p.x = y;
            p.y = x;
        }
        _ => unimplemented!("cwrotatepf by {cwrot} degrees"),
    }
    p
}

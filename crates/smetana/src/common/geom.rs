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

/// `ccwrotatepf`: `p` rotated counter-clockwise by a multiple of 90 degrees.
pub fn ccwrotatepf(p: pointf, ccwrot: i32) -> pointf {
    let x = p.x;
    let y = p.y;
    match ccwrot {
        0 => p,
        90 => pointf { x: -y, y: x },
        180 => pointf { x, y: -y },
        270 => pointf { x: y, y: x },
        _ => unimplemented!("ccwrotatepf by {ccwrot} degrees"),
    }
}

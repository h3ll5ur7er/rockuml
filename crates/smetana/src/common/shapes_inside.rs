//! The half of `shapes.c` that edges use once nodes are sized: ports, inside tests and the port boxes of polygon
//! and record shapes. (`bind_shape` and the init functions are in `shapes.rs`.)

use crate::cgraph::obj::{agraphof, agroot};
use crate::common::geom::{BETWEEN, INSIDE, ccwrotatepf};
use crate::common::shapes::compassPort;

use crate::core::Globals;
use crate::core::consts::{BOTTOM, LEFT, RIGHT, TOP};
use crate::core::ids::{FieldId, NodeId};
use crate::core::jmath::{ROUND, hypot};
use crate::h::{SHAPE_INFO, boxf, inside_t, point, pointf, polygon_t, port, shape_functions};

/// A shape's `insidefn`: whether a point, relative to the node's centre, is inside the shape (or port box).
pub type InsideFn = fn(&mut Globals, &inside_t, pointf) -> bool;
/// A shape's `pboxfn`: the box an edge leaves through when its port demands one; returns the side mask.
pub type PboxFn = fn(&mut Globals, NodeId, &port, i32, &mut boxf, &mut i32) -> i32;

impl shape_functions {
    pub fn insidefn(self) -> InsideFn {
        match self {
            shape_functions::poly_fns => poly_inside,
            shape_functions::record_fns => record_inside,
        }
    }

    pub fn pboxfn(self) -> PboxFn {
        match self {
            shape_functions::poly_fns => poly_path,
            shape_functions::record_fns => record_path,
        }
    }
}

/// The point (0, 0), `O` in C.
const O: pointf = pointf { x: 0.0, y: 0.0 };

/// `same_side`: whether `p0` and `p1` lie on the same side of the line through `L0` and `L1`.
fn same_side(p0: pointf, p1: pointf, L0: pointf, L1: pointf) -> bool {
    let a = -(L1.y - L0.y);
    let b = L1.x - L0.x;
    let c = a * L0.x + b * L0.y;
    let s0 = a * p0.x + b * p0.y - c >= 0.0;
    let s1 = a * p1.x + b * p1.y - c >= 0.0;
    s0 == s1
}

fn polygon_of(zz: &Globals, n: NodeId) -> polygon_t {
    match zz.nd(n).shape_info {
        Some(SHAPE_INFO::Polygon(poly)) => zz.polygons[poly],
        other => panic!("node shape info {other:?} is not a polygon"),
    }
}

fn field_of(zz: &Globals, n: NodeId) -> FieldId {
    match zz.nd(n).shape_info {
        Some(SHAPE_INFO::Field(f)) => f,
        other => panic!("node shape info {other:?} is not a record"),
    }
}

fn has_html_label(zz: &Globals, n: NodeId) -> bool {
    zz.textlabels[zz.nd(n).label.expect("node without label")].html
}

/// `poly_inside`. It caches what it derives from the node's polygon until it is asked about another node, and
/// starts testing at the side it last stopped at.
pub fn poly_inside(zz: &mut Globals, inside_context: &inside_t, p: pointf) -> bool {
    let bp = inside_context.s_bp;
    let n = inside_context.s_n.expect("inside_context.s.n");
    let g = agraphof(zz, n);
    let mut P = ccwrotatepf(p, 90 * zz.gd(g).GD_rankdir());
    if let Some(bbox) = bp {
        return INSIDE(P, bbox);
    }
    if zz.lastn != Some(n) {
        let poly = polygon_of(zz, n);
        zz.vertex = poly.vertices;
        zz.sides = poly.sides;
        if poly.option & (1 << 11) != 0 {
            unimplemented!("18yw1scg4sol8bhyf1vedj9kn: polyBB");
        }
        let nd = *zz.nd(n);
        if zz.gd(g).GD_flip() {
            zz.ysize = nd.lw + nd.rw;
            zz.xsize = nd.ht;
        } else {
            zz.xsize = nd.lw + nd.rw;
            zz.ysize = nd.ht;
        }
        let n_width = f64::from(ROUND(nd.width * 72.0));
        let n_height = f64::from(ROUND(nd.height * 72.0));
        if zz.xsize == 0.0 {
            zz.xsize = 1.0;
        }
        if zz.ysize == 0.0 {
            zz.ysize = 1.0;
        }
        zz.scalex = n_width / zz.xsize;
        zz.scaley = n_height / zz.ysize;
        zz.box_URx = n_width / 2.0;
        zz.box_URy = n_height / 2.0;
        zz.outp = (poly.peripheries - 1) * zz.sides;
        if zz.outp < 0 {
            zz.outp = 0;
        }
        zz.lastn = Some(n);
    }
    P.x *= zz.scalex;
    P.y *= zz.scaley;
    if P.x.abs() > zz.box_URx || P.y.abs() > zz.box_URy {
        return false;
    }
    if zz.sides <= 2 {
        return hypot(P.x / zz.box_URx, P.y / zz.box_URy) < 1.0;
    }
    let vertices = zz.vertex.expect("polygon without vertices");
    let vertex = |zz: &Globals, i: i32| zz.pointfs.get(vertices, i + zz.outp);
    let mut i = zz.last % zz.sides;
    let mut i1 = (i + 1) % zz.sides;
    let Q = vertex(zz, i);
    let R = vertex(zz, i1);
    if !same_side(P, O, Q, R) {
        return false;
    }
    let s = same_side(P, Q, R, O);
    if s && same_side(P, R, O, Q) {
        return true;
    }
    for _j in 1..zz.sides {
        if s {
            i = i1;
            i1 = (i + 1) % zz.sides;
        } else {
            i1 = i;
            i = (i + zz.sides - 1) % zz.sides;
        }
        if !same_side(P, O, vertex(zz, i), vertex(zz, i1)) {
            zz.last = i;
            return false;
        }
    }
    zz.last = i;
    true
}

/// `poly_path`: polygons constrain no edge, unless they have HTML ports.
pub fn poly_path(
    zz: &mut Globals,
    n: NodeId,
    _p: &port,
    _side: i32,
    _rv: &mut boxf,
    _kptr: &mut i32,
) -> i32 {
    if has_html_label(zz, n) && zz.nd(n).has_port {
        unimplemented!("67g7bthntnw8syb6zd03ueg84: html_path");
    }
    0
}

/// `record_inside`.
pub fn record_inside(zz: &mut Globals, inside_context: &inside_t, p: pointf) -> bool {
    let n = inside_context.s_n.expect("inside_context.s.n");
    let p = ccwrotatepf(p, 90 * zz.gd(agraphof(zz, n)).GD_rankdir());
    let bbox = match inside_context.s_bp {
        Some(bp) => bp,
        None => zz.fields[field_of(zz, n)].b,
    };
    INSIDE(p, bbox)
}

/// `record_path`: for a port on a top-level field, the column of that field across the whole node height.
pub fn record_path(
    zz: &mut Globals,
    n: NodeId,
    prt: &port,
    side: i32,
    rv: &mut boxf,
    kptr: &mut i32,
) -> i32 {
    if !prt.defined {
        return 0;
    }
    let p = prt.p;
    let info = &zz.fields[field_of(zz, n)];
    let g = agraphof(zz, n);
    for i in 0..info.n_flds {
        if zz.gd(g).GD_flip() {
            unimplemented!("dm9w81fxfdqc5bhtaimpbisvl: record_path on a flipped node");
        }
        let fld = zz
            .field_lists
            .get(info.fld.expect("field without subfields"), i);
        let b = zz.fields[fld.expect("NULL subfield")].b;
        let ls = b.LL.x as i32;
        let rs = b.UR.x as i32;
        if BETWEEN(f64::from(ls), p.x, f64::from(rs)) {
            let nd = zz.nd(n);
            rv.LL.x = nd.coord.x + f64::from(ls);
            rv.LL.y = nd.coord.y - (nd.ht / 2.0);
            rv.UR.x = nd.coord.x + f64::from(rs);
            rv.UR.y = nd.coord.y + (nd.ht / 2.0);
            *kptr = 1;
            break;
        }
    }
    side
}

/// `cvtPt`: a point in the rank direction's coordinates, rounded.
fn cvtPt(p: pointf, rankdir: i32) -> point {
    if rankdir != 0 {
        unimplemented!("drh1t5heo8w8z199n0vydnon7: cvtPt for rankdir {rankdir}");
    }
    point {
        x: ROUND(p.x),
        y: ROUND(p.y),
    }
}

const side_port: [&str; 4] = ["s", "e", "n", "w"];

/// `closestSide`: the compass point, among the port's sides, closest to node `other`; `None` for the centre.
fn closestSide(zz: &Globals, n: NodeId, other: NodeId, oldport: &port) -> Option<&'static str> {
    let rkd = zz.gd(agroot(zz, n)).GD_rankdir();
    let pt = cvtPt(zz.nd(n).coord, rkd);
    let opt = cvtPt(zz.nd(other).coord, rkd);
    let sides = oldport.side;
    let mut rv = None;
    let mut mind = 0;
    if sides == 0 || sides == (TOP | BOTTOM | LEFT | RIGHT) {
        return rv;
    }
    let Some(b) = oldport.bp else {
        unimplemented!("ek9a7u2yx8w4r9x5k7somxuup: closestSide without a port box");
    };
    for (i, name) in side_port.iter().enumerate() {
        if sides & (1 << i) == 0 {
            continue;
        }
        let mut p = match i {
            0 => point {
                y: b.LL.y as i32,
                x: (b.LL.x + b.UR.x) as i32 / 2,
            },
            1 => point {
                x: b.UR.x as i32,
                y: (b.LL.y + b.UR.y) as i32 / 2,
            },
            2 => point {
                y: b.UR.y as i32,
                x: (b.LL.x + b.UR.x) as i32 / 2,
            },
            _ => point {
                x: b.LL.x as i32,
                y: (b.LL.y + b.UR.y) as i32 / 2,
            },
        };
        // Java's int arithmetic, which DIST2 only widens after subtracting.
        p.x = p.x.wrapping_add(pt.x);
        p.y = p.y.wrapping_add(pt.y);
        let dx = f64::from(p.x.wrapping_sub(opt.x));
        let dy = f64::from(p.y.wrapping_sub(opt.y));
        let d = (dx * dx + dy * dy) as i32;
        if rv.is_none() || d < mind {
            mind = d;
            rv = Some(*name);
        }
    }
    rv
}

/// `resolvePort`: a dynamic port (`compass` `_`) fixed to the side closest to `other`.
pub fn resolvePort(zz: &mut Globals, n: NodeId, other: NodeId, oldport: &port) -> port {
    let mut rv = port::default();
    let compass = closestSide(zz, n, other, oldport);
    rv.name = oldport.name;
    compassPort(
        zz,
        n,
        oldport.bp,
        &mut rv,
        compass.unwrap_or(""),
        oldport.side,
    );
    rv
}

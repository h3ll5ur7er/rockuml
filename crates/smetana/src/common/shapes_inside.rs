//! The half of `shapes.c` that edges use once nodes are sized: ports, inside tests and the port boxes of polygon
//! and record shapes. (`bind_shape` and the init functions are in `shapes.rs`.)

use crate::cgraph::obj::{agraphof, agroot};
use crate::common::geom::{BETWEEN, INSIDE, ccwrotatepf, cwrotatepf};
use crate::common::{GD_flip, GD_rankdir};
use crate::core::Globals;
use crate::core::consts::{BOTTOM, LEFT, M_PI, RIGHT, TOP};
use crate::core::ids::{FieldId, NodeId};
use crate::core::jmath::{ROUND, atan2, hypot};
use crate::h::{
    Center, SHAPE_INFO, boxf, inside_t, point, pointf, pointfof, polygon_t, port, shape_functions,
};

/// A shape's `insidefn`: whether a point, relative to the node's centre, is inside the shape (or port box).
pub type InsideFn = fn(&mut Globals, &inside_t, pointf) -> bool;
/// A shape's `portfn`: resolves a port name and compass point.
pub type PortFn = fn(&mut Globals, NodeId, &str, Option<&str>) -> port;
/// A shape's `pboxfn`: the box an edge leaves through when its port demands one; returns the side mask.
pub type PboxFn = fn(&mut Globals, NodeId, &port, i32, &mut boxf, &mut i32) -> i32;

impl shape_functions {
    pub fn portfn(self) -> PortFn {
        match self {
            shape_functions::poly_fns => poly_port,
            shape_functions::record_fns => record_port,
        }
    }

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
    let mut P = ccwrotatepf(p, 90 * GD_rankdir(zz, g));
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
        if GD_flip(zz, g) {
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

/// `invflip_side`: a side of a node in the rank direction's coordinates, back in the graph's.
fn invflip_side(side: i32, rankdir: i32) -> i32 {
    match rankdir {
        0 => side,
        _ => unimplemented!("o4wjkq58uh9dgs94m2vxettc: invflip_side for rankdir {rankdir}"),
    }
}

/// `invflip_angle`, likewise for an angle.
fn invflip_angle(angle: f64, rankdir: i32) -> f64 {
    match rankdir {
        0 => angle,
        _ => unimplemented!("b5wrpw5rvhjh7999v3sqqlbo3: invflip_angle for rankdir {rankdir}"),
    }
}

/// `compassPort`: sets `pp` to the port at `compass` of the box `bp` (or of the whole node). Returns whether the
/// compass point was not recognised. Smetana has no inside context here: ports only exist on boxes and records.
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
fn compassPort(
    zz: &mut Globals,
    n: NodeId,
    bp: Option<boxf>,
    pp: &mut port,
    compass: Option<&str>,
    sides: i32,
) -> bool {
    let mut b = boxf::default();
    let mut p = pointf::default();
    let mut rv = 0;
    let mut theta = 0.0;
    let mut constrain = false;
    let mut dyna = false;
    let mut side = 0;
    let mut clip = true;
    let mut defined;
    let g = agraphof(zz, n);
    if let Some(bp) = bp {
        b = bp;
        p = pointfof((b.LL.x + b.UR.x) / 2.0, (b.LL.y + b.UR.y) / 2.0);
        defined = true;
    } else {
        if GD_flip(zz, g) {
            unimplemented!("e21k9f24vr25zdbgo37m5er48: compassPort on a flipped node");
        }
        b.UR.y = zz.nd(n).ht / 2.0;
        b.LL.y = -b.UR.y;
        b.UR.x = zz.nd(n).lw;
        b.LL.x = -b.UR.x;
        defined = false;
    }
    let ctr = p;
    if let Some(compass) = compass.filter(|c| !c.is_empty()) {
        let mut chars = compass.chars();
        let first = chars.next();
        let next = chars.next();
        match first {
            Some('e') => {
                if next.is_some() {
                    unimplemented!("en0rarvkx5srsxnlqpf6ja1us: compass {compass}");
                }
                p.x = b.UR.x;
                theta = 0.0;
                constrain = true;
                defined = true;
                clip = false;
                side = sides & RIGHT;
            }
            Some('s') => {
                p.y = b.LL.y;
                constrain = true;
                clip = false;
                if next.is_some() {
                    unimplemented!("avfplp4wadl774qo2yrqn2btg: compass {compass}");
                }
                theta = -M_PI * 0.5;
                defined = true;
                p.x = ctr.x;
                side = sides & BOTTOM;
            }
            Some('w') => {
                if next.is_some() {
                    rv = 1;
                } else {
                    p.x = b.LL.x;
                    theta = M_PI;
                    constrain = true;
                    defined = true;
                    clip = false;
                    side = sides & LEFT;
                }
            }
            Some('n') => {
                p.y = b.UR.y;
                constrain = true;
                clip = false;
                if next.is_some() {
                    unimplemented!("bfouf47misaa32ulv25melpbm: compass {compass}");
                }
                defined = true;
                theta = M_PI * 0.5;
                p.x = ctr.x;
                side = sides & TOP;
            }
            Some('_') => {
                dyna = true;
                side = sides;
            }
            Some('c') => unimplemented!("ai3czg6gaaxspsmndknpyvuiu: compass c"),
            _ => rv = 1,
        }
    }
    let rankdir = GD_rankdir(zz, g);
    p = cwrotatepf(p, 90 * rankdir);
    pp.side = if dyna {
        side
    } else {
        invflip_side(side, rankdir)
    };
    pp.bp = bp;
    pp.p = pointf {
        x: f64::from(ROUND(p.x)),
        y: f64::from(ROUND(p.y)),
    };
    pp.theta = invflip_angle(theta, rankdir);
    if p.x == 0.0 && p.y == 0.0 {
        pp.order = 256 / 2;
    } else {
        let mut angle = atan2(p.y, p.x) + 1.5 * M_PI;
        if angle >= 2.0 * M_PI {
            angle -= 2.0 * M_PI;
        }
        pp.order = ((256.0 * angle) / (2.0 * M_PI)) as i32;
    }
    pp.constrained = constrain;
    pp.defined = defined;
    pp.clip = clip;
    pp.dyna = dyna;
    rv != 0
}

fn IS_BOX(zz: &Globals, n: NodeId) -> bool {
    zz.Shapes[zz.nd(n).shape.expect("node without shape")].name == "box"
}

/// `poly_port`. Smetana's `unrecognized` only prints a warning, which the engine leaves out.
pub fn poly_port(zz: &mut Globals, n: NodeId, portname: &str, _compass: Option<&str>) -> port {
    if portname.is_empty() {
        return Center;
    }
    let mut rv = port::default();
    let sides = BOTTOM | RIGHT | TOP | LEFT;
    if has_html_label(zz, n) {
        unimplemented!("dl6n43wu7irkeiaxb6wed3388: html_port");
    }
    if !IS_BOX(zz, n) {
        unimplemented!("17pbmb7rfq2rdapm13ww6pefz: port on a non-box polygon");
    }
    compassPort(zz, n, None, &mut rv, Some(portname), sides);
    rv
}

/// `map_rec_port`: the field of `f`'s tree named `str`.
fn map_rec_port(zz: &Globals, f: FieldId, str: &str) -> Option<FieldId> {
    let field = &zz.fields[f];
    if field.id.as_deref() == Some(str) {
        return Some(f);
    }
    (0..field.n_flds).find_map(|sub| {
        let subf = zz
            .field_lists
            .get(field.fld.expect("field without subfields"), sub);
        map_rec_port(zz, subf.expect("NULL subfield"), str)
    })
}

/// `record_port`: a named field's port, or a compass point of the whole record.
pub fn record_port(zz: &mut Globals, n: NodeId, portname: &str, compass: Option<&str>) -> port {
    if portname.is_empty() {
        return Center;
    }
    let mut rv = port::default();
    let sides = BOTTOM | RIGHT | TOP | LEFT;
    let compass = compass.unwrap_or("_");
    let f = field_of(zz, n);
    if let Some(subf) = map_rec_port(zz, f, portname) {
        let (b, subf_sides) = (zz.fields[subf].b, zz.fields[subf].sides);
        if compassPort(zz, n, Some(b), &mut rv, Some(compass), subf_sides) {
            unimplemented!("cw5grwj6gbj94jcztvnp2ooyj: unrecognized compass point {compass}");
        }
    } else {
        let b = zz.fields[f].b;
        compassPort(zz, n, Some(b), &mut rv, Some(portname), sides);
    }
    rv
}

/// `record_inside`.
pub fn record_inside(zz: &mut Globals, inside_context: &inside_t, p: pointf) -> bool {
    let n = inside_context.s_n.expect("inside_context.s.n");
    let p = ccwrotatepf(p, 90 * GD_rankdir(zz, agraphof(zz, n)));
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
        if GD_flip(zz, g) {
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
    let rkd = GD_rankdir(zz, agroot(zz, n));
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
        p.x += pt.x;
        p.y += pt.y;
        let dx = f64::from(p.x - opt.x);
        let dy = f64::from(p.y - opt.y);
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
    compassPort(zz, n, oldport.bp, &mut rv, compass, oldport.side);
    rv
}

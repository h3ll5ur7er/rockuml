//! `splines.c`: clipping routed splines to their nodes and arrowheads and installing them on edges, the boxes
//! around an edge's end nodes, and self loops.

use crate::cgraph::obj::agraphof;
use crate::cgraph::{aghead, agtail};

use crate::common::arrows::{arrow_flags, arrowEndClip, arrowStartClip};
use crate::common::emit::update_bb_bz;
use crate::common::geom::APPROXEQPT;
use crate::common::shapes_inside::{InsideFn, resolvePort};
use crate::common::utils::{Bezier, dotneato_closest};
use crate::core::Globals;
use crate::core::carray::CArray;
use crate::core::consts::{
    BOTTOM, ET_CURVED, ET_SPLINE, FLATEDGE, LEFT, MILLIPOINT, NORMAL, REGULAREDGE, RIGHT, SELFEDGE,
    TOP,
};
use crate::core::ids::{EdgeId, GraphId, NodeId, SplinesId};
use crate::core::jmath::{max, min};
use crate::h::{
    add_pointf, bezier, boxf, inside_t, path, pathend_t, pointf, pointfof, splineInfo, splines,
};

/// The shape's `insidefn`, if the node has a shape.
fn insidefn(zz: &Globals, n: NodeId) -> Option<InsideFn> {
    zz.nd(n).shape.map(|shape| zz.Shapes[shape].fns.insidefn())
}

fn to_orig(zz: &Globals, e: EdgeId) -> EdgeId {
    zz.ed(e).to_orig.expect("virtual edge without original")
}

/// The real edge behind `e`, the first along `ED_to_orig` whose type is `NORMAL`.
fn normal_orig(zz: &Globals, mut e: EdgeId) -> EdgeId {
    while zz.ed(e).edge_type != NORMAL {
        e = to_orig(zz, e);
    }
    e
}

/// `arrow_clip`: shortens the ends of the spline `ps[startp..endp + 4]` by the edge's arrowheads.
#[allow(clippy::too_many_arguments, reason = "Graphviz's signature")]
fn arrow_clip(
    zz: &mut Globals,
    fe: EdgeId,
    hn: NodeId,
    ps: &mut [pointf],
    startp: &mut i32,
    endp: &mut i32,
    spl: &mut bezier,
    info: &splineInfo,
) {
    let mut e = fe;
    while let Some(orig) = zz.ed(e).to_orig {
        e = orig;
    }
    let j = if info.ignoreSwap {
        false
    } else {
        (info.swapEnds)(zz, e)
    };
    let mut sflag = 0;
    let mut eflag = 0;
    arrow_flags(zz, e, &mut sflag, &mut eflag);
    if (info.splineMerge)(zz, hn) {
        eflag = 0;
    }
    if (info.splineMerge)(zz, agtail(zz, fe)) {
        sflag = 0;
    }
    if j {
        std::mem::swap(&mut sflag, &mut eflag);
    }
    if info.isOrtho {
        unimplemented!("7a3lmojyfh13d6shkviuogx2c: arrowOrthoClip");
    }
    if sflag != 0 {
        *startp = arrowStartClip(zz, e, ps, *startp, *endp, spl, sflag);
    }
    if eflag != 0 {
        *endp = arrowEndClip(zz, e, ps, *startp, *endp, spl, eflag);
    }
}

/// `bezier_clip`: clips the Bézier piece `sp` where it leaves the shape, by bisection. `left_inside` says
/// whether `sp[0]` (or else `sp[3]`) is the end inside the shape; points are relative to the shape's centre.
pub fn bezier_clip(
    zz: &mut Globals,
    inside_context: &inside_t,
    inside: InsideFn,
    sp: &mut [pointf; 4],
    left_inside: bool,
) {
    let mut seg = [pointf::default(); 4];
    let mut best = [pointf::default(); 4];
    let mut low = 0.0;
    let mut high = 1.0;
    let mut found = false;
    let mut pt = if left_inside { sp[0] } else { sp[3] };
    loop {
        let opt = pt;
        let t = (high + low) / 2.0;
        pt = if left_inside {
            Bezier(sp, 3, t, None, Some(&mut seg))
        } else {
            Bezier(sp, 3, t, Some(&mut seg), None)
        };
        let (idir, odir) = if left_inside {
            (&mut low, &mut high)
        } else {
            (&mut high, &mut low)
        };
        if inside(zz, inside_context, pt) {
            *idir = t;
        } else {
            best = seg;
            found = true;
            *odir = t;
        }
        if !((opt.x - pt.x).abs() > 0.5 || (opt.y - pt.y).abs() > 0.5) {
            break;
        }
    }
    *sp = if found { best } else { seg };
}

/// `shape_clip0`: clips the Bézier piece `curve[0..4]` to node `n`'s shape.
fn shape_clip0(
    zz: &mut Globals,
    inside_context: &inside_t,
    n: NodeId,
    curve: &mut [pointf],
    left_inside: bool,
) {
    let save_real_size = zz.nd(n).rw;
    let coord = zz.nd(n).coord;
    let mut c = [pointf::default(); 4];
    for i in 0..4 {
        c[i].x = curve[i].x - coord.x;
        c[i].y = curve[i].y - coord.y;
    }
    let inside = insidefn(zz, n).expect("node without shape");
    bezier_clip(zz, inside_context, inside, &mut c, left_inside);
    for i in 0..4 {
        curve[i].x = c[i].x + coord.x;
        curve[i].y = c[i].y + coord.y;
    }
    zz.nd_mut(n).rw = save_real_size;
}

/// `new_spline`: appends a Bézier of `sz` points to the splines of `e`'s real edge; returns it as a one-element
/// array.
pub fn new_spline(zz: &mut Globals, e: EdgeId, sz: i32) -> CArray<bezier> {
    let e = normal_orig(zz, e);
    if zz.ed(e).spl.is_none() {
        zz.ed_mut(e).spl = Some(zz.splines.push(splines::default()));
    }
    let spl = zz.ed(e).spl.expect("ED_spl was just set");
    let splines { list, size } = zz.splines[spl];
    let list = zz.beziers.REALLOC(size + 1, list);
    zz.splines[spl] = splines {
        list: Some(list),
        size: size + 1,
    };
    let rv = list.plus_(size);
    zz.beziers[rv.at(0)] = bezier {
        list: Some(zz.pointfs.ALLOC(sz)),
        size: sz,
        ..bezier::default()
    };
    rv
}

/// `clip_and_install`: clips the raw spline `ps[0..pn]`, which runs from `fe`'s tail to node `hn`, to the node
/// shapes and arrowheads, and installs it on the edge. Grows the graph's bounding box around it.
pub fn clip_and_install(
    zz: &mut Globals,
    fe: EdgeId,
    hn: NodeId,
    ps: &mut [pointf],
    pn: i32,
    info: &splineInfo,
) {
    let mut hn = hn;
    let mut tn = agtail(zz, fe);
    let g = agraphof(zz, tn);
    let newspl = new_spline(zz, fe, pn);
    let orig = normal_orig(zz, fe);
    if !info.ignoreSwap && zz.nd(tn).rank == zz.nd(hn).rank && zz.nd(tn).order > zz.nd(hn).order {
        std::mem::swap(&mut tn, &mut hn);
    }
    let ports = zz.ed(orig);
    let (clipTail, clipHead, tbox, hbox) = if tn == agtail(zz, orig) {
        (
            ports.tail_port.clip,
            ports.head_port.clip,
            ports.tail_port.bp,
            ports.head_port.bp,
        )
    } else {
        (
            ports.head_port.clip,
            ports.tail_port.clip,
            ports.head_port.bp,
            ports.tail_port.bp,
        )
    };
    let mut inside_context = inside_t::default();
    let mut start = 0;
    if let (true, Some(inside)) = (clipTail, insidefn(zz, tn)) {
        inside_context.s_n = Some(tn);
        inside_context.s_bp = tbox;
        while start < pn - 4 {
            let coord = zz.nd(tn).coord;
            let q = ps[(start + 3) as usize];
            let p2 = pointfof(q.x - coord.x, q.y - coord.y);
            if !inside(zz, &inside_context, p2) {
                break;
            }
            start += 3;
        }
        shape_clip0(zz, &inside_context, tn, &mut ps[start as usize..], true);
    }
    let mut end = pn - 4;
    if let (true, Some(inside)) = (clipHead, insidefn(zz, hn)) {
        inside_context.s_n = Some(hn);
        inside_context.s_bp = hbox;
        while end > 0 {
            let coord = zz.nd(hn).coord;
            let q = ps[end as usize];
            let p2 = pointfof(q.x - coord.x, q.y - coord.y);
            if !inside(zz, &inside_context, p2) {
                break;
            }
            end -= 3;
        }
        shape_clip0(zz, &inside_context, hn, &mut ps[end as usize..], false);
    }
    while start < pn - 4 && APPROXEQPT(ps[start as usize], ps[(start + 3) as usize], MILLIPOINT) {
        start += 3;
    }
    while end > 0 && APPROXEQPT(ps[end as usize], ps[(end + 3) as usize], MILLIPOINT) {
        end -= 3;
    }
    let mut spl = zz.beziers[newspl.at(0)];
    arrow_clip(zz, fe, hn, ps, &mut start, &mut end, &mut spl, info);
    let list = spl.list.expect("new spline without points");
    let mut bb = zz.gd(g).bb;
    let mut i = start;
    while i < end + 4 {
        let mut cp = [pointf::default(); 4];
        zz.pointfs.set(list, i - start, ps[i as usize]);
        cp[0] = ps[i as usize];
        i += 1;
        if i >= end + 4 {
            break;
        }
        zz.pointfs.set(list, i - start, ps[i as usize]);
        cp[1] = ps[i as usize];
        i += 1;
        zz.pointfs.set(list, i - start, ps[i as usize]);
        cp[2] = ps[i as usize];
        i += 1;
        cp[3] = ps[i as usize];
        update_bb_bz(&mut bb, &cp);
    }
    zz.gd_mut(g).bb = bb;
    spl.size = end - start + 4;
    zz.beziers[newspl.at(0)] = spl;
}

/// `conc_slope`, for concentrated edges, which PlantUML never asks for.
fn conc_slope(_n: NodeId) -> f64 {
    unimplemented!("e388y3vtrp8f6spgh9q4wx37w: conc_slope")
}

/// `add_box`: appends `b` to the path's boxes unless it is empty. `P.boxes` is allocated by the caller.
pub fn add_box(P: &mut path, b: boxf) {
    if b.LL.x < b.UR.x && b.LL.y < b.UR.y {
        P.boxes[P.nbox as usize] = b;
        P.nbox += 1;
    }
}

fn HT2(zz: &Globals, n: NodeId) -> f64 {
    zz.nd(n).ht / 2.0
}

/// `beginpath`: sets up the start of `P` at `e`'s tail and the boxes around the tail node in `endp`. `et` is
/// `REGULAREDGE` or `FLATEDGE`; for flat edges, `endp.sidemask` says which side to leave by.
pub fn beginpath(
    zz: &mut Globals,
    P: &mut path,
    e: EdgeId,
    et: i32,
    endp: &mut pathend_t,
    merge: bool,
) {
    let n = agtail(zz, e);
    if zz.ed(e).tail_port.dyna {
        let (tail, head, port) = (agtail(zz, e), aghead(zz, e), zz.ed(e).tail_port);
        zz.ed_mut(e).tail_port = resolvePort(zz, tail, head, &port);
    }
    let pboxfn = zz.nd(n).shape.map(|shape| zz.Shapes[shape].fns.pboxfn());
    let tail_port = zz.ed(e).tail_port;
    P.start.p = add_pointf(zz.nd(n).coord, tail_port.p);
    if merge {
        P.start.theta = conc_slope(agtail(zz, e));
        P.start.constrained = true;
    } else if tail_port.constrained {
        P.start.theta = tail_port.theta;
        P.start.constrained = true;
    } else {
        P.start.constrained = false;
    }
    P.nbox = 0;
    P.data = Some(e);
    endp.np = P.start.p;
    let side = tail_port.side;
    if et == REGULAREDGE && zz.nd(n).node_type == NORMAL && side != 0 {
        let mut b = endp.nb;
        if side & TOP != 0 {
            unimplemented!("1r4lctdj9z1ivlz3uqpcj1yzf: tail port on top");
        } else if side & BOTTOM != 0 {
            endp.sidemask = BOTTOM;
            b.UR.y = max(b.UR.y, P.start.p.y);
            endp.boxes[0] = b;
            endp.boxn = 1;
            P.start.p.y -= 1.0;
        } else if side & LEFT != 0 {
            endp.sidemask = LEFT;
            b.UR.x = P.start.p.x;
            b.LL.y = zz.nd(n).coord.y - HT2(zz, n);
            b.UR.y = P.start.p.y;
            endp.boxes[0] = b;
            endp.boxn = 1;
            P.start.p.x -= 1.0;
        } else {
            endp.sidemask = RIGHT;
            b.LL.x = P.start.p.x;
            b.LL.y = zz.nd(n).coord.y - HT2(zz, n);
            b.UR.y = P.start.p.y;
            endp.boxes[0] = b;
            endp.boxn = 1;
            P.start.p.x += 1.0;
        }
        let orig = normal_orig(zz, e);
        if n != agtail(zz, orig) {
            unimplemented!("2tw6ymudedo6qij3ux424ydsi: tail port on a reversed edge");
        }
        zz.ed_mut(orig).tail_port.clip = false;
        // Unlike Graphviz, Smetana leaves endp.sidemask alone here.
        return;
    }
    if et == FLATEDGE && side != 0 {
        unimplemented!("ew7nyfe712nsiphifeztwxfop: flat edge with a tail port");
    }
    let side = if et == REGULAREDGE {
        BOTTOM
    } else {
        endp.sidemask
    };
    let mask = match pboxfn {
        Some(pboxfn) => pboxfn(zz, n, &tail_port, side, &mut endp.boxes[0], &mut endp.boxn),
        None => 0,
    };
    if mask != 0 {
        endp.sidemask = mask;
        return;
    }
    endp.boxes[0] = endp.nb;
    endp.boxn = 1;
    match et {
        SELFEDGE => unimplemented!("9rnob8jdqqdjwzanv53yxc47u: beginpath for a self edge"),
        FLATEDGE => {
            if endp.sidemask == TOP {
                endp.boxes[0].LL.y = P.start.p.y;
            } else {
                endp.boxes[0].UR.y = P.start.p.y;
            }
        }
        REGULAREDGE => {
            endp.boxes[0].UR.y = P.start.p.y;
            endp.sidemask = BOTTOM;
            P.start.p.y -= 1.0;
        }
        _ => {}
    }
}

/// `endpath`: the same at `e`'s head.
pub fn endpath(
    zz: &mut Globals,
    P: &mut path,
    e: EdgeId,
    et: i32,
    endp: &mut pathend_t,
    merge: bool,
) {
    let n = aghead(zz, e);
    if zz.ed(e).head_port.dyna {
        let (tail, head, port) = (agtail(zz, e), aghead(zz, e), zz.ed(e).head_port);
        zz.ed_mut(e).head_port = resolvePort(zz, head, tail, &port);
    }
    let pboxfn = zz.nd(n).shape.map(|shape| zz.Shapes[shape].fns.pboxfn());
    let head_port = zz.ed(e).head_port;
    P.end.p = add_pointf(zz.nd(n).coord, head_port.p);
    if merge {
        unimplemented!("endpath with merge: conc_slope");
    } else if head_port.constrained {
        P.end.theta = head_port.theta;
        P.end.constrained = true;
    } else {
        P.end.constrained = false;
    }
    endp.np = P.end.p;
    let side = head_port.side;
    if et == REGULAREDGE && zz.nd(n).node_type == NORMAL && side != 0 {
        let mut b = endp.nb;
        if side & TOP != 0 {
            endp.sidemask = TOP;
            b.LL.y = min(b.LL.y, P.end.p.y);
            endp.boxes[0] = b;
            endp.boxn = 1;
            P.end.p.y += 1.0;
        } else if side & BOTTOM != 0 {
            unimplemented!("auefgwb39x5hzqqc9b1zgl239: head port at the bottom");
        } else if side & LEFT != 0 {
            unimplemented!("2lmjkw07sr4x9a3xxrcb3yj07: head port on the left");
        } else {
            endp.sidemask = RIGHT;
            b.LL.x = P.end.p.x;
            b.UR.y = zz.nd(n).coord.y + HT2(zz, n);
            b.LL.y = P.end.p.y;
            endp.boxes[0] = b;
            endp.boxn = 1;
            P.end.p.x += 1.0;
        }
        let orig = normal_orig(zz, e);
        if n != aghead(zz, orig) {
            unimplemented!("dk49xvmby8949ngdmft4sgrox: head port on a reversed edge");
        }
        zz.ed_mut(orig).head_port.clip = false;
        endp.sidemask = side;
        return;
    }
    if et == FLATEDGE && side != 0 {
        unimplemented!("ew7nyfe712nsiphifeztwxfop: flat edge with a head port");
    }
    let side = if et == REGULAREDGE {
        TOP
    } else {
        endp.sidemask
    };
    let mask = match pboxfn {
        Some(pboxfn) => pboxfn(zz, n, &head_port, side, &mut endp.boxes[0], &mut endp.boxn),
        None => 0,
    };
    if mask != 0 {
        endp.sidemask = mask;
        return;
    }
    endp.boxes[0] = endp.nb;
    endp.boxn = 1;
    match et {
        SELFEDGE => unimplemented!("bhkhf4i9pvxtxyka4sobszg33: endpath for a self edge"),
        FLATEDGE => {
            if endp.sidemask == TOP {
                endp.boxes[0].LL.y = P.end.p.y;
            } else {
                endp.boxes[0].UR.y = P.end.p.y;
            }
        }
        REGULAREDGE => {
            endp.boxes[0].LL.y = P.end.p.y;
            endp.sidemask = TOP;
            P.end.p.y += 1.0;
        }
        _ => {}
    }
}

/// `convert_sides_to_points`: a code for the pair of node points (corners and side midpoints) two sides meet at.
fn convert_sides_to_points(tail_side: i32, head_side: i32) -> i32 {
    const vertices: [i32; 8] = [12, 4, 6, 2, 3, 1, 9, 8];
    let tail_i = vertices.iter().position(|&v| v == tail_side);
    let head_i = vertices.iter().position(|&v| v == head_side);
    match (tail_i, head_i) {
        // pair_a[tail_i][head_i] in C, a table of these numbers.
        (Some(t), Some(h)) => (t as i32 + 1) * 10 + h as i32 + 1,
        _ => 0,
    }
}

/// `selfRight`: loops `edges[ind..ind + cnt]` around the right side of their node, `stepx` apart.
fn selfRight(
    zz: &mut Globals,
    edges: &[EdgeId],
    mut ind: usize,
    cnt: i32,
    stepx: f64,
    sizey: f64,
    sinfo: &splineInfo,
) {
    let e = edges[ind];
    let n = agtail(zz, e);
    let stepy = max((sizey / 2.0) / f64::from(cnt), 2.0);
    let np = zz.nd(n).coord;
    let ports = *zz.ed(e);
    let tp = add_pointf(ports.tail_port.p, np);
    let hp = add_pointf(ports.head_port.p, np);
    let mut sgn = if tp.y >= hp.y { 1 } else { -1 };
    let mut dx = zz.nd(n).rw;
    let mut dy = 0.0;
    let point_pair = convert_sides_to_points(ports.tail_port.side, ports.head_port.side);
    if matches!(point_pair, 32 | 65) && tp.y == hp.y {
        sgn = -sgn;
    }
    let mut tx = min(dx, 3.0 * (np.x + dx - tp.x));
    let mut hx = min(dx, 3.0 * (np.x + dx - hp.x));
    for _ in 0..cnt {
        let e = edges[ind];
        ind += 1;
        dx += stepx;
        tx += stepx;
        hx += stepx;
        dy += f64::from(sgn) * stepy;
        let mut points = [
            tp,
            pointfof(tp.x + tx / 3.0, tp.y + dy),
            pointfof(np.x + dx, tp.y + dy),
            pointfof(np.x + dx, (tp.y + hp.y) / 2.0),
            pointfof(np.x + dx, hp.y - dy),
            pointfof(hp.x + hx / 3.0, hp.y - dy),
            hp,
        ];
        if let Some(label) = zz.ed(e).label {
            let dimen = zz.textlabels[label].dimen;
            let width = if zz.gd(agraphof(zz, agtail(zz, e))).GD_flip() {
                dimen.y
            } else {
                dimen.x
            };
            let coord = zz.nd(n).coord;
            let l = &mut zz.textlabels[label];
            l.pos.x = coord.x + dx + width / 2.0;
            l.pos.y = coord.y;
            l.set = 1;
            if width > stepx {
                dx += width - stepx;
            }
        }
        let head = aghead(zz, e);
        clip_and_install(zz, e, head, &mut points, 7, sinfo);
    }
}

/// Whether a self loop's ports let it go around the right side of its node: none, or none on the left and not
/// both on the same top or bottom side.
fn loops_right(zz: &Globals, e: EdgeId) -> bool {
    let (tail, head) = (zz.ed(e).tail_port, zz.ed(e).head_port);
    (!tail.defined && !head.defined)
        || (tail.side & LEFT == 0
            && head.side & LEFT == 0
            && (tail.side != head.side || tail.side & (TOP | BOTTOM) == 0))
}

/// `selfRightSpace`: the room a self loop needs right of its node.
pub fn selfRightSpace(zz: &Globals, e: EdgeId) -> i32 {
    if !loops_right(zz, e) {
        return 0;
    }
    let mut sw = 18;
    if let Some(l) = zz.ed(e).label {
        let dimen = zz.textlabels[l].dimen;
        let label_width = if zz.gd(agraphof(zz, aghead(zz, e))).GD_flip() {
            dimen.y
        } else {
            dimen.x
        };
        // Java's `sw += label_width` truncates the sum back to int.
        sw = (f64::from(sw) + label_width) as i32;
    }
    sw
}

/// `makeSelfEdge`: routes the `cnt` self loops `edges[ind..]` of one node. Smetana only has loops on the right.
pub fn makeSelfEdge(
    zz: &mut Globals,
    edges: &[EdgeId],
    ind: usize,
    cnt: i32,
    sizex: f64,
    sizey: f64,
    sinfo: &splineInfo,
) {
    let e = edges[ind];
    if loops_right(zz, e) {
        selfRight(zz, edges, ind, cnt, sizex, sizey, sinfo);
    } else {
        unimplemented!(
            "self loop with ports on the left, top or bottom: selfLeft/selfTop/selfBottom"
        );
    }
}

/// `endPoints`: where spline `spl` touches its nodes. Smetana has no arrowheads on spline ends here.
fn endPoints(zz: &Globals, spl: SplinesId) -> (pointf, pointf) {
    let spl = zz.splines[spl];
    let list = spl.list.expect("spline without beziers");
    let first = zz.beziers.get(list, 0);
    if first.sflag != 0 {
        unimplemented!("4wazlko0bxmzxoobqacij1btk: endPoints with a start arrow");
    }
    let p = zz
        .pointfs
        .get(first.list.expect("bezier without points"), 0);
    let last = zz.beziers.get(list, spl.size - 1);
    if last.eflag != 0 {
        unimplemented!("78u9nvs8u7rxturidz5nf8hn4: endPoints with an end arrow");
    }
    let q = zz
        .pointfs
        .get(last.list.expect("bezier without points"), last.size - 1);
    (p, q)
}

/// `edgeMidpoint`: the point of `e`'s spline halfway between its ends.
pub fn edgeMidpoint(zz: &Globals, g: GraphId, e: EdgeId) -> pointf {
    let et = zz.gd(g).flags & (7 << 1);
    let spl = zz.ed(e).spl.expect("edge without spline");
    let (p, q) = endPoints(zz, spl);
    if APPROXEQPT(p, q, MILLIPOINT) {
        unimplemented!("7i8m5mpfnv7m9uqxh015zfdaj: edgeMidpoint of a degenerate spline");
    } else if et == ET_SPLINE || et == ET_CURVED {
        let d = pointfof((q.x + p.x) / 2.0, (p.y + q.y) / 2.0);
        dotneato_closest(zz, &zz.splines[spl], d)
    } else {
        unimplemented!("6he3hi05vusuthrchn4enk7o6: polylineMidpoint")
    }
}

/// `getsplinepoints`: the splines drawn for `e`, found along `ED_to_orig` if `e` is virtual.
pub fn getsplinepoints(zz: &Globals, e: EdgeId) -> Option<SplinesId> {
    let mut le = e;
    loop {
        let sp = zz.ed(le).spl;
        if sp.is_some() || zz.ed(le).edge_type == NORMAL {
            return sp;
        }
        le = to_orig(zz, le);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_pairs_number_like_graphviz_table() {
        assert_eq!(convert_sides_to_points(TOP | RIGHT, TOP), 32);
        assert_eq!(convert_sides_to_points(BOTTOM, BOTTOM | RIGHT), 65);
        assert_eq!(convert_sides_to_points(TOP | LEFT, LEFT), 18);
        assert_eq!(convert_sides_to_points(0, TOP), 0);
    }
}

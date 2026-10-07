//! `dotgen/dotsplines.c`: routes the edges once the nodes are placed. Edges that would be routed alike are
//! grouped; each group becomes self loops, flat edges (between nodes of one rank) or regular edges (down the
//! ranks, through their chains of virtual nodes), drawn through a corridor of boxes around the nodes in the way.

#![allow(
    clippy::manual_midpoint,
    reason = "f64::midpoint may round differently from Java's (a + b) / 2"
)]

use crate::cgraph::edge::{agfstout, agnxtout};
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::obj::agraphof;
use crate::cgraph::{AGSEQ, M_aghead, M_agtail, MAKEFWDEDGE, aghead, agtail};
use crate::common::geom::BETWEEN;
use crate::common::routespl::{routepolylines, routesplines, simpleSplineRoute};
use crate::common::splines::{
    add_box, beginpath, clip_and_install, endpath, getsplinepoints, makeSelfEdge,
};
use crate::common::utils::updateBB;
use crate::core::Globals;
use crate::core::consts::{
    BOTTOM, EDGE_LABEL, EDGETYPEMASK, ET_CURVED, ET_LINE, ET_NONE, ET_PLINE, ET_SPLINE, FLATEDGE,
    FLATORDER, GVSPLINES, IGNORED, M_PI, NORMAL, REGULAREDGE, TOP, VIRTUAL,
};
use crate::core::ids::{EdgeId, GraphId, NodeId, SplinesId};
use crate::core::jmath::{ROUND, max, min};
use crate::core::jutils::qsort;
use crate::dotgen::cluster::mark_lowclusters;
use crate::dotgen::fastgr::new_edge_pair;
use crate::dotgen::mincross::{rank_node, rank_v};
use crate::h::{add_pointf, bezier, boxf, path, pathend_t, pointf, pointfof, port, splineInfo};

const NSUB: i32 = 9;
const MINW: i32 = 16;
const HALFMINW: f64 = 8.0;
const FWDEDGE: i32 = 16;
const BWDEDGE: i32 = 32;
const MAINGRAPH: i32 = 64;
const AUXGRAPH: i32 = 128;
const FUDGE: f64 = 4.0;

/// `sinfo`: how dot answers the questions spline clipping asks.
#[allow(non_upper_case_globals, reason = "Graphviz's name")]
pub const sinfo: splineInfo = splineInfo {
    swapEnds: swap_ends_p,
    splineMerge: spline_merge,
    ignoreSwap: false,
    isOrtho: false,
};

/// `spline_info_t`: the horizontal limits of routing and the gaps between parallel splines.
struct spline_info_t {
    LeftBound: i32,
    RightBound: i32,
    Splinesep: i32,
    Multisep: i32,
    Rank_box: Vec<boxf>,
}

fn boxfof(llx: f64, lly: f64, urx: f64, ury: f64) -> boxf {
    boxf {
        LL: pointfof(llx, lly),
        UR: pointfof(urx, ury),
    }
}

fn out0(zz: &Globals, n: NodeId) -> EdgeId {
    zz.nd(n)
        .out
        .get(&zz.edge_lists, 0)
        .expect("node without out-edge")
}

fn in0(zz: &Globals, n: NodeId) -> Option<EdgeId> {
    zz.nd(n).in_.get(&zz.edge_lists, 0)
}

fn coord(zz: &Globals, n: NodeId) -> pointf {
    zz.nd(n).coord
}

/// `getmainedge`: the real edge `e` stands for.
fn getmainedge(zz: &Globals, e: EdgeId) -> EdgeId {
    let mut le = e;
    while let Some(v) = zz.ed(le).to_virt {
        le = v;
    }
    while let Some(o) = zz.ed(le).to_orig {
        le = o;
    }
    le
}

/// `spline_merge`: whether edges merge at virtual node `n` (only with `concentrate`).
pub fn spline_merge(zz: &Globals, n: NodeId) -> bool {
    let nd = zz.nd(n);
    nd.node_type == VIRTUAL && (nd.in_.size > 1 || nd.out.size > 1)
}

/// `swap_ends_p`: whether `e`'s real edge runs up the ranks, or right to left within one.
pub fn swap_ends_p(zz: &Globals, mut e: EdgeId) -> bool {
    while let Some(o) = zz.ed(e).to_orig {
        e = o;
    }
    let (head, tail) = (zz.nd(aghead(zz, e)), zz.nd(agtail(zz, e)));
    if head.rank > tail.rank {
        return false;
    }
    if head.rank < tail.rank {
        return true;
    }
    head.order < tail.order
}

/// `portcmp`.
fn portcmp(p0: &port, p1: &port) -> i32 {
    if !p1.defined {
        return i32::from(p0.defined);
    }
    if !p0.defined {
        return -1;
    }
    let rv = (p0.p.x - p1.p.x) as i32;
    if rv == 0 {
        (p0.p.y - p1.p.y) as i32
    } else {
        rv
    }
}

/// The tail and head ports of `e` as seen going forward: those of `MAKEFWDEDGE(e)` for a backward edge.
fn fwd_ports(zz: &Globals, e: EdgeId) -> (port, port) {
    let ed = zz.ed(e);
    if ed.tree_index & BWDEDGE != 0 {
        (ed.head_port, ed.tail_port)
    } else {
        (ed.tail_port, ed.head_port)
    }
}

/// `swap_bezier`: `old` reversed.
fn swap_bezier(zz: &mut Globals, old: bezier) -> bezier {
    let sz = old.size;
    let list = zz.pointfs.ALLOC(sz);
    let olist = old.list.expect("bezier without points");
    for i in 0..sz {
        let p = zz.pointfs.get(olist, sz - 1 - i);
        zz.pointfs.set(list, i, p);
    }
    bezier {
        list: Some(list),
        size: sz,
        sflag: old.eflag,
        eflag: old.sflag,
        sp: old.ep,
        ep: old.sp,
    }
}

/// `swap_spline`: reverses the splines `s`, so that they run from the other end.
fn swap_spline(zz: &mut Globals, s: SplinesId) {
    let sz = zz.splines[s].size;
    let olist = zz.splines[s].list.expect("splines without beziers");
    let list = zz.beziers.ALLOC(sz);
    for i in 0..sz {
        let old = zz.beziers.get(olist, sz - 1 - i);
        let new = swap_bezier(zz, old);
        zz.beziers.set(list, i, new);
    }
    zz.splines[s].list = Some(list);
}

/// `edge_normalize`: makes every spline run from its edge's tail to its head.
fn edge_normalize(zz: &mut Globals, g: GraphId) {
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        let mut e = agfstout(zz, g, nn);
        while let Some(ee) = e {
            if let Some(spl) = zz.ed(ee).spl
                && swap_ends_p(zz, ee)
            {
                swap_spline(zz, spl);
            }
            e = agnxtout(zz, g, ee);
        }
        n = agnxtnode(zz, g, nn);
    }
}

/// `dot_splines`, that is `_dot_splines(g, 1)`: routes all edges of `g`, then turns the splines to run tail to
/// head. C skips that normalization only for the auxiliary graph of flat edges with ports, which Smetana does not
/// support.
#[allow(clippy::too_many_lines, reason = "Graphviz's function")]
pub fn dot_splines(zz: &mut Globals, g: GraphId) {
    let et = zz.gd(g).flags & (7 << 1);
    if et == ET_NONE {
        return;
    }
    if et == ET_CURVED {
        unimplemented!("resetRW: splines=curved");
    }
    mark_lowclusters(zz, g);
    let mut P = path::default();
    let nodesep = zz.gd(g).nodesep;
    let mut sd = spline_info_t {
        LeftBound: 0,
        RightBound: 0,
        Splinesep: nodesep / 4,
        Multisep: nodesep,
        Rank_box: Vec::new(),
    };
    let mut edges: Vec<EdgeId> = Vec::new();

    // Compute boundaries and list of splines.
    let (minrank, maxrank) = (zz.gd(g).minrank, zz.gd(g).maxrank);
    let mut n_nodes = 0;
    for i in minrank..=maxrank {
        let rank = *zz.rank(g, i);
        let v = rank.v.expect("rank without nodes");
        n_nodes += rank.n;
        if let Some(n) = zz.node_lists.get(v, 0) {
            sd.LeftBound = min(f64::from(sd.LeftBound), coord(zz, n).x - zz.nd(n).lw) as i32;
        }
        if rank.n != 0
            && let Some(n) = zz.node_lists.get(v, rank.n - 1)
        {
            sd.RightBound = max(f64::from(sd.RightBound), coord(zz, n).x + zz.nd(n).rw) as i32;
        }
        sd.LeftBound -= MINW;
        sd.RightBound += MINW;

        for j in 0..rank.n {
            let n = zz.node_lists.get(v, j).expect("node in rank");
            // The label node of a flat edge gives the label its position.
            if let Some(fe) = zz.nd(n).alg {
                let label = zz.ed(fe).label.expect("flat edge label node without label");
                let pos = coord(zz, n);
                let l = &mut zz.textlabels[label];
                l.pos = pos;
                l.set = 1;
            }
            if zz.nd(n).node_type != NORMAL && !spline_merge(zz, n) {
                continue;
            }
            let mut k = 0;
            while let Some(e) = zz.nd(n).out.get(&zz.edge_lists, k) {
                k += 1;
                let edge_type = zz.ed(e).edge_type;
                if edge_type == FLATORDER || edge_type == IGNORED {
                    continue;
                }
                setflags(zz, e, REGULAREDGE, FWDEDGE, MAINGRAPH);
                edges.push(e);
            }
            let flat_out = zz.nd(n).flat_out;
            if flat_out.list.is_some() {
                let mut k = 0;
                while let Some(e) = flat_out.get(&zz.edge_lists, k) {
                    k += 1;
                    setflags(zz, e, FLATEDGE, 0, AUXGRAPH);
                    edges.push(e);
                }
            }
            let other = zz.nd(n).other;
            if other.list.is_some() {
                // Position kept the node's rw in mval and widened it for its loops; restore it.
                if zz.nd(n).node_type == NORMAL {
                    let nd = zz.nd_mut(n);
                    std::mem::swap(&mut nd.rw, &mut nd.mval);
                }
                let mut k = 0;
                while let Some(e) = other.get(&zz.edge_lists, k) {
                    k += 1;
                    setflags(zz, e, 0, 0, AUXGRAPH);
                    edges.push(e);
                }
            }
        }
    }
    // Sort so that equivalent edges, which would be routed identically, are contiguous.
    qsort(&mut edges, |e0, e1| edgecmp(zz, e0, e1));

    P.boxes = vec![boxf::default(); (n_nodes + 20 * 2 * NSUB) as usize];
    sd.Rank_box = vec![boxf::default(); (maxrank + 1) as usize];

    if et == ET_LINE {
        place_vnlabels(zz, g, false);
    }

    let n_edges = edges.len();
    let mut i = 0;
    while i < n_edges {
        let ind = i;
        let e0 = edges[i];
        i += 1;
        let le0 = getmainedge(zz, e0);
        let havePorts = zz.ed(e0).tail_port.defined || zz.ed(e0).head_port.defined;
        let ea = if havePorts { e0 } else { le0 };
        let (ea_tail, ea_head) = fwd_ports(zz, ea);

        let mut cnt = 1;
        while i < n_edges {
            let e1 = edges[i];
            if le0 != getmainedge(zz, e1) {
                break;
            }
            // All flat adjacent edges at once.
            if zz.ed(e0).adjacent == 0 {
                let eb = if zz.ed(e1).tail_port.defined || zz.ed(e1).head_port.defined {
                    if !havePorts {
                        break;
                    }
                    e1
                } else {
                    if havePorts {
                        break;
                    }
                    getmainedge(zz, e1)
                };
                let (eb_tail, eb_head) = fwd_ports(zz, eb);
                if portcmp(&ea_tail, &eb_tail) != 0 || portcmp(&ea_head, &eb_head) != 0 {
                    break;
                }
                if (zz.ed(e0).tree_index & EDGETYPEMASK) == FLATEDGE
                    && zz.ed(e0).label != zz.ed(e1).label
                {
                    break;
                }
                if zz.ed(edges[i]).tree_index & MAINGRAPH != 0 {
                    break;
                }
            }
            cnt += 1;
            i += 1;
        }

        let (tail, head) = (agtail(zz, e0), aghead(zz, e0));
        if tail == head {
            let n = tail;
            let r = zz.nd(n).rank;
            let sizey = if r == maxrank {
                if r > 0 {
                    (coord(zz, rank_node(zz, g, r - 1, 0)).y - coord(zz, n).y) as i32
                } else {
                    zz.nd(n).ht as i32
                }
            } else if r == minrank {
                (coord(zz, n).y - coord(zz, rank_node(zz, g, r + 1, 0)).y) as i32
            } else {
                let upy = (coord(zz, rank_node(zz, g, r - 1, 0)).y - coord(zz, n).y) as i32;
                let dwny = (coord(zz, n).y - coord(zz, rank_node(zz, g, r + 1, 0)).y) as i32;
                upy.min(dwny)
            };
            makeSelfEdge(
                zz,
                &edges,
                ind,
                cnt,
                f64::from(sd.Multisep),
                f64::from(sizey / 2),
                &sinfo,
            );
            for &e in &edges[ind..ind + cnt as usize] {
                if let Some(label) = zz.ed(e).label {
                    updateBB(zz, g, label);
                }
            }
        } else if zz.nd(tail).rank == zz.nd(head).rank {
            make_flat_edge(zz, g, &sd, &mut P, &edges, ind, cnt, et);
        } else {
            make_regular_edge(zz, g, &mut sd, &mut P, &edges, ind, cnt, et);
        }
    }

    place_vnlabels(zz, g, true);

    edge_normalize(zz, g);
    if (zz.E_headlabel.is_some() || zz.E_taillabel.is_some())
        && (zz.E_labelangle.is_some() || zz.E_labeldistance.is_some())
    {
        unimplemented!("place_portlabel");
    }
    zz.State = GVSPLINES;
    zz.EdgeLabelsDone = 1;
}

/// Places the labels of the virtual nodes that carry regular edges' labels; with `update_bb`, grows the graph's
/// bounding box around them.
fn place_vnlabels(zz: &mut Globals, g: GraphId, update_bb: bool) {
    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        if zz.nd(nn).node_type == VIRTUAL
            && let Some(label) = zz.nd(nn).label
        {
            place_vnlabel(zz, nn);
            if update_bb {
                updateBB(zz, g, label);
            }
        }
        n = zz.nd(nn).next;
    }
}

/// `place_vnlabel`: puts the label of the edge through virtual node `n` right of the node.
fn place_vnlabel(zz: &mut Globals, n: NodeId) {
    if zz.nd(n).in_.size == 0 {
        // Flat edge labels are placed elsewhere.
        return;
    }
    let mut e = out0(zz, n);
    while zz.ed(e).edge_type != NORMAL {
        e = zz.ed(e).to_orig.expect("virtual edge without original");
    }
    let label = zz.ed(e).label.expect("label node of an edge without label");
    let dimen = zz.textlabels[label].dimen;
    let width = if zz.gd(agraphof(zz, n)).GD_flip() {
        dimen.y
    } else {
        dimen.x
    };
    let c = coord(zz, n);
    let l = &mut zz.textlabels[label];
    l.pos.x = c.x + width / 2.0;
    l.pos.y = c.y;
    l.set = 1;
}

/// `setflags`: classifies `e` in `ED_tree_index`: its kind (regular, flat, self loop), its direction and the
/// graph it comes from. Zero hints are worked out from the edge.
fn setflags(zz: &mut Globals, e: EdgeId, hint1: i32, hint2: i32, f3: i32) {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    let f1 = if hint1 != 0 {
        hint1
    } else if tail == head {
        if zz.ed(e).tail_port.defined || zz.ed(e).head_port.defined {
            4
        } else {
            8
        }
    } else if zz.nd(tail).rank == zz.nd(head).rank {
        2
    } else {
        1
    };
    let f2 = if hint2 != 0 {
        hint2
    } else if f1 == 1 {
        if zz.nd(tail).rank < zz.nd(head).rank {
            FWDEDGE
        } else {
            BWDEDGE
        }
    } else if f1 == 2 {
        if zz.nd(tail).order < zz.nd(head).order {
            FWDEDGE
        } else {
            BWDEDGE
        }
    } else {
        FWDEDGE
    };
    zz.ed_mut(e).tree_index = f1 | f2 | f3;
}

/// `edgecmp`: orders edges by kind, rank span, x span, real edge (so equivalent edges are contiguous), ports,
/// graph and creation order.
fn edgecmp(zz: &Globals, e0: EdgeId, e1: EdgeId) -> i32 {
    let et0 = zz.ed(e0).tree_index & EDGETYPEMASK;
    let et1 = zz.ed(e1).tree_index & EDGETYPEMASK;
    if et0 != et1 {
        return et1 - et0;
    }
    let le0 = getmainedge(zz, e0);
    let le1 = getmainedge(zz, e1);
    let rank_span = |le: EdgeId| {
        let t = f64::from(
            zz.nd(agtail(zz, le))
                .rank
                .wrapping_sub(zz.nd(aghead(zz, le)).rank),
        );
        (t as i32).wrapping_abs()
    };
    let (v0, v1) = (rank_span(le0), rank_span(le1));
    if v0 != v1 {
        return v0.wrapping_sub(v1);
    }
    let x_span = |le: EdgeId| {
        let t = coord(zz, agtail(zz, le)).x - coord(zz, aghead(zz, le)).x;
        (t as i32).wrapping_abs()
    };
    let (v0, v1) = (x_span(le0), x_span(le1));
    if v0 != v1 {
        return v0.wrapping_sub(v1);
    }
    // A cheap test for edges having the same set of endpoints.
    let (s0, s1) = (AGSEQ(zz, le0), AGSEQ(zz, le1));
    if s0 != s1 {
        return s0.wrapping_sub(s1);
    }
    let with_ports = |e: EdgeId, le: EdgeId| {
        if zz.ed(e).tail_port.defined || zz.ed(e).head_port.defined {
            e
        } else {
            le
        }
    };
    let (ea_tail, ea_head) = fwd_ports(zz, with_ports(e0, le0));
    let (eb_tail, eb_head) = fwd_ports(zz, with_ports(e1, le1));
    let rv = portcmp(&ea_tail, &eb_tail);
    if rv != 0 {
        return rv;
    }
    let rv = portcmp(&ea_head, &eb_head);
    if rv != 0 {
        return rv;
    }
    let et0 = zz.ed(e0).tree_index & (MAINGRAPH | AUXGRAPH);
    let et1 = zz.ed(e1).tree_index & (MAINGRAPH | AUXGRAPH);
    if et0 != et1 {
        return et0 - et1;
    }
    AGSEQ(zz, e0).wrapping_sub(AGSEQ(zz, e1))
}

/// `edgelblcmpfn`: labeled edges first, larger labels first.
fn edgelblcmpfn(zz: &Globals, e0: EdgeId, e1: EdgeId) -> i32 {
    match (zz.ed(e0).label, zz.ed(e1).label) {
        (Some(l0), Some(l1)) => {
            let sz0 = zz.textlabels[l0].dimen;
            let sz1 = zz.textlabels[l1].dimen;
            if sz0.x > sz1.x {
                -1
            } else if sz0.x < sz1.x {
                1
            } else if sz0.y > sz1.y {
                -1
            } else {
                i32::from(sz0.y < sz1.y)
            }
        }
        (Some(_), None) => -1,
        (None, Some(_)) => 1,
        (None, None) => 0,
    }
}

/// The spline through `P`'s boxes, or `None` if there is none (Java's `pn == 0`).
fn route(zz: &mut Globals, P: &mut path, splines: bool) -> Option<Vec<pointf>> {
    let ps = if splines {
        routesplines(zz, P)
    } else {
        routepolylines(zz, P)
    }?;
    (!ps.is_empty()).then_some(ps)
}

/// Clips and installs the spline `ps` on `e`.
fn install(zz: &mut Globals, e: EdgeId, hn: NodeId, ps: &mut [pointf]) {
    let pn = ps.len() as i32;
    clip_and_install(zz, e, hn, ps, pn, &sinfo);
}

/// `makeSimpleFlatLabels`: flat edges between adjacent nodes with labels: the largest label sits on the
/// straight edge, the others alternate below and above it, with the unlabeled edges outside.
#[allow(clippy::too_many_arguments, reason = "Graphviz's signature")]
#[allow(clippy::too_many_lines, reason = "Graphviz's function")]
fn makeSimpleFlatLabels(
    zz: &mut Globals,
    tn: NodeId,
    hn: NodeId,
    edges: &[EdgeId],
    ind: usize,
    cnt: i32,
    et: i32,
    n_lbls: i32,
) {
    let e = edges[ind];
    let mut earray = edges[ind..ind + cnt as usize].to_vec();
    qsort(&mut earray, |e0, e1| edgelblcmpfn(zz, e0, e1));
    let tp = add_pointf(coord(zz, tn), zz.ed(e).tail_port.p);
    let hp = add_pointf(coord(zz, hn), zz.ed(e).head_port.p);

    let leftend = tp.x + zz.nd(tn).rw;
    let rightend = hp.x - zz.nd(hn).lw;
    let ctrx = (leftend + rightend) / 2.0;

    // Do first edge.
    let e = earray[0];
    let mut points = [pointf::default(); 8];
    points[0] = tp;
    points[1] = tp;
    points[2] = hp;
    points[3] = hp;
    let head = aghead(zz, e);
    install(zz, e, head, &mut points[..4]);
    let label = zz.ed(e).label.expect("labeled edge");
    let dimen = zz.textlabels[label].dimen;
    let l = &mut zz.textlabels[label];
    l.pos.x = ctrx;
    l.pos.y = tp.y + (dimen.y + 6.0) / 2.0;
    l.set = 1;

    let mut miny = tp.y + 6.0 / 2.0;
    let mut maxy = miny + dimen.y;
    let uminx = ctrx - dimen.x / 2.0;
    let umaxx = ctrx + dimen.x / 2.0;
    let mut lminx = 0.0;
    let mut lmaxx = 0.0;

    let polyline = et == ET_PLINE;
    let mut i = 1;
    while i < n_lbls {
        let e = earray[i as usize];
        let label = zz.ed(e).label.expect("labeled edge");
        let dimen = zz.textlabels[label].dimen;
        let ctry;
        if i % 2 != 0 {
            // Down.
            if i == 1 {
                lminx = ctrx - dimen.x / 2.0;
                lmaxx = ctrx + dimen.x / 2.0;
            }
            miny -= 6.0 + dimen.y;
            points = below(tp, hp, miny, lminx, lmaxx);
            ctry = miny + dimen.y / 2.0;
        } else {
            // Up.
            points = above(tp, hp, maxy, uminx, umaxx);
            ctry = maxy + dimen.y / 2.0 + 6.0;
            maxy += dimen.y + 6.0;
        }
        let Some(mut ps) = simpleSplineRoute(zz, tp, hp, &points, polyline) else {
            return;
        };
        if ps.is_empty() {
            return;
        }
        let l = &mut zz.textlabels[label];
        l.pos.x = ctrx;
        l.pos.y = ctry;
        l.set = 1;
        let head = aghead(zz, e);
        install(zz, e, head, &mut ps);
        i += 1;
    }

    // Edges with no labels.
    while i < cnt {
        let e = earray[i as usize];
        if i % 2 != 0 {
            // Down.
            if i == 1 {
                lminx = (2.0 * leftend + rightend) / 3.0;
                lmaxx = (leftend + 2.0 * rightend) / 3.0;
            }
            miny -= 6.0;
            points = below(tp, hp, miny, lminx, lmaxx);
        } else {
            // Up.
            points = above(tp, hp, maxy, uminx, umaxx);
            maxy += 6.0;
        }
        let Some(mut ps) = simpleSplineRoute(zz, tp, hp, &points, polyline) else {
            return;
        };
        if ps.is_empty() {
            return;
        }
        let head = aghead(zz, e);
        install(zz, e, head, &mut ps);
        i += 1;
    }
}

/// The polygon of a flat edge passing below the box `[lminx, lmaxx] × [miny, tp.y]`.
fn below(tp: pointf, hp: pointf, miny: f64, lminx: f64, lmaxx: f64) -> [pointf; 8] {
    [
        tp,
        pointfof(tp.x, miny - 6.0),
        pointfof(hp.x, miny - 6.0),
        hp,
        pointfof(lmaxx, hp.y),
        pointfof(lmaxx, miny),
        pointfof(lminx, miny),
        pointfof(lminx, tp.y),
    ]
}

/// The polygon of a flat edge passing above the box `[uminx, umaxx] × [tp.y, maxy]`.
fn above(tp: pointf, hp: pointf, maxy: f64, uminx: f64, umaxx: f64) -> [pointf; 8] {
    [
        tp,
        pointfof(uminx, tp.y),
        pointfof(uminx, maxy),
        pointfof(umaxx, maxy),
        pointfof(umaxx, hp.y),
        pointfof(hp.x, hp.y),
        pointfof(hp.x, maxy + 6.0),
        pointfof(tp.x, maxy + 6.0),
    ]
}

/// `makeSimpleFlat`: unlabeled flat edges between adjacent nodes, fanned out over the tail's height.
fn makeSimpleFlat(
    zz: &mut Globals,
    tn: NodeId,
    hn: NodeId,
    edges: &[EdgeId],
    ind: usize,
    cnt: i32,
    et: i32,
) {
    let e = edges[ind];
    let tp = add_pointf(coord(zz, tn), zz.ed(e).tail_port.p);
    let hp = add_pointf(coord(zz, hn), zz.ed(e).head_port.p);
    let ht = zz.nd(tn).ht;
    let stepy = if cnt > 1 {
        ht / f64::from(cnt - 1)
    } else {
        0.0
    };
    let mut dy = tp.y - if cnt > 1 { ht / 2.0 } else { 0.0 };
    for &e in &edges[ind..ind + cnt as usize] {
        if et != ET_SPLINE && et != ET_LINE {
            unimplemented!("makeSimpleFlat for splines=polyline");
        }
        let mut points = [
            tp,
            pointfof((2.0 * tp.x + hp.x) / 3.0, dy),
            pointfof((2.0 * hp.x + tp.x) / 3.0, dy),
            hp,
        ];
        dy += stepy;
        let head = aghead(zz, e);
        install(zz, e, head, &mut points);
    }
}

/// `make_flat_adj_edges`: flat edges between nodes next to each other in their rank.
fn make_flat_adj_edges(
    zz: &mut Globals,
    edges: &[EdgeId],
    ind: usize,
    cnt: i32,
    e0: EdgeId,
    et: i32,
) {
    let (tn, hn) = (agtail(zz, e0), aghead(zz, e0));
    let class = &edges[ind..ind + cnt as usize];
    let labels = class.iter().filter(|&&e| zz.ed(e).label.is_some()).count() as i32;
    let ports = class
        .iter()
        .any(|&e| zz.ed(e).tail_port.defined || zz.ed(e).head_port.defined);
    if ports {
        unimplemented!("flat adjacent edges with ports: dot on a rotated auxiliary graph");
    }
    if labels == 0 {
        makeSimpleFlat(zz, tn, hn, edges, ind, cnt, et);
    } else {
        makeSimpleFlatLabels(zz, tn, hn, edges, ind, cnt, et, labels);
    }
}

/// `makeFlatEnd`: the end boxes at node `n` of a flat edge leaving by the top.
#[allow(clippy::too_many_arguments, reason = "Graphviz's signature")]
fn makeFlatEnd(
    zz: &mut Globals,
    g: GraphId,
    sp: &spline_info_t,
    P: &mut path,
    n: NodeId,
    e: EdgeId,
    endp: &mut pathend_t,
    isBegin: bool,
) {
    let mut b = maximal_bbox(zz, g, sp, n, None, Some(e));
    endp.nb = b;
    endp.sidemask = TOP;
    if isBegin {
        beginpath(zz, P, e, FLATEDGE, endp, false);
    } else {
        endpath(zz, P, e, FLATEDGE, endp, false);
    }
    let last = endp.boxes[(endp.boxn - 1) as usize];
    b.UR.y = last.UR.y;
    b.LL.y = last.LL.y;
    let y = coord(zz, n).y + zz.rank(g, zz.nd(n).rank).ht2;
    let b = makeregularend(b, TOP, y);
    if b.LL.x < b.UR.x && b.LL.y < b.UR.y {
        unimplemented!("makeFlatEnd with room above the end box");
    }
}

/// `findLabelVnodeByAlg`, PlantUML's [FIX-flat-label]: the label node of flat edge `e`, found as the node of the
/// rank above whose `ND_alg` is `e` (as `flat_node` made it) rather than along `ED_to_virt`, which can end at a
/// real node for a labeled duplicate of an unlabeled flat edge.
fn findLabelVnodeByAlg(zz: &Globals, g: GraphId, e: EdgeId) -> Option<NodeId> {
    let r = zz.nd(agtail(zz, e)).rank - 1;
    if r < zz.gd(g).minrank {
        return None;
    }
    let v = rank_v(zz, g, r);
    (0..zz.rank(g, r).n)
        .filter_map(|i| zz.node_lists.get(v, i))
        .find(|&cand| zz.nd(cand).alg == Some(e))
}

/// `make_flat_labeled_edge`: a flat edge routed over its label node in the rank above.
fn make_flat_labeled_edge(
    zz: &mut Globals,
    g: GraphId,
    sp: &spline_info_t,
    P: &mut path,
    e: EdgeId,
    et: i32,
) {
    let (tn, hn) = (agtail(zz, e), aghead(zz, e));
    let ln = findLabelVnodeByAlg(zz, g, e).unwrap_or_else(|| {
        let mut f = zz
            .ed(e)
            .to_virt
            .expect("labeled flat edge without label node");
        while let Some(v) = zz.ed(f).to_virt {
            f = v;
        }
        agtail(zz, f)
    });
    let label = zz.ed(e).label.expect("labeled flat edge");
    let lc = coord(zz, ln);
    zz.textlabels[label].pos = lc;
    zz.textlabels[label].set = 1;
    if et == ET_LINE {
        unimplemented!("make_flat_labeled_edge for splines=line");
    }
    let lnd = *zz.nd(ln);
    let mut lb = boxf::default();
    lb.LL.x = lc.x - lnd.lw;
    lb.UR.x = lc.x + lnd.rw;
    lb.UR.y = lc.y + lnd.ht / 2.0;
    let trank = *zz.rank(g, zz.nd(tn).rank);
    let ydelta = (lc.y - trank.ht1 - coord(zz, tn).y + trank.ht2) as i32;
    let ydelta = (f64::from(ydelta) / 6.0) as i32;
    lb.LL.y = lb.UR.y - max(5.0, f64::from(ydelta));

    let mut tend = pathend_t::default();
    let mut hend = pathend_t::default();
    makeFlatEnd(zz, g, sp, P, tn, e, &mut tend, true);
    makeFlatEnd(zz, g, sp, P, hn, e, &mut hend, false);
    let tlast = tend.boxes[(tend.boxn - 1) as usize];
    let hlast = hend.boxes[(hend.boxn - 1) as usize];
    let boxes = [
        boxfof(tlast.LL.x, tlast.UR.y, lb.LL.x, lb.LL.y),
        boxfof(tlast.LL.x, lb.LL.y, hlast.UR.x, lb.UR.y),
        boxfof(lb.UR.x, hlast.UR.y, hlast.UR.x, lb.LL.y),
    ];
    add_end_and_boxes(P, &tend, &boxes, &hend);
    let Some(mut ps) = route(zz, P, et == ET_SPLINE) else {
        return;
    };
    let head = aghead(zz, e);
    install(zz, e, head, &mut ps);
}

/// Adds the tail end's boxes, then `boxes`, then the head end's boxes in reverse to `P`.
fn add_end_and_boxes(P: &mut path, tend: &pathend_t, boxes: &[boxf], hend: &pathend_t) {
    for &b in &tend.boxes[..tend.boxn as usize] {
        add_box(P, b);
    }
    for &b in boxes {
        add_box(P, b);
    }
    for &b in hend.boxes[..hend.boxn as usize].iter().rev() {
        add_box(P, b);
    }
}

/// `make_flat_edge`: the flat edges `edges[ind..ind + cnt]`, between adjacent nodes, with a label, or else
/// routed over the top in nested arcs.
#[allow(clippy::too_many_arguments, reason = "Graphviz's signature")]
fn make_flat_edge(
    zz: &mut Globals,
    g: GraphId,
    sp: &spline_info_t,
    P: &mut path,
    edges: &[EdgeId],
    ind: usize,
    cnt: i32,
    et: i32,
) {
    // Get sample edge; normalize to go from left to right.
    let mut e = edges[ind];
    let mut isAdjacent = zz.ed(e).adjacent != 0;
    if zz.ed(e).tree_index & BWDEDGE != 0 {
        let fwdedge = new_edge_pair(zz);
        MAKEFWDEDGE(zz, fwdedge, e);
        e = fwdedge;
    }
    // The lead edge might not have been marked adjacent, so check them all.
    if edges[ind + 1..ind + cnt as usize]
        .iter()
        .any(|&f| zz.ed(f).adjacent != 0)
    {
        isAdjacent = true;
    }
    if isAdjacent {
        make_flat_adj_edges(zz, edges, ind, cnt, e, et);
        return;
    }
    if zz.ed(e).label.is_some() {
        // Edges with labels aren't multi-edges.
        make_flat_labeled_edge(zz, g, sp, P, e, et);
        return;
    }
    if et == ET_LINE {
        let (tail, head) = (agtail(zz, e), aghead(zz, e));
        makeSimpleFlat(zz, tail, head, edges, ind, cnt, et);
        return;
    }
    let tside = zz.ed(e).tail_port.side;
    let hside = zz.ed(e).head_port.side;
    if (tside == BOTTOM && hside != TOP) || (hside == BOTTOM && tside != TOP) {
        unimplemented!("make_flat_bottom_edges");
    }

    let (tn, hn) = (agtail(zz, e), aghead(zz, e));
    let r = zz.nd(tn).rank;
    let vspace = if r > 0 {
        let prevr = if zz.gd(g).has_labels & EDGE_LABEL != 0 {
            r - 2
        } else {
            r - 1
        };
        let prev = *zz.rank(g, prevr);
        coord(zz, rank_node(zz, g, prevr, 0)).y - prev.ht1 - coord(zz, tn).y - zz.rank(g, r).ht2
    } else {
        f64::from(zz.gd(g).ranksep)
    };
    let stepx = f64::from(sp.Multisep) / f64::from(cnt + 1);
    let stepy = vspace / f64::from(cnt + 1);
    let mut tend = pathend_t::default();
    let mut hend = pathend_t::default();
    makeFlatEnd(zz, g, sp, P, tn, e, &mut tend, true);
    makeFlatEnd(zz, g, sp, P, hn, e, &mut hend, false);

    for (i, &e) in edges[ind..ind + cnt as usize].iter().enumerate() {
        let step = (i + 1) as f64;
        let tlast = tend.boxes[(tend.boxn - 1) as usize];
        let hlast = hend.boxes[(hend.boxn - 1) as usize];
        let b0 = boxfof(
            tlast.LL.x,
            tlast.UR.y,
            tlast.UR.x + step * stepx,
            tlast.UR.y + step * stepy,
        );
        let b1 = boxfof(tlast.LL.x, b0.UR.y, hlast.UR.x, b0.UR.y + stepy);
        let b2 = boxfof(hlast.LL.x - step * stepx, hlast.UR.y, hlast.UR.x, b1.LL.y);
        add_end_and_boxes(P, &tend, &[b0, b1, b2], &hend);
        let Some(mut ps) = route(zz, P, et == ET_SPLINE) else {
            return;
        };
        let head = aghead(zz, e);
        install(zz, e, head, &mut ps);
        P.nbox = 0;
    }
}

/// `make_regular_edge`: the edges `edges[ind..ind + cnt]` between different ranks, routed once through the
/// boxes along the first edge's chain (straight runs of virtual nodes become straight segments) and copied
/// `Multisep` apart for the others.
#[allow(clippy::too_many_arguments, reason = "Graphviz's signature")]
#[allow(clippy::too_many_lines, reason = "Graphviz's function")]
fn make_regular_edge(
    zz: &mut Globals,
    g: GraphId,
    sp: &mut spline_info_t,
    P: &mut path,
    edges: &[EdgeId],
    ind: usize,
    cnt: i32,
    et: i32,
) {
    let fwdedgea = new_edge_pair(zz);
    let fwdedgeb = new_edge_pair(zz);
    let mut sl = 0;
    let mut e = edges[ind];
    let mut hackflag = false;
    if (zz.nd(agtail(zz, e)).rank - zz.nd(aghead(zz, e)).rank).abs() > 1 {
        // A real edge spanning several ranks (a multi-edge whose chain belongs to another edge): route it as
        // the edge from its tail to the first node of that chain.
        *zz.ed_mut(fwdedgea) = *zz.ed(e);
        *zz.edge_mut(fwdedgea) = *zz.edge(e);
        if zz.ed(e).tree_index & BWDEDGE != 0 {
            MAKEFWDEDGE(zz, fwdedgeb, e);
            let head = aghead(zz, e);
            M_agtail(zz, fwdedgea, head);
            zz.ed_mut(fwdedgea).tail_port = zz.ed(e).head_port;
        } else {
            *zz.ed_mut(fwdedgeb) = *zz.ed(e);
            *zz.edge_mut(fwdedgeb) = *zz.edge(e);
            let tail = agtail(zz, e);
            M_agtail(zz, fwdedgea, tail);
        }
        let mut le = getmainedge(zz, e);
        while let Some(v) = zz.ed(le).to_virt {
            le = v;
        }
        let head = aghead(zz, le);
        M_aghead(zz, fwdedgea, head);
        let fa = zz.ed_mut(fwdedgea);
        fa.head_port.defined = false;
        fa.edge_type = VIRTUAL;
        fa.head_port.p.x = 0.0;
        fa.head_port.p.y = 0.0;
        fa.to_orig = Some(e);
        e = fwdedgea;
        hackflag = true;
    } else if zz.ed(e).tree_index & BWDEDGE != 0 {
        MAKEFWDEDGE(zz, fwdedgea, e);
        e = fwdedgea;
    }
    let fe = e;

    // Compute the spline points for the edge.
    if et == ET_LINE {
        unimplemented!("makeLineEdge");
    }
    let splines = et == ET_SPLINE;
    let mut boxes: Vec<boxf> = Vec::new();
    let mut pointfs: Vec<pointf> = Vec::new();
    let mut segfirst = e;
    let mut tn = agtail(zz, e);
    let mut hn = aghead(zz, e);
    let mut tend = pathend_t::default();
    let mut hend = pathend_t::default();
    let mut b = maximal_bbox(zz, g, sp, tn, None, Some(e));
    tend.nb = b;
    beginpath(zz, P, e, REGULAREDGE, &mut tend, spline_merge(zz, tn));
    let last = tend.boxes[(tend.boxn - 1) as usize];
    b.UR.y = last.UR.y;
    b.LL.y = last.LL.y;
    let b = makeregularend(b, BOTTOM, coord(zz, tn).y - zz.rank(g, zz.nd(tn).rank).ht1);
    if b.LL.x < b.UR.x && b.LL.y < b.UR.y {
        tend.boxes[tend.boxn as usize] = b;
        tend.boxn += 1;
    }
    let mut smode = false;
    let mut si = -1;
    while zz.nd(hn).node_type == VIRTUAL && !spline_merge(zz, hn) {
        boxes.push(rank_box(zz, sp, g, zz.nd(tn).rank));
        if !smode {
            sl = straight_len(zz, hn);
            let min_len = if zz.gd(g).has_labels & EDGE_LABEL != 0 {
                4 + 1
            } else {
                2 + 1
            };
            if sl >= min_len {
                smode = true;
                si = 1;
                sl -= 2;
            }
        }
        if !smode || si > 0 {
            si -= 1;
            let next = out0(zz, hn);
            boxes.push(maximal_bbox(zz, g, sp, hn, Some(e), Some(next)));
            e = next;
            tn = agtail(zz, e);
            hn = aghead(zz, e);
            continue;
        }
        let next = out0(zz, hn);
        hend.nb = maximal_bbox(zz, g, sp, hn, Some(e), Some(next));
        let merge = spline_merge(zz, aghead(zz, e));
        endpath(zz, P, e, REGULAREDGE, &mut hend, merge);
        let y = coord(zz, hn).y + zz.rank(g, zz.nd(hn).rank).ht2;
        let b = makeregularend(hend.boxes[(hend.boxn - 1) as usize], TOP, y);
        if b.LL.x < b.UR.x && b.LL.y < b.UR.y {
            unimplemented!("room above the head end of a straight run");
        }
        P.end.theta = M_PI / 2.0;
        P.end.constrained = true;
        completeregularpath(zz, P, segfirst, e, &tend, &hend, &boxes);
        if !splines {
            unimplemented!("polyline through a straight run");
        }
        let Some(ps) = route(zz, P, true) else {
            return;
        };
        pointfs.extend_from_slice(&ps);
        e = straight_path(zz, out0(zz, hn), sl, &mut pointfs);
        recover_slack(zz, segfirst, P);
        segfirst = e;
        tn = agtail(zz, e);
        hn = aghead(zz, e);
        boxes.clear();
        tend.nb = maximal_bbox(zz, g, sp, tn, in0(zz, tn), Some(e));
        beginpath(zz, P, e, REGULAREDGE, &mut tend, spline_merge(zz, tn));
        let y = coord(zz, tn).y - zz.rank(g, zz.nd(tn).rank).ht1;
        let b = makeregularend(tend.boxes[(tend.boxn - 1) as usize], BOTTOM, y);
        if b.LL.x < b.UR.x && b.LL.y < b.UR.y {
            unimplemented!("room below the tail end of a straight run");
        }
        P.start.theta = -M_PI / 2.0;
        P.start.constrained = true;
        smode = false;
    }
    boxes.push(rank_box(zz, sp, g, zz.nd(tn).rank));
    let mut b = maximal_bbox(zz, g, sp, hn, Some(e), None);
    hend.nb = b;
    let merge = spline_merge(zz, aghead(zz, e));
    endpath(
        zz,
        P,
        if hackflag { fwdedgeb } else { e },
        REGULAREDGE,
        &mut hend,
        merge,
    );
    let last = hend.boxes[(hend.boxn - 1) as usize];
    b.UR.y = last.UR.y;
    b.LL.y = last.LL.y;
    let b = makeregularend(b, TOP, coord(zz, hn).y + zz.rank(g, zz.nd(hn).rank).ht2);
    if b.LL.x < b.UR.x && b.LL.y < b.UR.y {
        hend.boxes[hend.boxn as usize] = b;
        hend.boxn += 1;
    }
    completeregularpath(zz, P, segfirst, e, &tend, &hend, &boxes);
    let Some(ps) = route(zz, P, splines) else {
        return;
    };
    pointfs.extend_from_slice(&ps);
    recover_slack(zz, segfirst, P);
    let hn = if hackflag {
        aghead(zz, fwdedgeb)
    } else {
        aghead(zz, e)
    };

    // Make copies of the spline points, one per multi-edge.
    if cnt == 1 {
        install(zz, fe, hn, &mut pointfs);
        return;
    }
    let pointn = pointfs.len();
    let dx = sp.Multisep * (cnt - 1) / 2;
    for p in &mut pointfs[1..pointn - 1] {
        p.x -= f64::from(dx);
    }
    let mut pointfs2 = pointfs.clone();
    install(zz, fe, hn, &mut pointfs2);
    let fwdedge = new_edge_pair(zz);
    for &e in &edges[ind + 1..ind + cnt as usize] {
        let e = if zz.ed(e).tree_index & BWDEDGE != 0 {
            MAKEFWDEDGE(zz, fwdedge, e);
            fwdedge
        } else {
            e
        };
        for p in &mut pointfs[1..pointn - 1] {
            p.x += f64::from(sp.Multisep);
        }
        pointfs2.copy_from_slice(&pointfs);
        let head = aghead(zz, e);
        install(zz, e, head, &mut pointfs2);
    }
}

/// `completeregularpath`: the path's boxes: the tail end's, `boxes`, then the head end's, adjusted so that
/// the spline has room to pass between ranks.
#[allow(clippy::too_many_arguments, reason = "Graphviz's signature")]
fn completeregularpath(
    zz: &Globals,
    P: &mut path,
    first: EdgeId,
    last: EdgeId,
    tendp: &pathend_t,
    hendp: &pathend_t,
    boxes: &[boxf],
) {
    let neighbours = [
        top_bound(zz, first, -1),
        top_bound(zz, first, 1),
        bot_bound(zz, last, -1),
        bot_bound(zz, last, 1),
    ];
    for f in neighbours.into_iter().flatten() {
        if getsplinepoints(zz, f).is_none() {
            unimplemented!("getsplinepoints: no spline points available");
        }
    }
    for &b in &tendp.boxes[..tendp.boxn as usize] {
        add_box(P, b);
    }
    let fb = P.nbox + 1;
    let lb = fb + boxes.len() as i32 - 3;
    for &b in boxes {
        add_box(P, b);
    }
    for &b in hendp.boxes[..hendp.boxn as usize].iter().rev() {
        add_box(P, b);
    }
    adjustregularpath(P, fb, lb);
}

/// `makeregularend`: the box between `b`'s `side` and height `y`.
fn makeregularend(b: boxf, side: i32, y: f64) -> boxf {
    match side {
        BOTTOM => boxfof(b.LL.x, y, b.UR.x, b.LL.y),
        TOP => boxfof(b.LL.x, b.UR.y, b.UR.x, y),
        _ => boxf::default(),
    }
}

/// `adjustregularpath`: widens the boxes `fb - 1..=lb` to at least `MINW` and makes consecutive boxes overlap by
/// that much.
fn adjustregularpath(P: &mut path, fb: i32, lb: i32) {
    for i in fb - 1..=lb {
        let bp1 = &mut P.boxes[i as usize];
        let narrow = if (i - fb) % 2 == 0 {
            bp1.LL.x >= bp1.UR.x
        } else {
            bp1.LL.x + f64::from(MINW) > bp1.UR.x
        };
        if narrow {
            let x = f64::from(((bp1.LL.x + bp1.UR.x) / 2.0) as i32);
            bp1.LL.x = x - HALFMINW;
            bp1.UR.x = x + HALFMINW;
        }
    }
    let minw = f64::from(MINW);
    for i in 0..(P.nbox - 1).max(0) {
        let (bp1, bp2) = (P.boxes[i as usize], P.boxes[i as usize + 1]);
        if i >= fb && i <= lb && (i - fb) % 2 == 0 {
            let bp2 = &mut P.boxes[i as usize + 1];
            if bp1.LL.x + minw > bp2.UR.x {
                bp2.UR.x = bp1.LL.x + minw;
            }
            if bp1.UR.x - minw < bp2.LL.x {
                bp2.LL.x = bp1.UR.x - minw;
            }
        } else if i + 1 >= fb && i < lb && (i + 1 - fb) % 2 == 0 {
            let bp1 = &mut P.boxes[i as usize];
            if bp1.LL.x + minw > bp2.UR.x {
                bp1.LL.x = bp2.UR.x - minw;
            }
            if bp1.UR.x - minw < bp2.LL.x {
                bp1.UR.x = bp2.LL.x + minw;
            }
        }
    }
}

/// `rank_box`: the box between ranks `r` and `r + 1`, across the whole graph.
fn rank_box(zz: &Globals, sp: &mut spline_info_t, g: GraphId, r: i32) -> boxf {
    let mut b = sp.Rank_box[r as usize];
    if b.LL.x == b.UR.x {
        let left0 = rank_node(zz, g, r, 0);
        let left1 = rank_node(zz, g, r + 1, 0);
        b.LL.x = f64::from(sp.LeftBound);
        b.LL.y = coord(zz, left1).y + zz.rank(g, r + 1).ht2;
        b.UR.x = f64::from(sp.RightBound);
        b.UR.y = coord(zz, left0).y - zz.rank(g, r).ht1;
        sp.Rank_box[r as usize] = b;
    }
    b
}

/// `straight_len`: how many virtual nodes below `n` continue its chain straight down.
fn straight_len(zz: &Globals, n: NodeId) -> i32 {
    let mut cnt = 0;
    let mut v = n;
    loop {
        v = aghead(zz, out0(zz, v));
        let nd = zz.nd(v);
        if nd.node_type != VIRTUAL
            || nd.out.size != 1
            || nd.in_.size != 1
            || nd.coord.x != coord(zz, n).x
        {
            break;
        }
        cnt += 1;
    }
    cnt
}

/// `straight_path`: continues the spline `plist` straight down `cnt` edges of the chain from `e`; returns the
/// edge after them.
fn straight_path(zz: &Globals, e: EdgeId, cnt: i32, plist: &mut Vec<pointf>) -> EdgeId {
    let mut f = e;
    for _ in 0..cnt {
        f = out0(zz, aghead(zz, f));
    }
    let last = *plist.last().expect("spline points");
    plist.push(last);
    plist.push(last);
    // C also stores the tail of `f` after them, which the next spline's first point overwrites.
    f
}

/// `recover_slack`: shrinks the virtual nodes of `e`'s chain to the boxes the spline took, giving the room back.
fn recover_slack(zz: &mut Globals, e: EdgeId, p: &path) {
    // Skip the first rank box.
    let mut b = 0;
    let mut vn = aghead(zz, e);
    while zz.nd(vn).node_type == VIRTUAL && !spline_merge(zz, vn) {
        let y = coord(zz, vn).y;
        while b < p.nbox && p.boxes[b as usize].LL.y > y {
            b += 1;
        }
        if b >= p.nbox {
            break;
        }
        let bx = p.boxes[b as usize];
        if bx.UR.y >= y {
            if zz.nd(vn).label.is_some() {
                let rx = (bx.UR.x + zz.nd(vn).rw) as i32;
                resize_vn(zz, vn, bx.LL.x as i32, bx.UR.x as i32, rx);
            } else {
                resize_vn(
                    zz,
                    vn,
                    bx.LL.x as i32,
                    ((bx.LL.x + bx.UR.x) / 2.0) as i32,
                    bx.UR.x as i32,
                );
            }
        }
        vn = aghead(zz, out0(zz, vn));
    }
}

/// `resize_vn`.
fn resize_vn(zz: &mut Globals, vn: NodeId, lx: i32, cx: i32, rx: i32) {
    let nd = zz.nd_mut(vn);
    nd.coord.x = f64::from(cx);
    nd.lw = f64::from(cx - lx);
    nd.rw = f64::from(rx - cx);
}

/// `top_bound`: the nearest out-edge of `e`'s tail on `side` of `e` that already has a spline.
fn top_bound(zz: &Globals, e: EdgeId, side: i32) -> Option<EdgeId> {
    let tail = agtail(zz, e);
    let order = zz.nd(aghead(zz, e)).order;
    let mut ans: Option<EdgeId> = None;
    let mut i = 0;
    while let Some(f) = zz.nd(tail).out.get(&zz.edge_lists, i) {
        i += 1;
        let forder = zz.nd(aghead(zz, f)).order;
        if side * (forder - order) <= 0 || !has_spline(zz, f) {
            continue;
        }
        if ans.is_none_or(|a| side * (zz.nd(aghead(zz, a)).order - forder) > 0) {
            ans = Some(f);
        }
    }
    ans
}

/// `bot_bound`: the nearest in-edge of `e`'s head on `side` of `e` that already has a spline.
fn bot_bound(zz: &Globals, e: EdgeId, side: i32) -> Option<EdgeId> {
    let head = aghead(zz, e);
    let order = zz.nd(agtail(zz, e)).order;
    let mut ans: Option<EdgeId> = None;
    let mut i = 0;
    while let Some(f) = zz.nd(head).in_.get(&zz.edge_lists, i) {
        i += 1;
        let forder = zz.nd(agtail(zz, f)).order;
        if side * (forder - order) <= 0 || !has_spline(zz, f) {
            continue;
        }
        if ans.is_none_or(|a| side * (zz.nd(agtail(zz, a)).order - forder) > 0) {
            ans = Some(f);
        }
    }
    ans
}

/// Whether `f` or the edge it stands for has a spline.
fn has_spline(zz: &Globals, f: EdgeId) -> bool {
    zz.ed(f).spl.is_some() || zz.ed(f).to_orig.is_some_and(|o| zz.ed(o).spl.is_some())
}

/// `cl_vninside`: whether node `n` lies in cluster `cl`'s box.
fn cl_vninside(zz: &Globals, cl: GraphId, n: NodeId) -> bool {
    let bb = zz.gd(cl).bb;
    let c = coord(zz, n);
    BETWEEN(bb.LL.x, c.x, bb.UR.x) && BETWEEN(bb.LL.y, c.y, bb.UR.y)
}

/// `cl_bound`: the cluster neighbour `adj` of `n` belongs to, if `n`'s edge does not belong to it too.
fn cl_bound(zz: &Globals, g: GraphId, n: NodeId, adj: NodeId) -> Option<GraphId> {
    let real_ends = |v: NodeId| {
        let orig = zz
            .ed(out0(zz, v))
            .to_orig
            .expect("virtual edge without original");
        (agtail(zz, orig), aghead(zz, orig))
    };
    let (tcl, hcl) = if zz.nd(n).node_type == NORMAL {
        (zz.nd(n).clust, zz.nd(n).clust)
    } else {
        let (t, h) = real_ends(n);
        (zz.nd(t).clust, zz.nd(h).clust)
    };
    let below_root = |v: NodeId| zz.nd(v).clust.filter(|&c| c != g);
    let foreign = |cl: Option<GraphId>| cl.is_some() && cl != tcl && cl != hcl;
    if zz.nd(adj).node_type == NORMAL {
        let cl = below_root(adj);
        return if foreign(cl) { cl } else { None };
    }
    let (t, h) = real_ends(adj);
    let cl = below_root(t);
    if foreign(cl) && cl_vninside(zz, cl?, adj) {
        return cl;
    }
    let cl = below_root(h);
    if foreign(cl) && cl_vninside(zz, cl?, adj) {
        return cl;
    }
    None
}

/// `maximal_bbox`: the widest box around `vn` in its rank that stays clear of its neighbours (clusters, real
/// nodes, labels and edges it would cross).
fn maximal_bbox(
    zz: &Globals,
    g: GraphId,
    sp: &spline_info_t,
    vn: NodeId,
    ie: Option<EdgeId>,
    oe: Option<EdgeId>,
) -> boxf {
    let nd = *zz.nd(vn);
    let nodesep = f64::from(zz.gd(g).nodesep);
    let mut rv = boxf::default();

    // Give this node all the available space up to its neighbors.
    let mut b = nd.coord.x - nd.lw - FUDGE;
    if let Some(left) = neighbor(zz, g, vn, ie, oe, -1) {
        let nb = if let Some(left_cl) = cl_bound(zz, g, vn, left) {
            zz.gd(left_cl).bb.UR.x + f64::from(sp.Splinesep)
        } else {
            let nb = coord(zz, left).x + zz.nd(left).mval;
            if zz.nd(left).node_type == NORMAL {
                nb + nodesep / 2.0
            } else {
                nb + f64::from(sp.Splinesep)
            }
        };
        if nb < b {
            b = nb;
        }
        rv.LL.x = f64::from(ROUND(b));
    } else {
        rv.LL.x = f64::from(ROUND(b).min(sp.LeftBound));
    }

    // We have to leave room for our own label!
    let has_label = nd.node_type == VIRTUAL && nd.label.is_some();
    let mut b = if has_label {
        nd.coord.x + 10.0
    } else {
        nd.coord.x + nd.rw + FUDGE
    };
    if let Some(right) = neighbor(zz, g, vn, ie, oe, 1) {
        let nb = if let Some(right_cl) = cl_bound(zz, g, vn, right) {
            zz.gd(right_cl).bb.LL.x - f64::from(sp.Splinesep)
        } else {
            let nb = coord(zz, right).x - zz.nd(right).lw;
            if zz.nd(right).node_type == NORMAL {
                nb - nodesep / 2.0
            } else {
                nb - f64::from(sp.Splinesep)
            }
        };
        if nb > b {
            b = nb;
        }
        rv.UR.x = f64::from(ROUND(b));
    } else {
        rv.UR.x = f64::from(ROUND(b).max(sp.RightBound));
    }

    if has_label {
        rv.UR.x -= nd.rw;
        if rv.UR.x < rv.LL.x {
            rv.UR.x = nd.coord.x;
        }
    }

    let rank = zz.rank(g, nd.rank);
    rv.LL.y = nd.coord.y - rank.ht1;
    rv.UR.y = nd.coord.y + rank.ht2;
    rv
}

/// `neighbor`: the nearest node of `vn`'s rank in direction `dir` that the spline must keep clear of.
fn neighbor(
    zz: &Globals,
    g: GraphId,
    vn: NodeId,
    ie: Option<EdgeId>,
    oe: Option<EdgeId>,
    dir: i32,
) -> Option<NodeId> {
    let rank = *zz.rank(g, zz.nd(vn).rank);
    let v = rank.v.expect("rank without nodes");
    let mut i = zz.nd(vn).order + dir;
    while i >= 0 && i < rank.n {
        let n = zz.node_lists.get(v, i).expect("node in rank");
        let nd = zz.nd(n);
        if (nd.node_type == VIRTUAL && nd.label.is_some())
            || nd.node_type == NORMAL
            || !pathscross(zz, n, vn, ie, oe)
        {
            return Some(n);
        }
        i += dir;
    }
    None
}

/// `pathscross`: whether the chains through `n0` and `n1` cross within two ranks up or down.
fn pathscross(
    zz: &Globals,
    n0: NodeId,
    n1: NodeId,
    ie1: Option<EdgeId>,
    oe1: Option<EdgeId>,
) -> bool {
    let order = zz.nd(n0).order > zz.nd(n1).order;
    if zz.nd(n0).out.size != 1 && zz.nd(n0).out.size != 1 {
        return false;
    }
    if let Some(mut e1) = oe1
        && zz.nd(n0).out.size == 1
    {
        let mut e0 = out0(zz, n0);
        for _ in 0..2 {
            let (na, nb) = (aghead(zz, e0), aghead(zz, e1));
            if na == nb {
                break;
            }
            if order != (zz.nd(na).order > zz.nd(nb).order) {
                return true;
            }
            if zz.nd(na).out.size != 1 || zz.nd(na).node_type == NORMAL {
                break;
            }
            e0 = out0(zz, na);
            if zz.nd(nb).out.size != 1 || zz.nd(nb).node_type == NORMAL {
                break;
            }
            e1 = out0(zz, nb);
        }
    }
    if let Some(mut e1) = ie1
        && zz.nd(n0).in_.size == 1
    {
        let mut e0 = in0(zz, n0).expect("node without in-edge");
        for _ in 0..2 {
            let (na, nb) = (agtail(zz, e0), agtail(zz, e1));
            if na == nb {
                break;
            }
            if order != (zz.nd(na).order > zz.nd(nb).order) {
                return true;
            }
            if zz.nd(na).in_.size != 1 || zz.nd(na).node_type == NORMAL {
                break;
            }
            e0 = in0(zz, na).expect("node without in-edge");
            if zz.nd(nb).in_.size != 1 || zz.nd(nb).node_type == NORMAL {
                break;
            }
            e1 = in0(zz, nb).expect("node without in-edge");
        }
    }
    false
}

//! `postproc.c`: what follows the dot phases. Places the cluster labels, places head and tail labels (and edge
//! labels dot left unplaced) with xlabels, then moves the drawing so that its bounding box starts at the origin,
//! rotating it for `rankdir=LR`.

use crate::cgraph::AGRAPH;
use crate::cgraph::attr::agattr;
use crate::cgraph::edge::{agfstout, agnxtout};
use crate::cgraph::graph::agnnodes;
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::obj::agroot;
use crate::common::geom::ccwrotatepf;
use crate::common::splines::{edgeMidpoint, getsplinepoints};
use crate::common::utils::{gv_nodesize, late_bool, updateBB};
use crate::core::Globals;
use crate::core::consts::{
    EDGE_LABEL, EDGE_XLABEL, ET_NONE, GRAPH_LABEL, GVSPLINES, HEAD_LABEL, INT_MAX, LABEL_AT_LEFT,
    LABEL_AT_RIGHT, LABEL_AT_TOP, LEFT_IX, NODE_XLABEL, RANKDIR_BT, RANKDIR_LR, RANKDIR_RL,
    RANKDIR_TB, RIGHT_IX, TAIL_LABEL, TOP_IX,
};
use crate::core::ids::{EdgeId, GraphId, NodeId, TextlabelId};
use crate::core::jmath::{INCH2PS, max, min};
use crate::h::{boxf, pointf, pointfof};
use crate::label::{label_params_t, object_t, placeLabels, xlabel_t};

/// `map_point`: a point of the layout in the final drawing.
fn map_point(zz: &Globals, p: pointf) -> pointf {
    let mut p = ccwrotatepf(p, zz.Rankdir * 90);
    p.x -= zz.Offset.x;
    p.y -= zz.Offset.y;
    p
}

/// `map_edge`: moves an edge's splines and labels.
fn map_edge(zz: &mut Globals, e: EdgeId) {
    let Some(spl) = zz.ed(e).spl else {
        // Graphviz warns about a lost edge here (unless it was ignored); the engine does no I/O.
        return;
    };
    let spl = zz.splines[spl];
    for j in 0..spl.size {
        let bz = zz.beziers.get(spl.list.expect("spline list"), j);
        let list = bz.list.expect("bezier points");
        for k in 0..bz.size {
            zz.pointfs[list.at(k)] = map_point(zz, zz.pointfs.get(list, k));
        }
        let beziers = spl.list.expect("spline list");
        if bz.sflag != 0 {
            zz.beziers[beziers.at(j)].sp = map_point(zz, zz.beziers[beziers.at(j)].sp);
        }
        if bz.eflag != 0 {
            zz.beziers[beziers.at(j)].ep = map_point(zz, zz.beziers[beziers.at(j)].ep);
        }
    }
    if let Some(l) = zz.ed(e).label {
        zz.textlabels[l].pos = map_point(zz, zz.textlabels[l].pos);
    }
    if zz.ed(e).xlabel.is_some() {
        unimplemented!("map_edge with an xlabel");
    }
    if let Some(l) = zz.ed(e).head_label {
        zz.textlabels[l].pos = map_point(zz, zz.textlabels[l].pos);
    }
    if let Some(l) = zz.ed(e).tail_label {
        zz.textlabels[l].pos = map_point(zz, zz.textlabels[l].pos);
    }
}

/// `translate_bb`: moves the bounding boxes and labels of `g` and its clusters.
fn translate_bb(zz: &mut Globals, g: GraphId, rankdir: i32) {
    let bb = zz.gd(g).bb;
    let new_bb = if rankdir == RANKDIR_LR || rankdir == RANKDIR_BT {
        boxf {
            LL: map_point(zz, pointfof(bb.LL.x, bb.UR.y)),
            UR: map_point(zz, pointfof(bb.UR.x, bb.LL.y)),
        }
    } else {
        boxf {
            LL: map_point(zz, pointfof(bb.LL.x, bb.LL.y)),
            UR: map_point(zz, pointfof(bb.UR.x, bb.UR.y)),
        }
    };
    zz.gd_mut(g).bb = new_bb;
    if let Some(l) = zz.gd(g).label {
        zz.textlabels[l].pos = map_point(zz, zz.textlabels[l].pos);
    }
    for c in 1..=zz.gd(g).n_cluster {
        translate_bb(zz, GD_clust(zz, g, c), rankdir);
    }
}

/// `GD_clust(g)[c]`.
fn GD_clust(zz: &Globals, g: GraphId, c: i32) -> GraphId {
    zz.graph_lists
        .get(zz.gd(g).clust.expect("GD_clust"), c)
        .expect("cluster")
}

/// `translate_drawing`: moves (and for LR, rotates) nodes, edges and boxes by `Offset`.
fn translate_drawing(zz: &mut Globals, g: GraphId) {
    let shift = zz.Offset.x != 0.0 || zz.Offset.y != 0.0;
    if !shift && zz.Rankdir == 0 {
        return;
    }
    let mut v = agfstnode(zz, g);
    while let Some(vv) = v {
        if zz.Rankdir != 0 {
            gv_nodesize(zz, vv, false);
        }
        zz.nd_mut(vv).coord = map_point(zz, zz.nd(vv).coord);
        if zz.nd(vv).xlabel.is_some() {
            unimplemented!("translate_drawing with a node xlabel");
        }
        if zz.State == GVSPLINES {
            let mut e = agfstout(zz, g, vv);
            while let Some(ee) = e {
                map_edge(zz, ee);
                e = agnxtout(zz, g, ee);
            }
        }
        v = agnxtnode(zz, g, vv);
    }
    translate_bb(zz, g, zz.gd(g).GD_rankdir());
}

/// `centerPt`: the centre of a placed label.
fn centerPt(xlp: &xlabel_t) -> pointf {
    let mut p = xlp.pos;
    p.x += xlp.sz.x / 2.0;
    p.y += xlp.sz.y / 2.0;
    p
}

/// `edgeTailpoint`: where the edge's drawing starts.
fn edgeTailpoint(zz: &Globals, e: EdgeId) -> pointf {
    let spl = zz.splines[getsplinepoints(zz, e).expect("getsplinepoints: no spline points")];
    let bez = zz.beziers.get(spl.list.expect("spline list"), 0);
    if bez.sflag != 0 {
        bez.sp
    } else {
        zz.pointfs.get(bez.list.expect("bezier points"), 0)
    }
}

/// `edgeHeadpoint`: where the edge's drawing ends.
fn edgeHeadpoint(zz: &Globals, e: EdgeId) -> pointf {
    let spl = zz.splines[getsplinepoints(zz, e).expect("getsplinepoints: no spline points")];
    let bez = zz.beziers.get(spl.list.expect("spline list"), spl.size - 1);
    if bez.eflag != 0 {
        bez.ep
    } else {
        zz.pointfs
            .get(bez.list.expect("bezier points"), bez.size - 1)
    }
}

/// `adjustBB`: `bb` grown to contain the object.
fn adjustBB(objp: &object_t, mut bb: boxf) -> boxf {
    bb.LL.x = min(bb.LL.x, objp.pos.x);
    bb.LL.y = min(bb.LL.y, objp.pos.y);
    let ur = pointf {
        x: objp.pos.x + objp.sz.x,
        y: objp.pos.y + objp.sz.y,
    };
    bb.UR.x = max(bb.UR.x, ur.x);
    bb.UR.y = max(bb.UR.y, ur.y);
    bb
}

/// `addXLabel`: makes `lbls[xlp]` the label to place for `lp`; with `initObj`, `objp` becomes the point `pos`.
fn addXLabel(
    zz: &Globals,
    lp: TextlabelId,
    objp: &mut object_t,
    lbls: &mut [xlabel_t],
    xlp: usize,
    initObj: bool,
    pos: pointf,
) {
    if initObj {
        objp.sz.x = 0.0;
        objp.sz.y = 0.0;
        objp.pos = pos;
    }
    let dimen = zz.textlabels[lp].dimen;
    if zz.Flip {
        lbls[xlp].sz.x = dimen.y;
        lbls[xlp].sz.y = dimen.x;
    } else {
        lbls[xlp].sz = dimen;
    }
    lbls[xlp].lbl = Some(lp);
    lbls[xlp].set = 0;
    objp.lbl = Some(xlp);
}

/// `addLabelObj`: makes `objp` the obstacle a placed label is.
fn addLabelObj(zz: &Globals, lp: TextlabelId, objp: &mut object_t, bb: boxf) -> boxf {
    let lp = &zz.textlabels[lp];
    if zz.Flip {
        objp.sz.x = lp.dimen.y;
        objp.sz.y = lp.dimen.x;
    } else {
        objp.sz.x = lp.dimen.x;
        objp.sz.y = lp.dimen.y;
    }
    objp.pos = lp.pos;
    objp.pos.x -= objp.sz.x / 2.0;
    objp.pos.y -= objp.sz.y / 2.0;
    adjustBB(objp, bb)
}

/// `addNodeObj`: makes `objp` the obstacle a node is.
fn addNodeObj(zz: &Globals, np: NodeId, objp: &mut object_t, bb: boxf) -> boxf {
    let nd = zz.nd(np);
    if zz.Flip {
        objp.sz.x = INCH2PS(nd.height);
        objp.sz.y = INCH2PS(nd.width);
    } else {
        objp.sz.x = INCH2PS(nd.width);
        objp.sz.y = INCH2PS(nd.height);
    }
    objp.pos = nd.coord;
    objp.pos.x -= objp.sz.x / 2.0;
    objp.pos.y -= objp.sz.y / 2.0;
    adjustBB(objp, bb)
}

/// `cinfo_t`: the bounding box so far and the next free object.
#[derive(Clone, Copy)]
struct cinfo_t {
    bb: boxf,
    objp: usize,
}

/// `addClusterObj`: adds the placed labels of the clusters below `g` (depth first, the cluster's own last) as
/// obstacles.
fn addClusterObj(
    zz: &mut Globals,
    g: GraphId,
    mut info: cinfo_t,
    objs: &mut [object_t],
) -> cinfo_t {
    for c in 1..=zz.gd(g).n_cluster {
        info = addClusterObj(zz, GD_clust(zz, g, c), info, objs);
    }
    if g != agroot(zz, g)
        && let Some(l) = zz.gd(g).label
        && zz.textlabels[l].set != 0
    {
        info.bb = addLabelObj(zz, l, &mut objs[info.objp], info.bb);
        info.objp += 1;
    }
    info
}

/// `countClusterLabels`: the placed labels of `g`'s clusters, at any depth.
fn countClusterLabels(zz: &Globals, g: GraphId) -> i32 {
    let mut i = 0;
    if g != agroot(zz, g)
        && let Some(l) = zz.gd(g).label
        && zz.textlabels[l].set != 0
    {
        i += 1;
    }
    for c in 1..=zz.gd(g).n_cluster {
        i += countClusterLabels(zz, GD_clust(zz, g, c));
    }
    i
}

/// `HAVE_EDGE`: whether the edge was drawn.
fn HAVE_EDGE(zz: &Globals, ep: EdgeId, et: i32) -> bool {
    et != ET_NONE && zz.ed(ep).spl.is_some()
}

/// The labels of an edge that `addXLabels` looks at, and how it counts them.
fn count_edge_label(
    zz: &Globals,
    lp: Option<TextlabelId>,
    have_edge: bool,
    set: &mut i32,
    unset: &mut i32,
) {
    if let Some(lp) = lp {
        if zz.textlabels[lp].set != 0 {
            *set += 1;
        } else if have_edge {
            *unset += 1;
        }
    }
}

/// `addXLabels`: places the labels dot has not placed (head and tail labels, and edge labels dot could not put
/// on a virtual node) with xlabels, around the nodes and the labels already placed.
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
fn addXLabels(zz: &mut Globals, gp: GraphId) {
    let n_nlbls = 0;
    let mut n_elbls = 0;
    let mut n_set_lbls = 0;
    let mut n_clbls = 0;
    let et = zz.gd(gp).flags & (7 << 1);
    let has_labels = zz.gd(gp).has_labels;

    if (has_labels & NODE_XLABEL) == 0
        && (has_labels & EDGE_XLABEL) == 0
        && (has_labels & TAIL_LABEL) == 0
        && (has_labels & HEAD_LABEL) == 0
        && ((has_labels & EDGE_LABEL) == 0 || zz.EdgeLabelsDone != 0)
    {
        return;
    }

    let mut np = agfstnode(zz, gp);
    while let Some(n) = np {
        if zz.nd(n).xlabel.is_some() {
            unimplemented!("addXLabels with node xlabels");
        }
        let mut ep = agfstout(zz, gp, n);
        while let Some(e) = ep {
            if zz.ed(e).xlabel.is_some() {
                unimplemented!("addXLabels with edge xlabels");
            }
            let have_edge = HAVE_EDGE(zz, e, et);
            let ed = *zz.ed(e);
            count_edge_label(zz, ed.head_label, have_edge, &mut n_set_lbls, &mut n_elbls);
            count_edge_label(zz, ed.tail_label, have_edge, &mut n_set_lbls, &mut n_elbls);
            count_edge_label(zz, ed.label, have_edge, &mut n_set_lbls, &mut n_elbls);
            ep = agnxtout(zz, gp, e);
        }
        np = agnxtnode(zz, gp, n);
    }
    if (has_labels & GRAPH_LABEL) != 0 {
        n_clbls = countClusterLabels(zz, gp);
    }

    let n_lbls = n_nlbls + n_elbls;
    if n_lbls == 0 {
        return;
    }

    let n_objs = agnnodes(zz, gp) + n_set_lbls + n_clbls + n_elbls;
    let mut objs = vec![object_t::default(); n_objs as usize];
    let mut objp = 0;
    let mut lbls = vec![xlabel_t::default(); n_lbls as usize];
    let mut xlp = 0;
    let mut bb = boxf {
        LL: pointfof(f64::from(INT_MAX), f64::from(INT_MAX)),
        UR: pointfof(f64::from(-INT_MAX), f64::from(-INT_MAX)),
    };

    let mut np = agfstnode(zz, gp);
    while let Some(n) = np {
        bb = addNodeObj(zz, n, &mut objs[objp], bb);
        objp += 1;
        let mut ep = agfstout(zz, gp, n);
        while let Some(e) = ep {
            let have_edge = HAVE_EDGE(zz, e, et);
            if let Some(lp) = zz.ed(e).label {
                if zz.textlabels[lp].set != 0 {
                    bb = addLabelObj(zz, lp, &mut objs[objp], bb);
                } else if have_edge {
                    let pos = edgeMidpoint(zz, gp, e);
                    addXLabel(zz, lp, &mut objs[objp], &mut lbls, xlp, true, pos);
                    xlp += 1;
                } else {
                    unimplemented!("addXLabels: no position for edge with label");
                }
                objp += 1;
            }
            if let Some(lp) = zz.ed(e).tail_label {
                if zz.textlabels[lp].set != 0 {
                    unimplemented!("addXLabels with a placed tail label");
                } else if have_edge {
                    let pos = edgeTailpoint(zz, e);
                    addXLabel(zz, lp, &mut objs[objp], &mut lbls, xlp, true, pos);
                    xlp += 1;
                } else {
                    unimplemented!("addXLabels: no position for edge with tail label");
                }
                objp += 1;
            }
            if let Some(lp) = zz.ed(e).head_label {
                if zz.textlabels[lp].set != 0 {
                    unimplemented!("addXLabels with a placed head label");
                } else if have_edge {
                    let pos = edgeHeadpoint(zz, e);
                    addXLabel(zz, lp, &mut objs[objp], &mut lbls, xlp, true, pos);
                    xlp += 1;
                } else {
                    unimplemented!("addXLabels: no position for edge with head label");
                }
                objp += 1;
            }
            ep = agnxtout(zz, gp, e);
        }
        np = agnxtnode(zz, gp, n);
    }
    if n_clbls != 0 {
        let info = addClusterObj(zz, gp, cinfo_t { bb, objp }, &mut objs);
        bb = info.bb;
    }

    let force = agattr(zz, Some(gp), AGRAPH, "forcelabels", None);
    let params = label_params_t {
        force: late_bool(force, 1),
        bb,
    };
    placeLabels(&objs, n_objs, &mut lbls, &params);

    let mut cnt = 0;
    for xl in &lbls {
        if xl.set != 0 {
            cnt += 1;
            let lp = xl.lbl.expect("xlabel_t without its label");
            zz.textlabels[lp].set = 1;
            zz.textlabels[lp].pos = centerPt(xl);
            updateBB(zz, gp, lp);
        }
    }
    if cnt != n_lbls {
        unimplemented!("addXLabels: {cnt} out of {n_lbls} exterior labels positioned");
    }
}

/// `gv_postprocess`.
pub(crate) fn gv_postprocess(zz: &mut Globals, g: GraphId, allowTranslation: bool) {
    zz.Rankdir = zz.gd(g).GD_rankdir();
    zz.Flip = zz.gd(g).GD_flip();
    if zz.Flip {
        place_flip_graph_label(zz, g);
    } else {
        place_graph_label(zz, g);
    }

    addXLabels(zz, g);

    if let Some(l) = zz.gd(g).label
        && zz.textlabels[l].set == 0
    {
        unimplemented!("gv_postprocess with a root graph label");
    }

    if allowTranslation {
        let bb = zz.gd(g).bb;
        match zz.Rankdir {
            RANKDIR_TB => zz.Offset = bb.LL,
            RANKDIR_LR => zz.Offset = pointfof(-bb.UR.y, bb.LL.x),
            RANKDIR_BT | RANKDIR_RL => unimplemented!("rankdir BT and RL"),
            _ => {}
        }
        translate_drawing(zz, g);
    }
}

/// `dotneato_postprocess`.
pub fn dotneato_postprocess(zz: &mut Globals, g: GraphId) {
    gv_postprocess(zz, g, true);
}

/// `place_flip_graph_label`: puts the cluster labels in their place for a rotated layout (`rankdir=LR`), in the
/// layout's coordinates, before the rotation.
fn place_flip_graph_label(zz: &mut Globals, g: GraphId) {
    if g != agroot(zz, g)
        && let Some(l) = zz.gd(g).label
        && zz.textlabels[l].set == 0
    {
        let gd = zz.gd(g);
        let mut p = pointf::default();
        let d;
        if (gd.label_pos & LABEL_AT_TOP) != 0 {
            d = gd.border[RIGHT_IX as usize];
            p.x = gd.bb.UR.x - d.x / 2.0;
        } else {
            d = gd.border[LEFT_IX as usize];
            p.x = gd.bb.LL.x + d.x / 2.0;
        }
        if (gd.label_pos & LABEL_AT_RIGHT) != 0 {
            p.y = gd.bb.LL.y + d.y / 2.0;
        } else if (gd.label_pos & LABEL_AT_LEFT) != 0 {
            p.y = gd.bb.UR.y - d.y / 2.0;
        } else {
            p.y = (gd.bb.LL.y + gd.bb.UR.y) / 2.0;
        }
        zz.textlabels[l].pos = p;
        zz.textlabels[l].set = 1;
    }
    for c in 1..=zz.gd(g).n_cluster {
        place_flip_graph_label(zz, GD_clust(zz, g, c));
    }
}

/// `place_graph_label`: puts the cluster labels in their place, centred at the top.
fn place_graph_label(zz: &mut Globals, g: GraphId) {
    if g != agroot(zz, g)
        && let Some(l) = zz.gd(g).label
        && zz.textlabels[l].set == 0
    {
        let gd = zz.gd(g);
        let mut p = pointf::default();
        if (gd.label_pos & LABEL_AT_TOP) != 0 {
            let d = gd.border[TOP_IX as usize];
            p.y = gd.bb.UR.y - d.y / 2.0;
        } else {
            unimplemented!("place_graph_label at the bottom");
        }
        if (gd.label_pos & LABEL_AT_RIGHT) != 0 || (gd.label_pos & LABEL_AT_LEFT) != 0 {
            unimplemented!("place_graph_label at the side");
        }
        p.x = (gd.bb.LL.x + gd.bb.UR.x) / 2.0;
        zz.textlabels[l].pos = p;
        zz.textlabels[l].set = 1;
    }
    for c in 1..=zz.gd(g).n_cluster {
        place_graph_label(zz, GD_clust(zz, g, c));
    }
}

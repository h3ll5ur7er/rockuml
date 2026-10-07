//! `dotgen/position.c`: node coordinates. y comes from the ranks' heights; x from network simplex on an
//! auxiliary graph whose edges keep nodes apart within ranks, pull the ends of edges together and keep clusters
//! around their nodes. The auxiliary graph lives in the nodes' `ND_in`/`ND_out` lists, with the fast graph's
//! saved in `ND_save_in`/`ND_save_out` until `remove_aux_edges`.

use std::cmp::{max, min};
use std::collections::HashSet;

use crate::cgraph::attr::agget;
use crate::cgraph::graph::agnnodes;
use crate::cgraph::obj::{agcontains, agraphof, agroot};
use crate::cgraph::{M_aghead, M_agtail, aghead, agtail};
use crate::common::ns::rank;
use crate::common::utils::late_int;
use crate::core::Globals;
use crate::core::consts::{
    BOTTOM_IX, CL_OFFSET, EDGE_LABEL, INT_MAX, LEAFSET, LEFT_IX, NORMAL, RIGHT_IX, SLACKNODE,
    TOP_IX, USHRT_MAX, VIRTUAL,
};
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::core::jmath::{self, ROUND};
use crate::core::jutils::atof;
use crate::dotgen::aspect::aspect_t;
use crate::dotgen::cluster::mark_lowclusters;
use crate::dotgen::dotinit::dot_root;
use crate::dotgen::fastgr::{fast_edge, find_fast_edge, new_edge_pair, virtual_node};
use crate::dotgen::flat::flat_edges;
use crate::dotgen::mincross::{rank_node, rank_v};
use crate::dotgen::rank::cluster;
use crate::h::{EN_ratio_t, alloc_elist, elist, pointf};

/// `largeMinlen`: Smetana cannot lay out edges longer than 65535 points.
fn largeMinlen(l: f64) -> f64 {
    unimplemented!("largeMinlen({l})")
}

/// The edges of a NULL-terminated list, read before the caller changes anything.
fn edges_of(zz: &Globals, l: elist) -> Vec<EdgeId> {
    if l.list.is_none() {
        return Vec::new();
    }
    (0..).map_while(|i| l.get(&zz.edge_lists, i)).collect()
}

/// The nodes of `g`'s fast graph (`GD_nlist`), read before the caller changes anything.
fn nlist(zz: &Globals, g: GraphId) -> Vec<NodeId> {
    std::iter::successors(zz.gd(g).nlist, |&n| zz.nd(n).next).collect()
}

/// `GD_margin` of cluster `g`, with default `def`.
fn margin_of(zz: &mut Globals, g: GraphId, def: i32) -> i32 {
    let margin = zz.G_margin;
    late_int(zz, g, margin, def, 0)
}

/// `connectGraph`: connects the auxiliary graph when network simplex found it disconnected, linking the first
/// node of every rank without an edge to a lower rank to the first node of a neighbouring rank.
fn connectGraph(zz: &mut Globals, g: GraphId) {
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        let mut found = false;
        let mut tp = None;
        for i in 0..zz.rank(g, r).n {
            let t = rank_node(zz, g, r, i);
            tp = Some(t);
            let lower =
                |e: &EdgeId| zz.nd(aghead(zz, *e)).rank > r || zz.nd(agtail(zz, *e)).rank > r;
            if edges_of(zz, zz.nd(t).save_out).iter().any(lower)
                || edges_of(zz, zz.nd(t).save_in).iter().any(lower)
            {
                found = true;
                break;
            }
        }
        if found || tp.is_none() {
            continue;
        }
        let tp = rank_node(zz, g, r, 0);
        let hp = if r < zz.gd(g).maxrank {
            rank_node(zz, g, r + 1, 0)
        } else {
            rank_node(zz, g, r - 1, 0)
        };
        let sn = virtual_node(zz, g);
        zz.nd_mut(sn).node_type = SLACKNODE;
        make_aux_edge(zz, sn, tp, 0.0, 0);
        make_aux_edge(zz, sn, hp, 0.0, 0);
        zz.nd_mut(sn).rank = min(zz.nd(tp).rank, zz.nd(hp).rank);
    }
}

/// `dot_position`: the coordinates of all nodes, and the bounding boxes of the graph and its clusters.
pub fn dot_position(zz: &mut Globals, g: GraphId, asp: Option<&aspect_t>) {
    if zz.gd(g).nlist.is_none() {
        return; // ignore empty graph
    }
    mark_lowclusters(zz, g); // we could remove from splines.c now
    set_ycoords(zz, g);
    if zz.Concentrate {
        unimplemented!("dot_concentrate");
    }
    expand_leaves(zz, g);
    if flat_edges(zz, g) {
        set_ycoords(zz, g);
    }
    create_aux_edges(zz, g);
    let maxiter = nsiter2(zz, g);
    if rank(zz, g, 2, maxiter) != 0 {
        // LR balance == 2
        connectGraph(zz, g);
    }
    set_xcoords(zz, g);
    clampSkippedLabelVnodes(zz);
    set_aspect(zz, g, asp);
    remove_aux_edges(zz, g); // must come after set_aspect since we now use GD_ln and GD_rn for bbox width.
}

/// `nsiter2`: the iteration limit of network simplex, from the `nslimit` attribute.
fn nsiter2(zz: &mut Globals, g: GraphId) -> i32 {
    match agget(zz, g, "nslimit") {
        Some(s) => {
            let limit = atof(zz.agstr(s));
            (limit * f64::from(agnnodes(zz, g))) as i32
        }
        None => INT_MAX,
    }
}

/// `make_aux_edge`: an auxiliary edge from `u` to `v` of minimum length `len` and weight `wt`, in the fast graph.
pub fn make_aux_edge(zz: &mut Globals, u: NodeId, v: NodeId, len: f64, wt: i32) -> EdgeId {
    let e = new_edge_pair(zz);
    M_agtail(zz, e, u);
    M_aghead(zz, e, v);
    let len = if len > f64::from(USHRT_MAX) {
        largeMinlen(len)
    } else {
        len
    };
    zz.ed_mut(e).minlen = ROUND(len);
    zz.ed_mut(e).weight = wt;
    fast_edge(zz, e);
    e
}

/// `allocate_aux_edges`: saves the fast graph's edge lists and starts empty ones for the auxiliary edges.
fn allocate_aux_edges(zz: &mut Globals, g: GraphId) {
    for n in nlist(zz, g) {
        let (in_, out) = (zz.nd(n).in_, zz.nd(n).out);
        zz.nd_mut(n).save_in = in_;
        zz.nd_mut(n).save_out = out;
        let n_in = edges_of(zz, out).len() + edges_of(zz, in_).len();
        let n_in = i32::try_from(n_in).expect("edge count");
        let mut in_ = in_;
        let mut out = out;
        alloc_elist(&mut zz.edge_lists, n_in + 3, &mut in_);
        alloc_elist(&mut zz.edge_lists, 3, &mut out);
        zz.nd_mut(n).in_ = in_;
        zz.nd_mut(n).out = out;
    }
}

/// `selfRightSpace` (`common/splines.c`): the room a self loop takes right of its node.
fn selfRightSpace(zz: &Globals, e: EdgeId) -> i32 {
    let info = zz.ed(e);
    let (tp, hp) = (info.tail_port, info.head_port);
    if (!tp.defined && !hp.defined)
        || ((tp.side & (1 << 3)) == 0
            && (hp.side & (1 << 3)) == 0
            && (tp.side != hp.side || (tp.side & ((1 << 2) | (1 << 0))) == 0))
    {
        let mut sw = 18;
        if let Some(l) = info.label {
            let dimen = zz.textlabels[l].dimen;
            let flip = zz.gd(agraphof(zz, aghead(zz, e))).GD_flip();
            let label_width = if flip { dimen.y } else { dimen.x };
            sw = (f64::from(sw) + label_width) as i32;
        }
        sw
    } else {
        0
    }
}

/// `make_LR_constraints`, as PlantUML patched it: edges keeping the nodes of every rank apart, and the ends of
/// flat edges. The constraints placing flat edge labels between their ends come last, once all the others
/// exist, and are left out where they would close a cycle; their label nodes are recorded in
/// `zz.skippedConstraintLabelVnodes`.
fn make_LR_constraints(zz: &mut Globals, g: GraphId) {
    let nodesep_g = zz.gd(g).nodesep;
    let mut deferredLabelNodes = Vec::new();
    // Use smaller separation on odd ranks if g has edge labels.
    let sep = if (zz.gd(g).has_labels & EDGE_LABEL) != 0 {
        [nodesep_g, 5]
    } else {
        [nodesep_g, nodesep_g]
    };
    // Make edges to constrain left-to-right ordering.
    for i in zz.gd(g).minrank..=zz.gd(g).maxrank {
        let first = rank_node(zz, g, i, 0);
        zz.nd_mut(first).rank = 0;
        let mut last = 0.0;
        let nodesep = sep[usize::from(i & 1 != 0)];
        let v_list = rank_v(zz, g, i);
        for j in 0..zz.rank(g, i).n {
            let u = zz.node_lists.get(v_list, j).expect("node in rank");
            zz.nd_mut(u).mval = zz.nd(u).rw; // keep it somewhere safe
            if zz.nd(u).other.size > 0 {
                // Compute self size. Dot assumes all self loops go to the right.
                let mut sw = 0;
                for e in edges_of(zz, zz.nd(u).other) {
                    if agtail(zz, e) == aghead(zz, e) {
                        sw += selfRightSpace(zz, e);
                    }
                }
                zz.nd_mut(u).rw += f64::from(sw); // increment to include self edges
            }
            if let Some(v) = zz.node_lists.get(v_list, j + 1) {
                let width = zz.nd(u).rw + zz.nd(v).lw + f64::from(nodesep);
                make_aux_edge(zz, u, v, width, 0);
                zz.nd_mut(v).rank = (last + width) as i32;
                last = f64::from((last + width) as i32);
            }
            // Constraints from labels of flat edges on the previous rank come after the loop.
            if zz.nd(u).alg.is_some() {
                deferredLabelNodes.push(u);
            }
            // Position flat edge endpoints.
            for k in 0..zz.nd(u).flat_out.size {
                let e = zz.nd(u).flat_out.get(&zz.edge_lists, k).expect("flat edge");
                let (t0, h0) = if zz.nd(agtail(zz, e)).order < zz.nd(aghead(zz, e)).order {
                    (agtail(zz, e), aghead(zz, e))
                } else {
                    (aghead(zz, e), agtail(zz, e))
                };
                let width = zz.nd(t0).rw + zz.nd(h0).lw;
                let mut m0 = (f64::from(zz.ed(e).minlen * nodesep_g) + width) as i32;
                if let Some(e0) = find_fast_edge(zz, t0, h0) {
                    // Flat edge between adjacent neighbors; ED_dist has the largest label width.
                    let dist = f64::from(ROUND(zz.ed(e).dist));
                    m0 = max(m0, (width + f64::from(nodesep_g) + dist) as i32);
                    if m0 > USHRT_MAX {
                        m0 = largeMinlen(f64::from(m0)) as i32;
                    }
                    zz.ed_mut(e0).minlen = max(zz.ed(e0).minlen, m0);
                    zz.ed_mut(e0).weight = max(zz.ed(e0).weight, zz.ed(e).weight);
                } else if zz.ed(e).label.is_none() {
                    // Unlabeled flat edge between non-neighbors; ED_minlen(e) is the max of the equivalent edges'.
                    let weight = zz.ed(e).weight;
                    make_aux_edge(zz, t0, h0, f64::from(m0), weight);
                }
                // Labeled flat edges between non-neighbors are constrained by their label.
            }
        }
    }
    for lu in deferredLabelNodes {
        let e = zz.nd(lu).alg.expect("label edge");
        let save_out = zz.nd(lu).save_out;
        let mut e0 = save_out.get(&zz.edge_lists, 0).expect("label node edge");
        let mut e1 = save_out.get(&zz.edge_lists, 1).expect("label node edge");
        if zz.nd(aghead(zz, e0)).order > zz.nd(aghead(zz, e1)).order {
            std::mem::swap(&mut e0, &mut e1);
        }
        let m0 = (zz.ed(e).minlen * nodesep_g) / 2;
        let weight = zz.ed(e).weight;
        // These guards are needed because the flat edges work very poorly with cluster layout.
        let m1 = m0 + (zz.nd(aghead(zz, e0)).rw + zz.nd(agtail(zz, e0)).lw) as i32;
        if canReachInAuxGraph(zz, agtail(zz, e0), aghead(zz, e0)) {
            skip_label_constraint(zz, lu);
        } else {
            make_aux_edge(zz, aghead(zz, e0), agtail(zz, e0), f64::from(m1), weight);
        }
        let m1 = m0 + (zz.nd(agtail(zz, e1)).rw + zz.nd(aghead(zz, e1)).lw) as i32;
        if canReachInAuxGraph(zz, aghead(zz, e1), agtail(zz, e1)) {
            skip_label_constraint(zz, lu);
        } else {
            make_aux_edge(zz, agtail(zz, e1), aghead(zz, e1), f64::from(m1), weight);
        }
    }
}

/// `zz.skippedConstraintLabelVnodes.put(lu, TRUE)`.
fn skip_label_constraint(zz: &mut Globals, lu: NodeId) {
    if !zz.skippedConstraintLabelVnodes.contains(&lu) {
        zz.skippedConstraintLabelVnodes.push(lu);
    }
}

/// `canReachInAuxGraph` (PlantUML's): whether the auxiliary graph has a path from `from` to `to`.
fn canReachInAuxGraph(zz: &Globals, from: NodeId, to: NodeId) -> bool {
    if from == to {
        return true;
    }
    let mut visited = HashSet::from([from]);
    let mut stack = vec![from];
    while let Some(cur) = stack.pop() {
        for e in edges_of(zz, zz.nd(cur).out) {
            let head = aghead(zz, e);
            if head == to {
                return true;
            }
            if visited.insert(head) {
                stack.push(head);
            }
        }
    }
    false
}

/// `make_edge_pairs`: for every edge, a slack node with edges to both ends, which pulls them together.
fn make_edge_pairs(zz: &mut Globals, g: GraphId) {
    for n in nlist(zz, g) {
        for e in edges_of(zz, zz.nd(n).save_out) {
            let sn = virtual_node(zz, g);
            zz.nd_mut(sn).node_type = SLACKNODE;
            let mut m0 = (zz.ed(e).head_port.p.x - zz.ed(e).tail_port.p.x) as i32;
            let m1 = if m0 > 0 {
                0
            } else {
                let m1 = -m0;
                m0 = 0;
                m1
            };
            let (tail, head, weight) = (agtail(zz, e), aghead(zz, e), zz.ed(e).weight);
            make_aux_edge(zz, sn, tail, f64::from(m0 + 1), weight);
            make_aux_edge(zz, sn, head, f64::from(m1 + 1), weight);
            zz.nd_mut(sn).rank = min(zz.nd(tail).rank - m0 - 1, zz.nd(head).rank - m1 - 1);
        }
    }
}

/// `contain_clustnodes`: keeps the nodes of every cluster between its boundary nodes, and pulls these together.
fn contain_clustnodes(zz: &mut Globals, g: GraphId) {
    if g != dot_root(zz, g) {
        contain_nodes(zz, g);
        let (ln, rn) = boundary_nodes(zz, g);
        match find_fast_edge(zz, ln, rn) {
            // maybe from lrvn()?
            Some(e) => zz.ed_mut(e).weight += 128,
            None => {
                make_aux_edge(zz, ln, rn, 1.0, 128); // clust compaction edge
            }
        }
    }
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        contain_clustnodes(zz, clust);
    }
}

/// `GD_ln(g)` and `GD_rn(g)`, which must exist.
fn boundary_nodes(zz: &Globals, g: GraphId) -> (NodeId, NodeId) {
    let info = zz.gd(g);
    (info.ln.expect("GD_ln"), info.rn.expect("GD_rn"))
}

/// `vnode_not_related_to`: whether virtual node `v` belongs to an edge with no end in `g`.
fn vnode_not_related_to(zz: &mut Globals, g: GraphId, v: NodeId) -> bool {
    if zz.nd(v).node_type != VIRTUAL {
        return false;
    }
    let mut e = zz.nd(v).save_out.get(&zz.edge_lists, 0).expect("out edge");
    while let Some(orig) = zz.ed(e).to_orig {
        e = orig;
    }
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    !agcontains(zz, g, tail) && !agcontains(zz, g, head)
}

/// `keepout_othernodes`: keeps the nearest unrelated node on each side out of every cluster.
fn keepout_othernodes(zz: &mut Globals, g: GraphId) {
    let margin = margin_of(zz, g, 8);
    let root = dot_root(zz, g);
    let (ln, rn) = (zz.gd(g).ln, zz.gd(g).rn);
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        if zz.rank(g, r).n == 0 {
            continue;
        }
        let Some(v) = zz.node_lists.get(rank_v(zz, g, r), 0) else {
            continue;
        };
        let mut i = zz.nd(v).order - 1;
        while i >= 0 {
            let u = rank_node(zz, root, r, i);
            // Can't use "is_a_vnode_of" because elists are swapped.
            if zz.nd(u).node_type == NORMAL || vnode_not_related_to(zz, g, u) {
                let len = f64::from(margin) + zz.nd(u).rw;
                make_aux_edge(zz, u, ln.expect("GD_ln"), len, 0);
                break;
            }
            i -= 1;
        }
        let mut i = zz.nd(v).order + zz.rank(g, r).n;
        while i < zz.rank(root, r).n {
            let u = rank_node(zz, root, r, i);
            if zz.nd(u).node_type == NORMAL || vnode_not_related_to(zz, g, u) {
                let len = f64::from(margin) + zz.nd(u).lw;
                make_aux_edge(zz, rn.expect("GD_rn"), u, len, 0);
                break;
            }
            i += 1;
        }
    }
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        keepout_othernodes(zz, clust);
    }
}

/// `contain_subclust`: keeps every sub-cluster inside its parent.
fn contain_subclust(zz: &mut Globals, g: GraphId) {
    let margin = margin_of(zz, g, 8);
    make_lrvn(zz, g);
    for c in 1..=zz.gd(g).n_cluster {
        let subg = cluster(zz, g, c);
        make_lrvn(zz, subg);
        let (gl, gr) = boundary_nodes(zz, g);
        let (sl, sr) = boundary_nodes(zz, subg);
        let border = zz.gd(g).border;
        make_aux_edge(
            zz,
            gl,
            sl,
            f64::from(margin) + border[LEFT_IX as usize].x,
            0,
        );
        make_aux_edge(
            zz,
            sr,
            gr,
            f64::from(margin) + border[RIGHT_IX as usize].x,
            0,
        );
        contain_subclust(zz, subg);
    }
}

/// `separate_subclust`: keeps sibling clusters that share ranks apart.
fn separate_subclust(zz: &mut Globals, g: GraphId) {
    let margin = margin_of(zz, g, 8);
    let n_cluster = zz.gd(g).n_cluster;
    for i in 1..=n_cluster {
        let clust = cluster(zz, g, i);
        make_lrvn(zz, clust);
    }
    for i in 1..=n_cluster {
        for j in i + 1..=n_cluster {
            let mut low = cluster(zz, g, i);
            let mut high = cluster(zz, g, j);
            if zz.gd(low).minrank > zz.gd(high).minrank {
                std::mem::swap(&mut low, &mut high);
            }
            if zz.gd(low).maxrank < zz.gd(high).minrank {
                continue;
            }
            let r = zz.gd(high).minrank;
            let (left, right) =
                if zz.nd(rank_node(zz, low, r, 0)).order < zz.nd(rank_node(zz, high, r, 0)).order {
                    (low, high)
                } else {
                    (high, low)
                };
            let rn = boundary_nodes(zz, left).1;
            let ln = boundary_nodes(zz, right).0;
            make_aux_edge(zz, rn, ln, f64::from(margin), 0);
        }
        let clust = cluster(zz, g, i);
        separate_subclust(zz, clust);
    }
}

/// `pos_clusters`: the constraints of clusters.
fn pos_clusters(zz: &mut Globals, g: GraphId) {
    if zz.gd(g).n_cluster > 0 {
        contain_clustnodes(zz, g);
        keepout_othernodes(zz, g);
        contain_subclust(zz, g);
        separate_subclust(zz, g);
    }
}

/// `compress_graph`: only for `ratio=compress`, which PlantUML never sets.
fn compress_graph(zz: &Globals, g: GraphId) {
    if zz.gd(g).drawing.expect("GD_drawing").ratio_kind == EN_ratio_t::R_COMPRESS {
        unimplemented!("ratio=compress");
    }
}

/// `create_aux_edges`: the auxiliary graph.
fn create_aux_edges(zz: &mut Globals, g: GraphId) {
    allocate_aux_edges(zz, g);
    make_LR_constraints(zz, g);
    make_edge_pairs(zz, g);
    pos_clusters(zz, g);
    compress_graph(zz, g);
}

/// `remove_aux_edges`: restores the fast graph's edge lists and drops the slack nodes.
fn remove_aux_edges(zz: &mut Globals, g: GraphId) {
    for n in nlist(zz, g) {
        let info = zz.nd_mut(n);
        info.out = info.save_out;
        info.in_ = info.save_in;
    }
    // Cannot be merged with the previous loop.
    let mut nprev: Option<NodeId> = None;
    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        let nnext = zz.nd(nn).next;
        if zz.nd(nn).node_type == SLACKNODE {
            match nprev {
                Some(p) => zz.nd_mut(p).next = nnext,
                None => zz.gd_mut(g).nlist = nnext,
            }
        } else {
            nprev = Some(nn);
        }
        n = nnext;
    }
    let first = zz.gd(g).nlist.expect("GD_nlist");
    zz.nd_mut(first).prev = None;
}

/// `set_xcoords`: x is the rank network simplex gave each node; the rank goes back to `ND_rank`.
fn set_xcoords(zz: &mut Globals, g: GraphId) {
    for i in zz.gd(g).minrank..=zz.gd(g).maxrank {
        for j in 0..zz.rank(g, i).n {
            let v = rank_node(zz, g, i, j);
            zz.nd_mut(v).coord.x = f64::from(zz.nd(v).rank);
            zz.nd_mut(v).rank = i;
        }
    }
}

/// `clampSkippedLabelVnodes` (PlantUML's): moves a label node that lost a constraint in `make_LR_constraints`
/// back between the ends of its edge, so that the edge can be routed.
fn clampSkippedLabelVnodes(zz: &mut Globals) {
    for lu in zz.skippedConstraintLabelVnodes.clone() {
        let save_out = zz.nd(lu).save_out;
        let (Some(e0), Some(e1)) = (
            save_out.get(&zz.edge_lists, 0),
            save_out.get(&zz.edge_lists, 1),
        ) else {
            continue;
        };
        let a = zz.nd(aghead(zz, e0)).coord.x;
        let b = zz.nd(aghead(zz, e1)).coord.x;
        let lo = jmath::min(a, b);
        let hi = jmath::max(a, b);
        let x = zz.nd(lu).coord.x;
        if x < lo || x > hi {
            zz.nd_mut(lu).coord.x = jmath::max(lo, jmath::min(hi, x));
        }
    }
}

/// `adjustSimple`: spreads `delta` more points of height over the top and bottom of cluster `g`.
fn adjustSimple(zz: &mut Globals, g: GraphId, delta: i32, margin_total: i32) {
    let root = dot_root(zz, g);
    let maxr = zz.gd(g).maxrank;
    let minr = zz.gd(g).minrank;
    let margin_total = f64::from(margin_total);

    let bottom = (delta + 1) / 2;
    let delbottom =
        (zz.gd(g).ht1 + f64::from(bottom) - (zz.rank(root, maxr).ht1 - margin_total)) as i32;
    let deltop = if delbottom > 0 {
        for r in (minr..=maxr).rev() {
            shift_rank(zz, root, r, delbottom);
        }
        (zz.gd(g).ht2 + f64::from(delta - bottom) + f64::from(delbottom)
            - (zz.rank(root, minr).ht2 - margin_total)) as i32
    } else {
        (zz.gd(g).ht2 + f64::from(delta - bottom) - (zz.rank(root, minr).ht2 - margin_total)) as i32
    };
    if deltop > 0 {
        for r in (zz.gd(root).minrank..minr).rev() {
            shift_rank(zz, root, r, deltop);
        }
    }
    zz.gd_mut(g).ht2 += f64::from(delta - bottom);
    zz.gd_mut(g).ht1 += f64::from(bottom);
}

/// Moves the leftmost node of rank `r`, if any, `d` points up.
fn shift_rank(zz: &mut Globals, root: GraphId, r: i32, d: i32) {
    if zz.rank(root, r).n > 0 {
        let v = rank_node(zz, root, r, 0);
        zz.nd_mut(v).coord.y += f64::from(d);
    }
}

/// `adjustRanks`: makes room for wide cluster labels when ranks run left to right, dividing the extra space
/// between top and bottom and updating `ht1` and `ht2`.
fn adjustRanks(zz: &mut Globals, g: GraphId, margin_total: i32) {
    let root = dot_root(zz, g);
    let margin = if g == root {
        0
    } else {
        margin_of(zz, g, CL_OFFSET)
    };

    let mut ht1 = zz.gd(g).ht1;
    let mut ht2 = zz.gd(g).ht2;

    for c in 1..=zz.gd(g).n_cluster {
        let subg = cluster(zz, g, c);
        adjustRanks(zz, subg, margin + margin_total);
        if zz.gd(subg).maxrank == zz.gd(g).maxrank {
            ht1 = jmath::max(ht1, zz.gd(subg).ht1 + f64::from(margin));
        }
        if zz.gd(subg).minrank == zz.gd(g).minrank {
            ht2 = jmath::max(ht2, zz.gd(subg).ht2 + f64::from(margin));
        }
    }

    zz.gd_mut(g).ht1 = ht1;
    zz.gd_mut(g).ht2 = ht2;

    if g != root && zz.gd(g).label.is_some() {
        let border = zz.gd(g).border;
        let lht = jmath::max(border[LEFT_IX as usize].y, border[RIGHT_IX as usize].y);
        let maxr = zz.gd(g).maxrank;
        let minr = zz.gd(g).minrank;
        let rht = zz.nd(rank_node(zz, root, minr, 0)).coord.y
            - zz.nd(rank_node(zz, root, maxr, 0)).coord.y;
        let delta = lht - (rht + ht1 + ht2);
        if delta > 0.0 {
            adjustSimple(zz, g, delta as i32, margin_total);
        }
    }

    // Update the global ranks.
    if g != root {
        let (minr, maxr) = (zz.gd(g).minrank, zz.gd(g).maxrank);
        let (ht1, ht2) = (zz.gd(g).ht1, zz.gd(g).ht2);
        zz.rank_mut(root, minr).ht2 = jmath::max(zz.rank(root, minr).ht2, ht2);
        zz.rank_mut(root, maxr).ht1 = jmath::max(zz.rank(root, maxr).ht1, ht1);
    }
}

/// `clust_ht`: the height clusters need above and below their ranks, from their nodes (already in `ht1` and
/// `ht2`), sub-clusters and labels; also raises the root's rank heights. Returns whether a cluster has a label.
fn clust_ht(zz: &mut Globals, g: GraphId) -> bool {
    let root = dot_root(zz, g);
    let margin = if g == root {
        CL_OFFSET
    } else {
        margin_of(zz, g, CL_OFFSET)
    };
    let mut haveClustLabel = false;

    let mut ht1 = zz.gd(g).ht1;
    let mut ht2 = zz.gd(g).ht2;

    // Account for sub-clusters.
    for c in 1..=zz.gd(g).n_cluster {
        let subg = cluster(zz, g, c);
        haveClustLabel |= clust_ht(zz, subg);
        if zz.gd(subg).maxrank == zz.gd(g).maxrank {
            ht1 = jmath::max(ht1, zz.gd(subg).ht1 + f64::from(margin));
        }
        if zz.gd(subg).minrank == zz.gd(g).minrank {
            ht2 = jmath::max(ht2, zz.gd(subg).ht2 + f64::from(margin));
        }
    }

    // Account for a cluster label; room for the root graph's label is made in dotneato_postprocess.
    if g != root && zz.gd(g).label.is_some() {
        haveClustLabel = true;
        if !zz.gd(agroot(zz, g)).GD_flip() {
            let border = zz.gd(g).border;
            ht1 += border[BOTTOM_IX as usize].y;
            ht2 += border[TOP_IX as usize].y;
        }
    }
    zz.gd_mut(g).ht1 = ht1;
    zz.gd_mut(g).ht2 = ht2;

    // Update the global ranks.
    if g != root {
        let (minr, maxr) = (zz.gd(g).minrank, zz.gd(g).maxrank);
        zz.rank_mut(root, minr).ht2 = jmath::max(zz.rank(root, minr).ht2, ht2);
        zz.rank_mut(root, maxr).ht1 = jmath::max(zz.rank(root, maxr).ht1, ht1);
    }
    haveClustLabel
}

/// `set_ycoords`: the y coordinate of every rank, from the heights of its nodes, self loop labels and clusters.
fn set_ycoords(zz: &mut Globals, g: GraphId) {
    // Scan ranks for tallest nodes.
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        for i in 0..zz.rank(g, r).n {
            let n = rank_node(zz, g, r, i);

            // Assumes symmetry, ht1 = ht2.
            let mut ht2 = zz.nd(n).ht / 2.0;

            // Have to look for high self-edge labels, too.
            for e in edges_of(zz, zz.nd(n).other) {
                if agtail(zz, e) == aghead(zz, e)
                    && let Some(l) = zz.ed(e).label
                {
                    ht2 = jmath::max(ht2, zz.textlabels[l].dimen.y / 2.0);
                }
            }

            // Update global rank ht.
            let rank = zz.rank_mut(g, r);
            if rank.pht2 < ht2 {
                rank.ht2 = ht2;
                rank.pht2 = ht2;
            }
            if rank.pht1 < ht2 {
                rank.ht1 = ht2;
                rank.pht1 = ht2;
            }

            // Update nearest enclosing cluster rank ht.
            if let Some(clust) = zz.nd(n).clust {
                let yoff = if clust == g {
                    0
                } else {
                    margin_of(zz, clust, CL_OFFSET)
                };
                let yoff = f64::from(yoff);
                if zz.nd(n).rank == zz.gd(clust).minrank {
                    zz.gd_mut(clust).ht2 = jmath::max(zz.gd(clust).ht2, ht2 + yoff);
                }
                if zz.nd(n).rank == zz.gd(clust).maxrank {
                    zz.gd_mut(clust).ht1 = jmath::max(zz.gd(clust).ht1, ht2 + yoff);
                }
            }
        }
    }

    // Scan sub-clusters.
    let lbl = clust_ht(zz, g);

    // Make the initial assignment of y coordinates to the leftmost nodes by ranks.
    let mut r = zz.gd(g).maxrank;
    let n = rank_node(zz, g, r, 0);
    zz.nd_mut(n).coord.y = zz.rank(g, r).ht1;
    r -= 1;
    while r >= zz.gd(g).minrank {
        let (below, here) = (*zz.rank(g, r + 1), *zz.rank(g, r));
        let d0 = below.pht2 + here.pht1 + f64::from(zz.gd(g).ranksep); // prim node sep
        let d1 = below.ht2 + here.ht1 + f64::from(CL_OFFSET); // cluster sep
        let delta = jmath::max(d0, d1);
        if here.n > 0 {
            // this may reflect some problem
            let y = zz.nd(rank_node(zz, g, r + 1, 0)).coord.y + delta;
            let n = rank_node(zz, g, r, 0);
            zz.nd_mut(n).coord.y = y;
        }
        r -= 1;
    }

    // With cluster labels and a rotated drawing, adjustRanks makes room.
    if lbl && zz.gd(g).GD_flip() {
        adjustRanks(zz, g, 0);
    }

    if zz.gd(g).exact_ranksep != 0 {
        unimplemented!("ranksep=equally");
    }

    // Copy the y coordinate from the leftmost nodes to the others.
    for n in nlist(zz, g) {
        let y = zz.nd(rank_node(zz, g, zz.nd(n).rank, 0)).coord.y;
        zz.nd_mut(n).coord.y = y;
    }
}

/// `dot_compute_bb`: the bounding box of `g`. A cluster's x limits are the positions of its boundary nodes,
/// still in `ND_rank`; the root's come from its outermost real nodes and its clusters.
fn dot_compute_bb(zz: &mut Globals, g: GraphId, root: GraphId) {
    let mut LL = pointf::default();
    let mut UR = pointf::default();
    if g == dot_root(zz, g) {
        LL.x = f64::from(INT_MAX);
        UR.x = -f64::from(INT_MAX);
        for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
            let rnkn = zz.rank(g, r).n;
            if rnkn == 0 {
                continue;
            }
            let vlist = rank_v(zz, g, r);
            let Some(mut v) = zz.node_lists.get(vlist, 0) else {
                continue;
            };
            let mut c = 1;
            while zz.nd(v).node_type != NORMAL && c < rnkn {
                v = zz.node_lists.get(vlist, c).expect("node in rank");
                c += 1;
            }
            if zz.nd(v).node_type != NORMAL {
                continue;
            }
            let x = zz.nd(v).coord.x - zz.nd(v).lw;
            LL.x = jmath::min(LL.x, x);
            // At this point, we know the rank contains a NORMAL node.
            v = zz.node_lists.get(vlist, rnkn - 1).expect("node in rank");
            let mut c = rnkn - 2;
            while zz.nd(v).node_type != NORMAL {
                v = zz.node_lists.get(vlist, c).expect("node in rank");
                c -= 1;
            }
            let x = zz.nd(v).coord.x + zz.nd(v).rw;
            UR.x = jmath::max(UR.x, x);
        }
        let offset = f64::from(CL_OFFSET);
        for c in 1..=zz.gd(g).n_cluster {
            let bb = zz.gd(cluster(zz, g, c)).bb;
            LL.x = jmath::min(LL.x, bb.LL.x - offset);
            UR.x = jmath::max(UR.x, bb.UR.x + offset);
        }
    } else {
        let (ln, rn) = boundary_nodes(zz, g);
        LL.x = f64::from(zz.nd(ln).rank);
        UR.x = f64::from(zz.nd(rn).rank);
    }
    LL.y = zz.nd(rank_node(zz, root, zz.gd(g).maxrank, 0)).coord.y - zz.gd(g).ht1;
    UR.y = zz.nd(rank_node(zz, root, zz.gd(g).minrank, 0)).coord.y + zz.gd(g).ht2;
    zz.gd_mut(g).bb.LL = LL;
    zz.gd_mut(g).bb.UR = UR;
}

/// `rec_bb`: the bounding boxes of `g`'s clusters, then of `g`.
fn rec_bb(zz: &mut Globals, g: GraphId, root: GraphId) {
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        rec_bb(zz, clust, root);
    }
    dot_compute_bb(zz, g, root);
}

/// `set_aspect`: the bounding boxes. Scaling to a `ratio` and aspect-driven layout are not supported.
fn set_aspect(zz: &mut Globals, g: GraphId, asp: Option<&aspect_t>) {
    rec_bb(zz, g, g);
    if zz.gd(g).maxrank > 0
        && zz.gd(g).drawing.expect("GD_drawing").ratio_kind != EN_ratio_t::R_NONE
    {
        unimplemented!("ratio");
    }
    if asp.is_some() {
        unimplemented!("adjustAspectRatio");
    }
}

/// `make_leafslots`: makes room for the leaf nodes of each rank. Leaf sets are not supported, so this only
/// renumbers `ND_order`.
fn make_leafslots(zz: &mut Globals, g: GraphId) {
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        for i in 0..zz.rank(g, r).n {
            let v = rank_node(zz, g, r, i);
            zz.nd_mut(v).order = i;
            if zz.nd(v).ranktype == LEAFSET {
                unimplemented!("leaf sets");
            }
        }
    }
}

/// `ports_eq`: whether two edges have the same ports (undefined ports match any).
pub fn ports_eq(zz: &Globals, e: EdgeId, f: EdgeId) -> bool {
    let (e, f) = (zz.ed(e), zz.ed(f));
    e.head_port.defined == f.head_port.defined
        && ((e.head_port.p.x == f.head_port.p.x && e.head_port.p.y == f.head_port.p.y)
            || !e.head_port.defined)
        && ((e.tail_port.p.x == f.tail_port.p.x && e.tail_port.p.y == f.tail_port.p.y)
            || !e.tail_port.defined)
}

/// `expand_leaves`: places the leaf nodes. Smetana compares an edge's head rank with itself to find the
/// `ND_other` edges to restore, so it never restores any.
fn expand_leaves(zz: &mut Globals, g: GraphId) {
    make_leafslots(zz, g);
    for n in nlist(zz, g) {
        if zz.nd(n).inleaf.is_some() || zz.nd(n).outleaf.is_some() {
            unimplemented!("do_leaves");
        }
    }
}

/// `make_lrvn`: the left and right boundary nodes of `g`, kept apart by the width of its label.
fn make_lrvn(zz: &mut Globals, g: GraphId) {
    if zz.gd(g).ln.is_some() {
        return;
    }
    let root = dot_root(zz, g);
    let ln = virtual_node(zz, root);
    zz.nd_mut(ln).node_type = SLACKNODE;
    let rn = virtual_node(zz, root);
    zz.nd_mut(rn).node_type = SLACKNODE;
    if zz.gd(g).label.is_some() && g != root && !zz.gd(agroot(zz, g)).GD_flip() {
        let border = zz.gd(g).border;
        let w = max(
            border[BOTTOM_IX as usize].x as i32,
            border[TOP_IX as usize].x as i32,
        );
        make_aux_edge(zz, ln, rn, f64::from(w), 0);
    }
    zz.gd_mut(g).ln = Some(ln);
    zz.gd_mut(g).rn = Some(rn);
}

/// `contain_nodes`: keeps the nodes of every rank of cluster `g` between its boundary nodes.
fn contain_nodes(zz: &mut Globals, g: GraphId) {
    let margin = f64::from(margin_of(zz, g, 8));
    make_lrvn(zz, g);
    let (ln, rn) = boundary_nodes(zz, g);
    let border = zz.gd(g).border;
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        if zz.rank(g, r).n == 0 {
            continue;
        }
        let vlist = rank_v(zz, g, r);
        let Some(v) = zz.node_lists.get(vlist, 0) else {
            unimplemented!("contain_nodes: rank {r} missing node");
        };
        let len = zz.nd(v).lw + margin + border[LEFT_IX as usize].x;
        make_aux_edge(zz, ln, v, len, 0);
        let v = zz
            .node_lists
            .get(vlist, zz.rank(g, r).n - 1)
            .expect("node in rank");
        let len = zz.nd(v).rw + margin + border[RIGHT_IX as usize].x;
        make_aux_edge(zz, v, rn, len, 0);
    }
}

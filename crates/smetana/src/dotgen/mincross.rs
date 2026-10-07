//! `dotgen/mincross.c`: the order of the nodes within their ranks, chosen to reduce edge crossings. Each
//! connected component is ordered on its own, the components are merged, then every cluster is expanded and
//! ordered inside, and a last pass (`ReMincross`) orders the whole graph again.
//!
//! Rank arrays alias as in C: a cluster's `GD_rank(clust)[r].v` points into the root's, and `exchange` always
//! writes through the root's (`zz.Root`).

use crate::cgraph::edge::{agfstout, agnxtout};
use crate::cgraph::graph::agnedges;
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::obj::agcontains;
use crate::cgraph::{aghead, agtail};
use crate::common::utils::{agget_text, dequeue, enqueue, mapbool, new_queue, nodequeue};
use crate::core::Globals;
use crate::core::carray::CArray;
use crate::core::consts::{CLUSTER, FLATORDER, INT_MAX, NEW_RANK, NORMAL, REVERSED, VIRTUAL};
use crate::core::ids::{AdjmatrixId, EdgeId, GraphId, NodeId};
use crate::core::jutils::{atof, qsort};
use crate::dotgen::class2::class2;
use crate::dotgen::cluster::{expand_cluster, install_cluster, mark_lowclusters};
use crate::dotgen::decomp::decompose;
use crate::dotgen::dotinit::dot_root;
use crate::dotgen::fastgr::{
    EdgeList, append, delete_flat_edge, flat_edge, merge_oneway, new_virtual_edge,
};
use crate::dotgen::rank::cluster;
use crate::h::{adjmatrix_t, elist, rank_t};

/// `zz.Root`, the graph `init_mincross` started with.
fn Root(zz: &Globals) -> GraphId {
    zz.Root.expect("Root")
}

/// `GD_rank(g)[r].v`.
pub(crate) fn rank_v(zz: &Globals, g: GraphId, r: i32) -> CArray<Option<NodeId>> {
    zz.rank(g, r).v.expect("rank without nodes")
}

/// `GD_rank(g)[r].v[i]`, which must be set.
pub(crate) fn rank_node(zz: &Globals, g: GraphId, r: i32, i: i32) -> NodeId {
    zz.node_lists
        .get(rank_v(zz, g, r), i)
        .expect("node in rank")
}

/// `GD_rankleader(g)[r]`.
pub(crate) fn rankleader(zz: &Globals, g: GraphId, r: i32) -> Option<NodeId> {
    zz.node_lists
        .get(zz.gd(g).rankleader.expect("rank leaders"), r)
}

/// `dot_mincross`.
pub fn dot_mincross(zz: &mut Globals, g: GraphId) {
    init_mincross(zz, g);

    let mut c = 0;
    while c < zz.gd(g).comp.size {
        init_mccomp(zz, g, c);
        mincross_(zz, g, 0, 2);
        c += 1;
    }

    merge2(zz, g);

    // Run mincross on the contents of each cluster.
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        mincross_clust(zz, clust);
    }

    if zz.gd(g).n_cluster > 0 && agget_text(zz, g, "remincross").is_none_or(|s| mapbool(Some(&s))) {
        mark_lowclusters(zz, g);
        zz.ReMincross = true;
        mincross_(zz, g, 2, 2);
    }
    cleanup2(zz, g);
}

/// `new_matrix`: Smetana allocates a few spare rows and columns, square.
fn new_matrix(zz: &mut Globals, i: i32, j: i32) -> AdjmatrixId {
    let size = usize::try_from(i.max(j) + 8).expect("matrix size");
    zz.adjmatrices.push(adjmatrix_t {
        data: vec![vec![0; size]; size],
    })
}

/// `init_mccomp`: makes component `c` the node list of `g`, its rank arrays starting after the previous
/// components'.
fn init_mccomp(zz: &mut Globals, g: GraphId, c: i32) {
    let first = zz
        .node_lists
        .get(zz.gd(g).comp.list.expect("components"), c);
    zz.gd_mut(g).nlist = first;
    if c > 0 {
        for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
            let rank = zz.rank_mut(g, r);
            rank.v = Some(rank.v.expect("rank").plus_(rank.n));
            rank.n = 0;
        }
    }
}

/// `ordered_edges`: PlantUML sets no `ordering` attribute.
fn ordered_edges(zz: &Globals) {
    if zz.G_ordering.is_none() && zz.N_ordering.is_none() {
        return;
    }
    unimplemented!("ordered_edges");
}

/// `mincross_clust`: expands cluster `g` and orders its contents, then its sub-clusters'.
fn mincross_clust(zz: &mut Globals, g: GraphId) {
    expand_cluster(zz, g);
    ordered_edges(zz);
    flat_breakcycles(zz, g);
    flat_reorder(zz, g);
    mincross_(zz, g, 2, 2);

    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        mincross_clust(zz, clust);
    }

    save_vlist(zz, g);
}

/// `left2right`: whether `v` must stay left of `w`.
fn left2right(zz: &Globals, g: GraphId, v: NodeId, w: NodeId) -> bool {
    let (cv, cw) = (zz.nd(v).clust, zz.nd(w).clust);
    if !zz.ReMincross {
        if cv != cw && cv.is_some() && cw.is_some() {
            // Skeleton nodes of clusters may be exchanged.
            if zz.nd(v).ranktype == CLUSTER && zz.nd(v).node_type == VIRTUAL {
                return false;
            }
            if zz.nd(w).ranktype == CLUSTER && zz.nd(w).node_type == VIRTUAL {
                return false;
            }
            return true;
        }
    } else if cv != cw {
        return true;
    }
    match zz.rank(g, zz.nd(v).rank).flat {
        None => false,
        Some(M) => {
            let (v, w) = if zz.gd(g).GD_flip() { (w, v) } else { (v, w) };
            let low = |n: NodeId| usize::try_from(zz.nd(n).low).expect("flat index");
            zz.adjmatrices[M].data[low(v)][low(w)] != 0
        }
    }
}

/// `in_cross`: the crossings between the in-edges of `v` and `w` if `v` is left of `w`.
fn in_cross(zz: &Globals, v: NodeId, w: NodeId) -> i32 {
    let mut cross = 0i32;
    for e2 in zz.nd(w).in_.edges(&zz.edge_lists) {
        let cnt = zz.ed(e2).xpenalty;
        let inv = zz.nd(agtail(zz, e2)).order;
        let e2px = zz.ed(e2).tail_port.p.x;
        for e1 in zz.nd(v).in_.edges(&zz.edge_lists) {
            let t = zz.nd(agtail(zz, e1)).order - inv;
            if t > 0 || (t == 0 && zz.ed(e1).tail_port.p.x > e2px) {
                cross = cross.wrapping_add(zz.ed(e1).xpenalty.wrapping_mul(cnt));
            }
        }
    }
    cross
}

/// `out_cross`: the crossings between the out-edges of `v` and `w` if `v` is left of `w`.
fn out_cross(zz: &Globals, v: NodeId, w: NodeId) -> i32 {
    let mut cross = 0i32;
    for e2 in zz.nd(w).out.edges(&zz.edge_lists) {
        let cnt = zz.ed(e2).xpenalty;
        let inv = zz.nd(aghead(zz, e2)).order;
        let e2px = zz.ed(e2).head_port.p.x;
        for e1 in zz.nd(v).out.edges(&zz.edge_lists) {
            let t = zz.nd(aghead(zz, e1)).order - inv;
            if t > 0 || (t == 0 && zz.ed(e1).head_port.p.x > e2px) {
                cross = cross.wrapping_add(zz.ed(e1).xpenalty.wrapping_mul(cnt));
            }
        }
    }
    cross
}

/// `exchange`: swaps two nodes of a rank, in the root's rank array.
fn exchange(zz: &mut Globals, v: NodeId, w: NodeId) {
    let root = Root(zz);
    let r = zz.nd(v).rank;
    let vi = zz.nd(v).order;
    let wi = zz.nd(w).order;
    zz.nd_mut(v).order = wi;
    let vlist = rank_v(zz, root, r);
    zz.node_lists.set(vlist, wi, Some(v));
    zz.nd_mut(w).order = vi;
    zz.node_lists.set(vlist, vi, Some(w));
}

/// `transpose_step`: exchanges the neighbours of rank `r` whose exchange reduces crossings.
fn transpose_step(zz: &mut Globals, g: GraphId, r: i32, reverse: bool) -> i32 {
    let root = Root(zz);
    let mut rv = 0i32;
    zz.rank_mut(g, r).candidate = false;
    let mut i = 0;
    while i < zz.rank(g, r).n - 1 {
        let v = rank_node(zz, g, r, i);
        let w = rank_node(zz, g, r, i + 1);
        i += 1;
        if left2right(zz, g, v, w) {
            continue;
        }
        let (mut c0, mut c1) = (0i32, 0i32);
        if r > 0 {
            c0 = c0.wrapping_add(in_cross(zz, v, w));
            c1 = c1.wrapping_add(in_cross(zz, w, v));
        }
        if zz.rank(g, r + 1).n > 0 {
            c0 = c0.wrapping_add(out_cross(zz, v, w));
            c1 = c1.wrapping_add(out_cross(zz, w, v));
        }
        if c1 < c0 || (c0 > 0 && reverse && c1 == c0) {
            exchange(zz, v, w);
            rv = rv.wrapping_add(c0.wrapping_sub(c1));
            zz.rank_mut(root, r).valid = 0;
            zz.rank_mut(g, r).candidate = true;

            if r > zz.gd(g).minrank {
                zz.rank_mut(root, r - 1).valid = 0;
                zz.rank_mut(g, r - 1).candidate = true;
            }
            if r < zz.gd(g).maxrank {
                zz.rank_mut(root, r + 1).valid = 0;
                zz.rank_mut(g, r + 1).candidate = true;
            }
        }
    }
    rv
}

/// `transpose`: repeats `transpose_step` on the candidate ranks while it reduces crossings.
fn transpose(zz: &mut Globals, g: GraphId, reverse: bool) {
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        zz.rank_mut(g, r).candidate = true;
    }
    loop {
        let mut delta = 0i32;
        for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
            if zz.rank(g, r).candidate {
                delta = delta.wrapping_add(transpose_step(zz, g, r, reverse));
            }
        }
        if delta < 1 {
            break;
        }
    }
}

/// `mincross`: the passes of the ordering heuristic on `g`, from `startpass` to `endpass`, keeping the best
/// order found. Its final `balance` pass only runs with an aspect ratio, which `setAspect` rejects.
fn mincross_(zz: &mut Globals, g: GraphId, startpass: i32, endpass: i32) {
    let (mut cur_cross, mut best_cross);

    if startpass > 1 {
        cur_cross = ncross(zz);
        best_cross = cur_cross;
        save_best(zz, g);
    } else {
        cur_cross = INT_MAX;
        best_cross = INT_MAX;
    }
    for pass in startpass..=endpass {
        let maxthispass;
        if pass <= 1 {
            maxthispass = 4.min(zz.MaxIter);
            if g == dot_root(zz, g) {
                build_ranks(zz, g, pass);
            }
            if pass == 0 {
                flat_breakcycles(zz, g);
            }
            flat_reorder(zz, g);

            cur_cross = ncross(zz);
            if cur_cross <= best_cross {
                save_best(zz, g);
                best_cross = cur_cross;
            }
        } else {
            maxthispass = zz.MaxIter;
            if cur_cross > best_cross {
                restore_best(zz, g);
            }
            cur_cross = best_cross;
        }
        let mut trying = 0;
        for iter in 0..maxthispass {
            let quit = trying >= zz.MinQuit;
            trying += 1;
            if quit || cur_cross == 0 {
                break;
            }
            mincross_step(zz, g, iter);
            cur_cross = ncross(zz);
            if cur_cross <= best_cross {
                save_best(zz, g);
                if f64::from(cur_cross) < zz.Convergence * f64::from(best_cross) {
                    trying = 0;
                }
                best_cross = cur_cross;
            }
        }
        if cur_cross == 0 {
            break;
        }
    }
    if cur_cross > best_cross {
        restore_best(zz, g);
    }
    if best_cross > 0 {
        transpose(zz, g, false);
        // C recounts best_cross here; nothing reads it any more, but the count refreshes the ranks' caches.
        ncross(zz);
    }
}

/// `restore_best`: the order `save_best` saved in `ND_coord(n).x`. PlantUML walks `GD_rank(g)[r].v` rather
/// than `GD_nlist(g)`, which is empty for an expanded cluster.
fn restore_best(zz: &mut Globals, g: GraphId) {
    let root = Root(zz);
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        for i in 0..zz.rank(g, r).n {
            let n = rank_node(zz, g, r, i);
            zz.nd_mut(n).order = zz.nd(n).coord.x as i32;
        }
    }
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        zz.rank_mut(root, r).valid = 0;
        let v = rank_v(zz, g, r);
        let mut nodes = zz.node_lists.to_vec(v, zz.rank(g, r).n);
        qsort(&mut nodes, |n0, n1| nodeposcmpf(zz, n0, n1));
        zz.node_lists.copy_from(v, &nodes);
    }
}

/// `save_best`: saves the order in `ND_coord(n).x`, walking `GD_rank(g)[r].v` like `restore_best`.
fn save_best(zz: &mut Globals, g: GraphId) {
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        for i in 0..zz.rank(g, r).n {
            let n = rank_node(zz, g, r, i);
            zz.nd_mut(n).coord.x = f64::from(zz.nd(n).order);
        }
    }
}

/// `merge_components`: links the components' node lists into one.
fn merge_components(zz: &mut Globals, g: GraphId) {
    let comp = zz.gd(g).comp;
    if comp.size <= 1 {
        return;
    }
    let list = comp.list.expect("components");
    let mut u: Option<NodeId> = None;
    for c in 0..comp.size {
        let mut v = zz.node_lists.get(list, c).expect("component");
        if let Some(u) = u {
            zz.nd_mut(u).next = Some(v);
        }
        zz.nd_mut(v).prev = u;
        while let Some(next) = zz.nd(v).next {
            v = next;
        }
        u = Some(v);
    }
    zz.gd_mut(g).comp.size = 1;
    zz.gd_mut(g).nlist = zz.node_lists.get(list, 0);
    zz.gd_mut(g).minrank = zz.GlobalMinRank;
    zz.gd_mut(g).maxrank = zz.GlobalMaxRank;
}

/// `merge2`: merges the components and gives every rank its whole array back.
fn merge2(zz: &mut Globals, g: GraphId) {
    merge_components(zz, g);

    // Install the global ranks.
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        let rank = zz.rank_mut(g, r);
        rank.n = rank.an;
        rank.v = rank.av;
        for i in 0..zz.rank(g, r).n {
            match zz.node_lists.get(rank_v(zz, g, r), i) {
                None => {
                    zz.rank_mut(g, r).n = i;
                    break;
                }
                Some(v) => zz.nd_mut(v).order = i,
            }
        }
    }
}

/// `cleanup2`: final orders, the cluster rank arrays reset to their real nodes, and the ordering edges removed.
fn cleanup2(zz: &mut Globals, g: GraphId) {
    zz.TI_list = Vec::new();

    // Fix the vlists of the clusters.
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        rec_reset_vlists(zz, clust);
    }

    // Remove the node lists' FLATORDER edges.
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        for i in 0..zz.rank(g, r).n {
            let v = rank_node(zz, g, r, i);
            zz.nd_mut(v).order = i;
            if zz.nd(v).flat_out.list.is_some() {
                let mut j = 0;
                while let Some(e) = zz.nd(v).flat_out.get(&zz.edge_lists, j) {
                    if zz.ed(e).edge_type == FLATORDER {
                        delete_flat_edge(zz, e);
                        j -= 1;
                    }
                    j += 1;
                }
            }
        }
    }
}

/// `neighbor`: the node next to `v` in direction `dir` in the root's rank.
fn neighbor(zz: &Globals, v: NodeId, dir: i32) -> Option<NodeId> {
    let vlist = rank_v(zz, Root(zz), zz.nd(v).rank);
    let order = zz.nd(v).order;
    if dir < 0 {
        if order > 0 {
            return zz.node_lists.get(vlist, order - 1);
        }
        None
    } else {
        zz.node_lists.get(vlist, order + 1)
    }
}

/// `is_a_normal_node_of`.
fn is_a_normal_node_of(zz: &mut Globals, g: GraphId, v: NodeId) -> bool {
    zz.nd(v).node_type == NORMAL && agcontains(zz, g, v)
}

/// `is_a_vnode_of_an_edge_of`: whether `v` is a virtual node of a chain of an edge of `g`.
fn is_a_vnode_of_an_edge_of(zz: &mut Globals, g: GraphId, v: NodeId) -> bool {
    if zz.nd(v).node_type == VIRTUAL && zz.nd(v).in_.size == 1 && zz.nd(v).out.size == 1 {
        let mut e = zz.nd(v).out.get(&zz.edge_lists, 0).expect("out edge");
        while zz.ed(e).edge_type != NORMAL {
            e = zz.ed(e).to_orig.expect("original edge");
        }
        if agcontains(zz, g, e) {
            return true;
        }
    }
    false
}

/// `inside_cluster`.
fn inside_cluster(zz: &mut Globals, g: GraphId, v: NodeId) -> bool {
    // Both tests run, as with C's `|`.
    is_a_normal_node_of(zz, g, v) | is_a_vnode_of_an_edge_of(zz, g, v)
}

/// `furthestnode`: the furthest node of `g` from `v` in direction `dir`, in the root's rank.
fn furthestnode(zz: &mut Globals, g: GraphId, v: NodeId, dir: i32) -> NodeId {
    let mut rv = v;
    let mut u = v;
    while let Some(next) = neighbor(zz, u, dir) {
        u = next;
        if is_a_normal_node_of(zz, g, u) || is_a_vnode_of_an_edge_of(zz, g, u) {
            rv = u;
        }
    }
    rv
}

/// `save_vlist`: remembers each rank's leftmost node in `GD_rankleader`.
fn save_vlist(zz: &mut Globals, g: GraphId) {
    if let Some(leaders) = zz.gd(g).rankleader {
        for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
            let first = zz.node_lists.get(rank_v(zz, g, r), 0);
            zz.node_lists.set(leaders, r, first);
        }
    }
}

/// `rec_save_vlists`: `save_vlist` for `g` and all its clusters.
pub(crate) fn rec_save_vlists(zz: &mut Globals, g: GraphId) {
    save_vlist(zz, g);
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        rec_save_vlists(zz, clust);
    }
}

/// `rec_reset_vlists`: points every cluster's rank arrays at its nodes in the root's ranks.
pub(crate) fn rec_reset_vlists(zz: &mut Globals, g: GraphId) {
    // Fix the vlists of the sub-clusters.
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        rec_reset_vlists(zz, clust);
    }

    if let Some(leaders) = zz.gd(g).rankleader {
        for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
            let v = zz.node_lists.get(leaders, r).expect("rank leader");
            let u = furthestnode(zz, g, v, -1);
            let w = furthestnode(zz, g, v, 1);
            zz.node_lists.set(leaders, r, Some(u));
            let root = dot_root(zz, g);
            let rootv = rank_v(zz, root, r);
            let (ou, ow) = (zz.nd(u).order, zz.nd(w).order);
            let rank = zz.rank_mut(g, r);
            rank.v = Some(rootv.plus_(ou));
            rank.n = ow - ou + 1;
        }
    }
}

/// `init_mincross`.
fn init_mincross(zz: &mut Globals, g: GraphId) {
    zz.ReMincross = false;
    zz.Root = Some(g);
    let root = dot_root(zz, g);
    let size = agnedges(zz, root) + 1;
    zz.TI_list = vec![0; usize::try_from(size).expect("edge count")];
    mincross_options(zz, g);
    if (zz.gd(g).flags & NEW_RANK) != 0 {
        unimplemented!("fillRanks");
    }
    class2(zz, g);
    decompose(zz, g, 1);
    allocate_ranks(zz, g);
    ordered_edges(zz);
    zz.GlobalMinRank = zz.gd(g).minrank;
    zz.GlobalMaxRank = zz.gd(g).maxrank;
}

/// `flat_rev`: replaces a flat edge that points left by one pointing right (or merges it into an existing one).
fn flat_rev(zz: &mut Globals, g: GraphId, e: EdgeId) {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    let flat_out = zz.nd(head).flat_out;
    let rev = if flat_out.list.is_none() {
        None
    } else {
        flat_out
            .edges(&zz.edge_lists)
            .into_iter()
            .find(|&rev| aghead(zz, rev) == tail)
    };
    if let Some(rev) = rev {
        merge_oneway(zz, e, rev);
        if zz.ed(e).to_virt.is_none() {
            zz.ed_mut(e).to_virt = Some(rev);
        }
        if zz.ed(rev).edge_type == FLATORDER && zz.ed(rev).to_orig.is_none() {
            zz.ed_mut(rev).to_orig = Some(e);
        }
        append(zz, e, tail, EdgeList::Other);
    } else {
        let rev = new_virtual_edge(zz, head, tail, Some(e));
        zz.ed_mut(rev).edge_type = if zz.ed(e).edge_type == FLATORDER {
            FLATORDER
        } else {
            REVERSED
        };
        zz.ed_mut(rev).label = zz.ed(e).label;
        flat_edge(zz, g, rev);
    }
}

/// `flat_search`: depth-first search of the flat edges from `v`, recording their left-to-right constraints in
/// the rank's matrix and reversing the edges that close cycles.
fn flat_search(zz: &mut Globals, g: GraphId, v: NodeId) {
    let M = zz
        .rank(g, zz.nd(v).rank)
        .flat
        .expect("flat adjacency matrix");
    zz.nd_mut(v).mark = 1;
    zz.nd_mut(v).onstack = 1;
    let root = dot_root(zz, g);
    let hascl = zz.gd(root).n_cluster > 0;
    if zz.nd(v).flat_out.list.is_some() {
        let mut i = 0;
        while let Some(e) = zz.nd(v).flat_out.get(&zz.edge_lists, i) {
            i += 1;
            let (tail, head) = (agtail(zz, e), aghead(zz, e));
            if hascl && !(agcontains(zz, g, tail) && agcontains(zz, g, head)) {
                continue;
            }
            if zz.ed(e).weight == 0 {
                continue;
            }
            let low = |zz: &Globals, n: NodeId| usize::try_from(zz.nd(n).low).expect("flat index");
            if zz.nd(head).onstack != 0 {
                let (h, t) = (low(zz, head), low(zz, tail));
                zz.adjmatrices[M].data[h][t] = 1;
                delete_flat_edge(zz, e);
                i -= 1;
                if zz.ed(e).edge_type == FLATORDER {
                    continue;
                }
                flat_rev(zz, g, e);
            } else {
                let (t, h) = (low(zz, tail), low(zz, head));
                zz.adjmatrices[M].data[t][h] = 1;
                if zz.nd(head).mark == 0 {
                    flat_search(zz, g, head);
                }
            }
        }
    }
    zz.nd_mut(v).onstack = 0;
}

/// `flat_breakcycles`: numbers each rank's nodes (`flatindex`) and breaks the cycles of its flat edges.
fn flat_breakcycles(zz: &mut Globals, g: GraphId) {
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        let mut flat = false;
        for i in 0..zz.rank(g, r).n {
            let v = rank_node(zz, g, r, i);
            zz.nd_mut(v).mark = 0;
            zz.nd_mut(v).onstack = 0;
            zz.nd_mut(v).low = i;
            if zz.nd(v).flat_out.size > 0 && !flat {
                let n = zz.rank(g, r).n;
                let M = new_matrix(zz, n, n);
                zz.rank_mut(g, r).flat = Some(M);
                flat = true;
            }
        }
        if flat {
            for i in 0..zz.rank(g, r).n {
                let v = rank_node(zz, g, r, i);
                if zz.nd(v).mark == 0 {
                    flat_search(zz, g, v);
                }
            }
        }
    }
}

/// `allocate_ranks`: rank arrays big enough for the nodes and the virtual nodes of the edges crossing each rank.
pub(crate) fn allocate_ranks(zz: &mut Globals, g: GraphId) {
    let size = usize::try_from(zz.gd(g).maxrank + 2).expect("rank count");
    let mut cn = vec![0i32; size];
    let slot = |r: i32| usize::try_from(r).expect("rank");

    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        cn[slot(zz.nd(nn).rank)] += 1;
        let mut e = agfstout(zz, g, nn);
        while let Some(ee) = e {
            let mut low = zz.nd(agtail(zz, ee)).rank;
            let mut high = zz.nd(aghead(zz, ee)).rank;
            if low > high {
                std::mem::swap(&mut low, &mut high);
            }
            for r in low + 1..high {
                cn[slot(r)] += 1;
            }
            e = agnxtout(zz, g, ee);
        }
        n = agnxtnode(zz, g, nn);
    }
    let ranks = zz.ranks.ALLOC(zz.gd(g).maxrank + 2);
    zz.gd_mut(g).rank = Some(ranks);
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        let count = cn[slot(r)];
        let tmp = zz.node_lists.ALLOC(count + 1);
        *zz.rank_mut(g, r) = rank_t {
            n: count,
            an: count,
            v: Some(tmp),
            av: Some(tmp),
            ..rank_t::default()
        };
    }
}

/// `install_in_rank`: appends `n` to its rank in `g`.
pub(crate) fn install_in_rank(zz: &mut Globals, g: GraphId, n: NodeId) {
    let root = Root(zz);
    let r = zz.nd(n).rank;
    let i = zz.rank(g, r).n;
    if zz.rank(g, r).an <= 0 {
        unimplemented!("install_in_rank: rank {r} has no room");
    }

    zz.node_lists.set(rank_v(zz, g, r), i, Some(n));
    zz.nd_mut(n).order = i;
    zz.rank_mut(g, r).n += 1;
    if zz.nd(n).order > zz.rank(root, r).an {
        unimplemented!("install_in_rank: order beyond the root's rank");
    }
    if r < zz.gd(g).minrank || r > zz.gd(g).maxrank {
        unimplemented!("install_in_rank: rank {r} out of range");
    }
    let end = zz.rank(g, r).av.expect("rank").plus_(zz.rank(root, r).an);
    if rank_v(zz, g, r).plus_(zz.nd(n).order).compare_pointer_(end) > 0 {
        unimplemented!("install_in_rank: past the end of the rank array");
    }
}

/// `build_ranks`: an initial order, by breadth-first search from the sources (pass 0) or sinks (pass 1).
pub(crate) fn build_ranks(zz: &mut Globals, g: GraphId, pass: i32) {
    let root = Root(zz);
    let mut q = new_queue(zz.gd(g).n_nodes);
    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        zz.nd_mut(nn).mark = 0;
        n = zz.nd(nn).next;
    }

    for i in zz.gd(g).minrank..=zz.gd(g).maxrank {
        zz.rank_mut(g, i).n = 0;
    }

    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        n = zz.nd(nn).next;
        let otheredges = if pass == 0 {
            zz.nd(nn).in_
        } else {
            zz.nd(nn).out
        };
        if otheredges.get(&zz.edge_lists, 0).is_some() {
            continue;
        }
        if zz.nd(nn).mark == 0 {
            zz.nd_mut(nn).mark = 1;
            enqueue(&mut q, nn);
            while let Some(n0) = dequeue(&mut q) {
                if zz.nd(n0).ranktype == CLUSTER {
                    install_cluster(zz, g, n0, pass, &mut q);
                } else {
                    install_in_rank(zz, g, n0);
                    enqueue_neighbors(zz, &mut q, n0, pass);
                }
            }
        }
    }
    if dequeue(&mut q).is_some() {
        unimplemented!("build_ranks: surprise");
    }
    for i in zz.gd(g).minrank..=zz.gd(g).maxrank {
        zz.rank_mut(root, i).valid = 0;
        if zz.gd(g).GD_flip() && zz.rank(g, i).n > 0 {
            let vlist = rank_v(zz, g, i);
            let nn = zz.rank(g, i).n - 1;
            let ndiv2 = nn / 2;
            for j in 0..=ndiv2 {
                let v = zz.node_lists.get(vlist, j).expect("node");
                let w = zz.node_lists.get(vlist, nn - j).expect("node");
                exchange(zz, v, w);
            }
        }
    }

    if g == dot_root(zz, g) && ncross(zz) > 0 {
        transpose(zz, g, false);
    }
}

/// `enqueue_neighbors`: queues the unmarked heads (pass 0) or tails (pass 1) of `n0`'s edges.
pub(crate) fn enqueue_neighbors(zz: &mut Globals, q: &mut nodequeue, n0: NodeId, pass: i32) {
    if pass == 0 {
        for i in 0..zz.nd(n0).out.size {
            let e = zz.nd(n0).out.get(&zz.edge_lists, i).expect("out edge");
            let head = aghead(zz, e);
            if zz.nd(head).mark == 0 {
                zz.nd_mut(head).mark = 1;
                enqueue(q, head);
            }
        }
    } else {
        for i in 0..zz.nd(n0).in_.size {
            let e = zz.nd(n0).in_.get(&zz.edge_lists, i).expect("in edge");
            let tail = agtail(zz, e);
            if zz.nd(tail).mark == 0 {
                zz.nd_mut(tail).mark = 1;
                enqueue(q, tail);
            }
        }
    }
}

/// `constraining_flat_edge`: whether a flat edge must point left to right: it has weight and both its ends
/// belong to `g`.
fn constraining_flat_edge(zz: &mut Globals, g: GraphId, e: EdgeId) -> bool {
    if zz.ed(e).weight == 0 {
        return false;
    }
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    inside_cluster(zz, g, tail) && inside_cluster(zz, g, head)
}

/// `postorder`: writes the nodes reachable from `v` by constraining flat edges to `list`, in postorder, and
/// returns their number.
fn postorder(zz: &mut Globals, g: GraphId, v: NodeId, list: CArray<Option<NodeId>>) -> i32 {
    let mut cnt = 0;
    zz.nd_mut(v).mark = 1;
    if zz.nd(v).flat_out.size > 0 {
        let mut i = 0;
        while let Some(e) = zz.nd(v).flat_out.get(&zz.edge_lists, i) {
            i += 1;
            if !constraining_flat_edge(zz, g, e) {
                continue;
            }
            let head = aghead(zz, e);
            if zz.nd(head).mark == 0 {
                cnt += postorder(zz, g, head, list.plus_(cnt));
            }
        }
    }
    zz.node_lists.set(list, cnt, Some(v));
    cnt + 1
}

/// `flat_reorder`: orders each rank so that its constraining flat edges point left to right, and reverses the
/// others that do not.
fn flat_reorder(zz: &mut Globals, g: GraphId) {
    if zz.gd(g).has_flat_edges == 0 {
        return;
    }
    let root = Root(zz);
    let flip = zz.gd(g).GD_flip();
    let mut temprank: Option<CArray<Option<NodeId>>> = None;
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        let n = zz.rank(g, r).n;
        if n == 0 {
            continue;
        }
        let base_order = zz.nd(rank_node(zz, g, r, 0)).order;
        for i in 0..n {
            let v = rank_node(zz, g, r, i);
            zz.nd_mut(v).mark = 0;
        }
        let tr = zz.node_lists.REALLOC(n + 1, temprank);
        temprank = Some(tr);
        let mut pos = 0;

        // Construct the reverse topological sort order in temprank.
        for i in 0..n {
            let v = if flip {
                rank_node(zz, g, r, i)
            } else {
                rank_node(zz, g, r, n - i - 1)
            };

            let mut local_in_cnt = 0;
            let mut local_out_cnt = 0;
            for j in 0..zz.nd(v).flat_in.size {
                let flat_e = zz.nd(v).flat_in.get(&zz.edge_lists, j).expect("flat edge");
                if constraining_flat_edge(zz, g, flat_e) {
                    local_in_cnt += 1;
                }
            }
            for j in 0..zz.nd(v).flat_out.size {
                let flat_e = zz.nd(v).flat_out.get(&zz.edge_lists, j).expect("flat edge");
                if constraining_flat_edge(zz, g, flat_e) {
                    local_out_cnt += 1;
                }
            }
            if local_in_cnt == 0 && local_out_cnt == 0 {
                zz.node_lists.set(tr, pos, Some(v));
                pos += 1;
            } else if zz.nd(v).mark == 0 && local_in_cnt == 0 {
                let left = tr.plus_(pos);
                pos += postorder(zz, g, v, left);
            }
        }

        if pos != 0 {
            if !flip {
                let (mut left, mut right) = (0, pos - 1);
                while left < right {
                    zz.node_lists.swap(tr, left, right);
                    left += 1;
                    right -= 1;
                }
            }
            for i in 0..n {
                let v = zz.node_lists.get(tr, i).expect("ordered node");
                zz.node_lists.set(rank_v(zz, g, r), i, Some(v));
                zz.nd_mut(v).order = i + base_order;
            }

            // Nonconstraint flat edges must be made left to right.
            for i in 0..n {
                let v = rank_node(zz, g, r, i);
                if zz.nd(v).flat_out.list.is_some() {
                    let mut j = 0;
                    while let Some(e) = zz.nd(v).flat_out.get(&zz.edge_lists, j) {
                        j += 1;
                        let (ho, to) = (zz.nd(aghead(zz, e)).order, zz.nd(agtail(zz, e)).order);
                        if (!flip && ho < to) || (flip && ho > to) {
                            delete_flat_edge(zz, e);
                            j -= 1;
                            flat_rev(zz, g, e);
                        }
                    }
                }
            }
        }
        zz.rank_mut(root, r).valid = 0;
    }
}

/// `reorder`: bubbles the nodes of rank `r` towards their median values, keeping the left-to-right constraints.
fn reorder(zz: &mut Globals, g: GraphId, r: i32, reverse: bool, hasfixed: bool) {
    let root = Root(zz);
    let mut changed = false;
    let vlist = rank_v(zz, g, r);
    let node = |zz: &Globals, i: i32| zz.node_lists.get(vlist, i).expect("node");
    let mut ep = zz.rank(g, r).n;

    let mut nelt = zz.rank(g, r).n - 1;
    while nelt >= 0 {
        let mut lp = 0;
        while lp < ep {
            // Find the leftmost node that can be compared.
            while lp < ep && zz.nd(node(zz, lp)).mval < 0.0 {
                lp += 1;
            }
            if lp >= ep {
                break;
            }
            // Find the node that can be compared.
            let mut sawclust = false;
            let mut muststay = false;
            let mut rp = lp + 1;
            while rp < ep {
                if sawclust && zz.nd(node(zz, rp)).clust.is_some() {
                    rp += 1;
                    continue;
                }
                if left2right(zz, g, node(zz, lp), node(zz, rp)) {
                    muststay = true;
                    break;
                }
                if zz.nd(node(zz, rp)).mval >= 0.0 {
                    break;
                }
                if zz.nd(node(zz, rp)).clust.is_some() {
                    sawclust = true;
                }
                rp += 1;
            }
            if rp >= ep {
                break;
            }
            if !muststay {
                let p1 = zz.nd(node(zz, lp)).mval as i32;
                let p2 = zz.nd(node(zz, rp)).mval as i32;
                if p1 > p2 || (p1 == p2 && reverse) {
                    exchange(zz, node(zz, lp), node(zz, rp));
                    changed = true;
                }
            }
            lp = rp;
        }
        if !hasfixed && !reverse {
            ep -= 1;
        }
        nelt -= 1;
    }

    if changed {
        zz.rank_mut(root, r).valid = 0;
        if r > 0 {
            zz.rank_mut(root, r - 1).valid = 0;
        }
    }
}

/// `mincross_step`: one down (even `pass`) or up (odd) sweep of median reordering, then transposition.
fn mincross_step(zz: &mut Globals, g: GraphId, pass: i32) {
    let root = Root(zz);
    let reverse = (pass % 4) < 2;
    let (minrank, maxrank) = (zz.gd(g).minrank, zz.gd(g).maxrank);

    let (first, last, dir) = if pass % 2 == 0 {
        // Downwards, each rank follows the one above, which the root's top rank does not have.
        let first = if minrank > zz.gd(root).minrank {
            minrank
        } else {
            minrank + 1
        };
        (first, maxrank, 1)
    } else {
        // Upwards, each rank follows the one below, which the root's bottom rank does not have.
        let first = if maxrank < zz.gd(root).maxrank {
            maxrank
        } else {
            maxrank - 1
        };
        (first, minrank, -1)
    };

    let mut r = first;
    while r != last + dir {
        let other = r - dir;
        let hasfixed = medians(zz, g, r, other);
        reorder(zz, g, r, reverse, hasfixed);
        r += dir;
    }
    transpose(zz, g, !reverse);
}

/// `local_cross`: the crossings among the edges of one node with ports. Smetana only ports the test, and
/// throws where Graphviz would count a crossing.
fn local_cross(zz: &Globals, l: elist, dir: i32) {
    let is_out = dir > 0;
    let edges = l.edges(&zz.edge_lists);
    for (i, &e) in edges.iter().enumerate() {
        if is_out {
            for &f in &edges[i + 1..] {
                let dorder = zz.nd(aghead(zz, f)).order - zz.nd(aghead(zz, e)).order;
                let dx = zz.ed(f).tail_port.p.x - zz.ed(e).tail_port.p.x;
                if f64::from(dorder) * dx < 0.0 {
                    unimplemented!("local_cross: crossing out-edges");
                }
            }
        } else if i + 1 < edges.len() {
            unimplemented!("local_cross: several in-edges");
        }
    }
}

/// `rcross`: the crossings between ranks `r` and `r + 1` of `g`.
fn rcross(zz: &mut Globals, g: GraphId, r: i32) -> i32 {
    let root = Root(zz);
    let mut cross = 0i32;
    let mut max = 0;
    let rtop = rank_v(zz, g, r);

    if zz.C <= zz.rank(root, r + 1).n {
        zz.C = zz.rank(root, r + 1).n + 1;
        zz.Count
            .resize(usize::try_from(zz.C).expect("rank size"), 0);
    }

    for i in 0..zz.rank(g, r + 1).n {
        zz.Count[usize::try_from(i).expect("index")] = 0;
    }

    let slot = |k: i32| usize::try_from(k).expect("order");
    for top in 0..zz.rank(g, r).n {
        let v = zz.node_lists.get(rtop, top).expect("node");
        let outlist = zz.nd(v).out.edges(&zz.edge_lists);
        if max > 0 {
            for &e in &outlist {
                for k in zz.nd(aghead(zz, e)).order + 1..=max {
                    cross = cross.wrapping_add(zz.Count[slot(k)].wrapping_mul(zz.ed(e).xpenalty));
                }
            }
        }
        for &e in &outlist {
            let inv = zz.nd(aghead(zz, e)).order;
            if inv > max {
                max = inv;
            }
            zz.Count[slot(inv)] = zz.Count[slot(inv)].wrapping_add(zz.ed(e).xpenalty);
        }
    }
    for top in 0..zz.rank(g, r).n {
        let v = rank_node(zz, g, r, top);
        if zz.nd(v).has_port {
            local_cross(zz, zz.nd(v).out, 1);
        }
    }
    for bot in 0..zz.rank(g, r + 1).n {
        let v = rank_node(zz, g, r + 1, bot);
        if zz.nd(v).has_port {
            local_cross(zz, zz.nd(v).in_, -1);
        }
    }
    cross
}

/// `ncross`: the crossings of the root graph, from the ranks' caches where they are valid.
fn ncross(zz: &mut Globals) -> i32 {
    let g = Root(zz);
    let mut count = 0i32;
    for r in zz.gd(g).minrank..zz.gd(g).maxrank {
        if zz.rank(g, r).valid != 0 {
            count = count.wrapping_add(zz.rank(g, r).cache_nc);
        } else {
            let nc = rcross(zz, g, r);
            let rank = zz.rank_mut(g, r);
            rank.cache_nc = nc;
            rank.valid = 1;
            count = count.wrapping_add(nc);
        }
    }
    count
}

/// `ordercmpf`.
fn ordercmpf(i0: i32, i1: i32) -> i32 {
    i0.wrapping_sub(i1)
}

/// `flat_mval`: the median value of a node with only flat edges, next to its flat neighbour's. Returns whether
/// it has none.
fn flat_mval(zz: &mut Globals, n: NodeId) -> bool {
    if zz.nd(n).flat_in.size > 0 {
        let fl = zz.nd(n).flat_in.edges(&zz.edge_lists);
        let mut nn = agtail(zz, fl[0]);
        for &e in &fl[1..] {
            if zz.nd(agtail(zz, e)).order > zz.nd(nn).order {
                nn = agtail(zz, e);
            }
        }
        if zz.nd(nn).mval >= 0.0 {
            zz.nd_mut(n).mval = zz.nd(nn).mval + 1.0;
            return false;
        }
    } else if zz.nd(n).flat_out.size > 0 {
        let fl = zz.nd(n).flat_out.edges(&zz.edge_lists);
        let mut nn = aghead(zz, fl[0]);
        for &e in &fl[1..] {
            if zz.nd(aghead(zz, e)).order < zz.nd(nn).order {
                nn = aghead(zz, e);
            }
        }
        if zz.nd(nn).mval > 0.0 {
            zz.nd_mut(n).mval = zz.nd(nn).mval - 1.0;
            return false;
        }
    }
    true
}

/// `medians`: each node's weighted median position (`ND_mval`) among its neighbours on rank `r1`. Returns
/// whether some node has no neighbours there and no flat neighbour with a median.
fn medians(zz: &mut Globals, g: GraphId, r0: i32, r1: i32) -> bool {
    let mut hasfixed = false;
    let mut list = std::mem::take(&mut zz.TI_list);
    let v = rank_v(zz, g, r0);
    for i in 0..zz.rank(g, r0).n {
        let n = zz.node_lists.get(v, i).expect("node");
        let mut j = 0;
        if r1 > r0 {
            for e in zz.nd(n).out.edges(&zz.edge_lists) {
                if zz.ed(e).xpenalty > 0 {
                    list[j] = 256 * zz.nd(aghead(zz, e)).order + zz.ed(e).head_port.order;
                    j += 1;
                }
            }
        } else {
            for e in zz.nd(n).in_.edges(&zz.edge_lists) {
                if zz.ed(e).xpenalty > 0 {
                    list[j] = 256 * zz.nd(agtail(zz, e)).order + zz.ed(e).tail_port.order;
                    j += 1;
                }
            }
        }
        let mval = match j {
            0 => -1,
            1 => list[0],
            2 => i32::midpoint(list[0], list[1]),
            _ => {
                qsort(&mut list[..j], ordercmpf);
                if j % 2 != 0 {
                    list[j / 2]
                } else {
                    // Weighted median.
                    let rm = j / 2;
                    let lm = rm - 1;
                    let rspan = list[j - 1] - list[rm];
                    let lspan = list[lm] - list[0];
                    if lspan == rspan {
                        i32::midpoint(list[lm], list[rm])
                    } else {
                        let w = list[lm]
                            .wrapping_mul(rspan)
                            .wrapping_add(list[rm].wrapping_mul(lspan));
                        w / (lspan + rspan)
                    }
                }
            }
        };
        zz.nd_mut(n).mval = f64::from(mval);
    }
    zz.TI_list = list;
    for i in 0..zz.rank(g, r0).n {
        let n = zz.node_lists.get(v, i).expect("node");
        if zz.nd(n).out.size == 0 && zz.nd(n).in_.size == 0 {
            hasfixed |= flat_mval(zz, n);
        }
    }
    hasfixed
}

/// `nodeposcmpf`.
fn nodeposcmpf(zz: &Globals, n0: Option<NodeId>, n1: Option<NodeId>) -> i32 {
    let order = |n: Option<NodeId>| zz.nd(n.expect("node")).order;
    order(n0).wrapping_sub(order(n1))
}

/// `table`: the weight factor of a virtual edge by the classes of its ends: ordinary, singleton or virtual.
#[allow(non_upper_case_globals, reason = "Graphviz's name")]
const table: [[i32; 3]; 3] = [[1, 1, 1], [1, 2, 2], [1, 2, 4]];

/// `endpoint_class`.
fn endpoint_class(zz: &Globals, n: NodeId) -> usize {
    if zz.nd(n).node_type == VIRTUAL {
        return 2;
    }
    if zz.nd(n).weight_class <= 1 {
        return 1;
    }
    0
}

/// `virtual_weight`: weighs a virtual edge by the classes of its ends, to straighten long edges.
pub(crate) fn virtual_weight(zz: &mut Globals, e: EdgeId) {
    let t = table[endpoint_class(zz, agtail(zz, e))][endpoint_class(zz, aghead(zz, e))];
    zz.ed_mut(e).weight = zz.ed(e).weight.wrapping_mul(t);
}

/// `mincross_options`.
fn mincross_options(zz: &mut Globals, g: GraphId) {
    zz.MinQuit = 8;
    zz.MaxIter = 24;
    zz.Convergence = 0.995;

    if let Some(p) = agget_text(zz, g, "mclimit")
        && atof(&p) > 0.0
    {
        unimplemented!("mclimit");
    }
}

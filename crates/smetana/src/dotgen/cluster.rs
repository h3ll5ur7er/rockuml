//! `dotgen/cluster.c`: clusters in the fast graph. A cluster first takes part in mincross as a skeleton, a chain
//! of virtual rank leaders; `expand_cluster` then replaces the skeleton by the cluster's contents.

use crate::cgraph::edge::{agfstedge, agfstout, agnxtedge, agnxtout};
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::obj::{agcontains, agroot};
use crate::cgraph::{AGMKOUT, aghead, agtail};
use crate::common::utils::{UF_setname, UF_singleton, nodequeue};
use crate::core::Globals;
use crate::core::consts::{CL_CROSS, CLUSTER, CLUSTER_EDGE, NORMAL, VIRTUAL};
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::dotgen::class2::{class2, merge_chain, mergeable};
use crate::dotgen::dotinit::dot_root;
use crate::dotgen::fastgr::{
    delete_fast_edge, delete_fast_node, fast_node, find_fast_edge, find_flat_edge, flat_edge,
    merge_oneway, other_edge, safe_other_edge, virtual_edge, virtual_node,
};
use crate::dotgen::mincross::{
    allocate_ranks, build_ranks, enqueue_neighbors, install_in_rank, rank_node, rank_v, rankleader,
};
use crate::dotgen::position::ports_eq;
use crate::dotgen::rank::cluster;

/// `map_interclust_node`: the node an inter-cluster edge attaches to: the node itself if its cluster is
/// expanded, otherwise its cluster's rank leader.
fn map_interclust_node(zz: &Globals, n: NodeId) -> NodeId {
    match zz.nd(n).clust {
        Some(clust) if !zz.gd(clust).expanded => {
            rankleader(zz, clust, zz.nd(n).rank).expect("rank leader")
        }
        _ => n,
    }
}

/// `make_slots`: makes `d` slots in rank `r` of `root` starting at position `pos`, where one already exists.
fn make_slots(zz: &mut Globals, root: GraphId, r: i32, pos: i32, d: i32) {
    let vlist = rank_v(zz, root, r);
    let n = zz.rank(root, r).n;
    if d <= 0 {
        for i in pos - d + 1..n {
            let v = zz.node_lists.get(vlist, i).expect("node");
            zz.nd_mut(v).order = i + d - 1;
            zz.node_lists.set(vlist, i + d - 1, Some(v));
        }
        for i in n + d - 1..n {
            zz.node_lists.set(vlist, i, None);
        }
    } else {
        for i in (pos + 1..n).rev() {
            let v = zz.node_lists.get(vlist, i).expect("node");
            zz.nd_mut(v).order = i + d - 1;
            zz.node_lists.set(vlist, i + d - 1, Some(v));
        }
        for i in pos + 1..pos + d {
            zz.node_lists.set(vlist, i, None);
        }
    }
    zz.rank_mut(root, r).n = n + d - 1;
}

/// `clone_vn`: Smetana does not port it.
fn clone_vn() -> NodeId {
    unimplemented!("clone_vn")
}

/// `map_path`: moves the chain `ve` of `orig` to run from `from` to `to`.
fn map_path(zz: &mut Globals, from: NodeId, to: NodeId, orig: EdgeId, ve: EdgeId, kind: i32) {
    if agtail(zz, ve) == from && aghead(zz, ve) == to {
        return;
    }
    let (from_rank, to_rank) = (zz.nd(from).rank, zz.nd(to).rank);
    let both_normal =
        |zz: &Globals| zz.nd(from).node_type == NORMAL && zz.nd(to).node_type == NORMAL;
    if zz.ed(ve).count > 1 {
        zz.ed_mut(orig).to_virt = None;
        if to_rank - from_rank == 1
            && let Some(e) = find_fast_edge(zz, from, to)
            && ports_eq(zz, orig, e)
        {
            merge_oneway(zz, orig, e);
            if both_normal(zz) {
                other_edge(zz, orig);
            }
            return;
        }
        let mut u = from;
        let mut ve = Some(ve);
        for r in from_rank..to_rank {
            let v = if r < to_rank - 1 { clone_vn() } else { to };
            let e = virtual_edge(zz, u, v, Some(orig));
            zz.ed_mut(e).edge_type = kind;
            u = v;
            let cur = ve.expect("virtual edge");
            zz.ed_mut(cur).count -= 1;
            ve = zz.nd(aghead(zz, cur)).out.get(&zz.edge_lists, 0);
        }
    } else {
        let mut ve = ve;
        if to_rank - from_rank == 1 {
            match find_fast_edge(zz, from, to) {
                Some(found) if ports_eq(zz, orig, found) => {
                    ve = found;
                    zz.ed_mut(orig).to_virt = Some(ve);
                    zz.ed_mut(ve).edge_type = kind;
                    zz.ed_mut(ve).count += 1;
                    if both_normal(zz) {
                        other_edge(zz, orig);
                    }
                }
                _ => {
                    zz.ed_mut(orig).to_virt = None;
                    ve = virtual_edge(zz, from, to, Some(orig));
                    zz.ed_mut(ve).edge_type = kind;
                }
            }
        }
        if to_rank - from_rank > 1 {
            let mut e = if agtail(zz, ve) == from {
                ve
            } else {
                zz.ed_mut(orig).to_virt = None;
                let head = aghead(zz, ve);
                let e = virtual_edge(zz, from, head, Some(orig));
                zz.ed_mut(orig).to_virt = Some(e);
                delete_fast_edge(zz, ve);
                e
            };
            while zz.nd(aghead(zz, e)).rank != to_rank {
                e = zz
                    .nd(aghead(zz, e))
                    .out
                    .get(&zz.edge_lists, 0)
                    .expect("chain edge");
            }
            if aghead(zz, e) != to {
                let tail = agtail(zz, e);
                let last = virtual_edge(zz, tail, to, Some(orig));
                zz.ed_mut(last).edge_type = kind;
                delete_fast_edge(zz, e);
            }
        }
    }
}

/// `make_interclust_chain`.
fn make_interclust_chain(zz: &mut Globals, from: NodeId, to: NodeId, orig: EdgeId) {
    let u = map_interclust_node(zz, from);
    let v = map_interclust_node(zz, to);
    let newtype = if u == from && v == to {
        VIRTUAL
    } else {
        CLUSTER_EDGE
    };
    let ve = zz.ed(orig).to_virt.expect("virtual edge");
    map_path(zz, u, v, orig, ve, newtype);
}

/// `interclexp`: attaches and installs the edges between `subg` and the rest of the graph: `class2` for
/// inter-cluster edges.
fn interclexp(zz: &mut Globals, subg: GraphId) {
    let g = dot_root(zz, subg);
    let mut n = agfstnode(zz, subg);
    while let Some(nn) = n {
        // N.B. nn may be in a sub-cluster of subg.
        let mut prev = None;
        let mut e = agfstedge(zz, g, nn);
        while let Some(ee) = e {
            let next = agnxtedge(zz, g, ee, nn);
            if !agcontains(zz, subg, ee) {
                let out = AGMKOUT(zz, ee);
                interclexp_edge(zz, g, subg, out, &mut prev);
            }
            e = next;
        }
        n = agnxtnode(zz, subg, nn);
    }
}

/// The body of `interclexp`'s edge loop, for an edge `e` (canonicalised) leaving `subg`.
fn interclexp_edge(
    zz: &mut Globals,
    g: GraphId,
    subg: GraphId,
    e: EdgeId,
    prev: &mut Option<EdgeId>,
) {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    // Short/flat multi edges.
    if mergeable(zz, *prev, e) {
        let p = prev.expect("previous edge");
        zz.ed_mut(e).to_virt = if zz.nd(tail).rank == zz.nd(head).rank {
            Some(p)
        } else {
            None
        };
        // Without a chain, an internal edge.
        if let Some(pv) = zz.ed(p).to_virt {
            merge_chain(zz, subg, e, pv, false);
            safe_other_edge(zz, e);
        }
        return;
    }

    // Flat edges.
    if zz.nd(tail).rank == zz.nd(head).rank {
        match find_flat_edge(zz, tail, head) {
            None => {
                flat_edge(zz, g, e);
                *prev = Some(e);
            }
            Some(fe) if e != fe => {
                safe_other_edge(zz, e);
                if zz.ed(e).to_virt.is_none() {
                    merge_oneway(zz, e, fe);
                }
            }
            Some(_) => {}
        }
        return;
    }

    // Forward edges, then backward ones.
    if zz.nd(head).rank > zz.nd(tail).rank {
        make_interclust_chain(zz, tail, head, e);
    } else {
        make_interclust_chain(zz, head, tail, e);
    }
    *prev = Some(e);
}

/// `merge_ranks`: moves the ordered contents of `subg` into the root's ranks, in place of its rank leaders.
/// The cluster's rank arrays then point into the root's.
fn merge_ranks(zz: &mut Globals, subg: GraphId) {
    let root = dot_root(zz, subg);
    if zz.gd(subg).minrank > 0 {
        zz.rank_mut(root, zz.gd(subg).minrank - 1).valid = 0;
    }
    let mut r = zz.gd(subg).minrank;
    while r <= zz.gd(subg).maxrank {
        let d = zz.rank(subg, r).n;
        let leader = rankleader(zz, subg, r).expect("rank leader");
        let ipos = zz.nd(leader).order;
        make_slots(zz, root, r, ipos, d);
        for (pos, i) in (ipos..).zip(0..zz.rank(subg, r).n) {
            let v = rank_node(zz, subg, r, i);
            zz.node_lists.set(rank_v(zz, root, r), pos, Some(v));
            zz.nd_mut(v).order = pos;
            // Real nodes automatically have v->root = root graph.
            if zz.nd(v).node_type == VIRTUAL {
                zz.nodes[v].root = agroot(zz, root);
            }
            delete_fast_node(zz, subg, v);
            fast_node(zz, root, v);
            zz.gd_mut(root).n_nodes += 1;
        }
        let rootv = rank_v(zz, root, r);
        zz.rank_mut(subg, r).v = Some(rootv.plus_(ipos));
        zz.rank_mut(root, r).valid = 0;
        r += 1;
    }
    if r < zz.gd(root).maxrank {
        zz.rank_mut(root, r).valid = 0;
    }
    zz.gd_mut(subg).expanded = true;
}

/// `remove_rankleaders`: deletes the skeleton of `g`.
fn remove_rankleaders(zz: &mut Globals, g: GraphId) {
    let leaders = zz.gd(g).rankleader.expect("rank leaders");
    for r in zz.gd(g).minrank..=zz.gd(g).maxrank {
        let v = zz.node_lists.get(leaders, r).expect("rank leader");

        // Remove the entire chain.
        while let Some(e) = zz.nd(v).out.get(&zz.edge_lists, 0) {
            delete_fast_edge(zz, e);
        }
        while let Some(e) = zz.nd(v).in_.get(&zz.edge_lists, 0) {
            delete_fast_edge(zz, e);
        }
        let root = dot_root(zz, g);
        delete_fast_node(zz, root, v);
        zz.node_lists.set(leaders, r, None);
    }
}

/// `expand_cluster`: replaces the skeleton of `subg` by its nodes and sub-cluster skeletons, in a first order.
pub(crate) fn expand_cluster(zz: &mut Globals, subg: GraphId) {
    // Build the internal structure of the cluster.
    class2(zz, subg);
    zz.gd_mut(subg).comp.size = 1;
    let nlist = zz.gd(subg).nlist;
    let comp = zz.gd(subg).comp.list.expect("components");
    zz.node_lists.set(comp, 0, nlist);
    allocate_ranks(zz, subg);
    build_ranks(zz, subg, 0);
    merge_ranks(zz, subg);

    // Build the external structure of the cluster.
    interclexp(zz, subg);
    remove_rankleaders(zz, subg);
}

/// `mark_clusters`: marks every node of `g` with its top-level cluster under `g` (`ND_clust`), merges it into
/// the cluster's leader, and marks the virtual nodes of the cluster's edges too.
pub(crate) fn mark_clusters(zz: &mut Globals, g: GraphId) {
    // Remove the sub-clusters below this level.
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        if zz.nd(nn).ranktype == CLUSTER {
            UF_singleton(zz, nn);
        }
        zz.nd_mut(nn).clust = None;
        n = agnxtnode(zz, g, nn);
    }

    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        let mut n = agfstnode(zz, clust);
        while let Some(nn) = n {
            let nn_next = agnxtnode(zz, clust, nn);
            if zz.nd(nn).ranktype != NORMAL {
                unimplemented!("a node in a rankset and a cluster");
            }
            let leader = zz.gd(clust).leader.expect("cluster leader");
            UF_setname(zz, nn, leader);
            zz.nd_mut(nn).clust = Some(clust);
            zz.nd_mut(nn).ranktype = CLUSTER;

            // Mark the virtual nodes of the cluster's edges. Trouble if concentrators and clusters are mixed.
            let mut orig = agfstout(zz, clust, nn);
            while let Some(o) = orig {
                mark_chain(zz, o, Some(clust), false);
                orig = agnxtout(zz, clust, o);
            }
            n = nn_next;
        }
    }
}

/// `build_skeleton`: a chain of virtual rank leaders standing for `subg` in `g`, one per rank.
pub(crate) fn build_skeleton(zz: &mut Globals, g: GraphId, subg: GraphId) {
    let mut prev: Option<NodeId> = None;
    let leaders = zz.node_lists.ALLOC(zz.gd(subg).maxrank + 2);
    zz.gd_mut(subg).rankleader = Some(leaders);
    for r in zz.gd(subg).minrank..=zz.gd(subg).maxrank {
        let v = virtual_node(zz, g);
        zz.node_lists.set(leaders, r, Some(v));
        let info = zz.nd_mut(v);
        info.rank = r;
        info.ranktype = CLUSTER;
        info.clust = Some(subg);
        if let Some(prev) = prev {
            let e = virtual_edge(zz, prev, v, None);
            zz.ed_mut(e).xpenalty = zz.ed(e).xpenalty.wrapping_mul(CL_CROSS);
        }
        prev = Some(v);
    }

    // Set the counts on the virtual edges of the cluster skeleton.
    let mut v = agfstnode(zz, subg);
    while let Some(vv) = v {
        let rl = zz
            .node_lists
            .get(leaders, zz.nd(vv).rank)
            .expect("rank leader");
        zz.nd_mut(rl).UF_size += 1;
        let mut e = agfstout(zz, subg, vv);
        while let Some(ee) = e {
            for _ in zz.nd(agtail(zz, ee)).rank..zz.nd(aghead(zz, ee)).rank {
                let skeleton = zz.nd(rl).out.get(&zz.edge_lists, 0).expect("skeleton edge");
                zz.ed_mut(skeleton).count += 1;
            }
            e = agnxtout(zz, subg, ee);
        }
        v = agnxtnode(zz, subg, vv);
    }
    for r in zz.gd(subg).minrank..=zz.gd(subg).maxrank {
        let rl = zz.node_lists.get(leaders, r).expect("rank leader");
        if zz.nd(rl).UF_size > 1 {
            zz.nd_mut(rl).UF_size -= 1;
        }
    }
}

/// `install_cluster`: installs the skeleton of `n`'s cluster in `g`'s ranks, once per pass.
pub(crate) fn install_cluster(
    zz: &mut Globals,
    g: GraphId,
    n: NodeId,
    pass: i32,
    q: &mut nodequeue,
) {
    let clust = zz.nd(n).clust.expect("cluster");
    if zz.gd(clust).installed != pass + 1 {
        let (minrank, maxrank) = (zz.gd(clust).minrank, zz.gd(clust).maxrank);
        for r in minrank..=maxrank {
            let leader = rankleader(zz, clust, r).expect("rank leader");
            install_in_rank(zz, g, leader);
        }
        for r in minrank..=maxrank {
            let leader = rankleader(zz, clust, r).expect("rank leader");
            enqueue_neighbors(zz, q, leader, pass);
        }
        zz.gd_mut(clust).installed = pass + 1;
    }
}

/// Sets `ND_clust` of the virtual nodes of `orig`'s chain to `clust`; with `only_unset`, only where it is
/// not set yet.
fn mark_chain(zz: &mut Globals, orig: EdgeId, clust: Option<GraphId>, only_unset: bool) {
    let mut e = zz.ed(orig).to_virt;
    while let Some(ee) = e {
        let vn = aghead(zz, ee);
        if zz.nd(vn).node_type != VIRTUAL {
            break;
        }
        if !only_unset || zz.nd(vn).clust.is_none() {
            zz.nd_mut(vn).clust = clust;
        }
        e = zz.nd(vn).out.get(&zz.edge_lists, 0);
    }
}

/// `mark_lowclusters`: marks every node, and the virtual nodes of every edge, with its lowest cluster.
pub(crate) fn mark_lowclusters(zz: &mut Globals, root: GraphId) {
    // First, zap any previous cluster labelings.
    let mut n = agfstnode(zz, root);
    while let Some(nn) = n {
        zz.nd_mut(nn).clust = None;
        let mut orig = agfstout(zz, root, nn);
        while let Some(o) = orig {
            mark_chain(zz, o, None, false);
            orig = agnxtout(zz, root, o);
        }
        n = agnxtnode(zz, root, nn);
    }

    mark_lowcluster_basic(zz, root);
}

/// `mark_lowcluster_basic`.
fn mark_lowcluster_basic(zz: &mut Globals, g: GraphId) {
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        mark_lowcluster_basic(zz, clust);
    }
    // See what belongs to this graph that wasn't already marked.
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        if zz.nd(nn).clust.is_none() {
            zz.nd_mut(nn).clust = Some(g);
        }
        let mut orig = agfstout(zz, g, nn);
        while let Some(o) = orig {
            mark_chain(zz, o, Some(g), true);
            orig = agnxtout(zz, g, o);
        }
        n = agnxtnode(zz, g, nn);
    }
}

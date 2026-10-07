//! `dotgen/rank.c`: rank assignment. Clusters are ranked recursively and collapsed into their leaders, the
//! collapsed graph is made acyclic and ranked by network simplex, then the cluster nodes are expanded again.

use std::cmp::{max, min};

use crate::cgraph::aghead;
use crate::cgraph::attr::agget;
use crate::cgraph::edge::{agfstout, agnxtout, agsubedge};
use crate::cgraph::id::agnameof;
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::obj::agcontains;
use crate::cgraph::subg::{agfstsubg, agnxtsubg};
use crate::common::input::do_graph_label;
use crate::common::ns::rank;
use crate::common::utils::{UF_find, UF_singleton, UF_union, agget_text, maptoken};
use crate::core::Globals;
use crate::core::consts::{
    CLUSTER, EDGE_LABEL, LEAFSET, LOCAL, MAXRANK, MAXSHORT, MINRANK, NORMAL, SAMERANK, SINKRANK,
    SOURCERANK,
};
use crate::core::ids::{GraphId, NodeId};
use crate::dotgen::acyclic::acyclic_;

use crate::dotgen::class1::class1_;
use crate::dotgen::decomp::decompose;
use crate::dotgen::dotinit::dot_root;
use crate::h::{elist, node_list, point};

/// `renewlist`: empties an edge list.
fn renewlist(zz: &mut Globals, L: &mut elist) {
    let list = L.list.expect("edge list");
    for i in (0..=L.size).rev() {
        zz.edge_lists.set(list, i, None);
    }
    L.size = 0;
}

/// The `c`-th component of `g`.
fn component(zz: &Globals, g: GraphId, c: i32) -> Option<NodeId> {
    zz.node_lists
        .get(zz.gd(g).comp.list.expect("components"), c)
}

/// The `c`-th cluster of `g`, counting from 1.
pub(crate) fn cluster(zz: &Globals, g: GraphId, c: i32) -> GraphId {
    zz.graph_lists
        .get(zz.gd(g).clust.expect("clusters"), c)
        .expect("cluster")
}

/// `cleanup1`: empties the fast graph and drops the virtual edges of the real ones.
fn cleanup1(zz: &mut Globals, g: GraphId) {
    for c in 0..zz.gd(g).comp.size {
        let first = component(zz, g, c);
        zz.gd_mut(g).nlist = first;
        for n in node_list(zz, first) {
            let mut l = zz.nd(n).in_;
            renewlist(zz, &mut l);
            zz.nd_mut(n).in_ = l;
            let mut l = zz.nd(n).out;
            renewlist(zz, &mut l);
            zz.nd_mut(n).out = l;
            zz.nd_mut(n).mark = 0;
        }
    }
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        let mut e = agfstout(zz, g, nn);
        while let Some(ee) = e {
            let f = zz.ed(ee).to_virt;
            // Null out any other references to f, so that it is not handled twice: parallel multiedges share a
            // virtual edge.
            if let Some(f) = f
                && zz.ed(f).to_orig == Some(ee)
            {
                let mut n1 = agfstnode(zz, g);
                while let Some(nn1) = n1 {
                    let mut e1 = agfstout(zz, g, nn1);
                    while let Some(ee1) = e1 {
                        if ee != ee1 && zz.ed(ee1).to_virt == Some(f) {
                            zz.ed_mut(ee1).to_virt = None;
                        }
                        e1 = agnxtout(zz, g, ee1);
                    }
                    n1 = agnxtnode(zz, g, nn1);
                }
            }
            zz.ed_mut(ee).to_virt = None;
            e = agnxtout(zz, g, ee);
        }
        n = agnxtnode(zz, g, nn);
    }
    zz.gd_mut(g).comp.list = None;
    zz.gd_mut(g).comp.size = 0;
}

/// `edgelabel_ranks`: with edge labels, every edge gets twice its length, to leave a rank for the label's
/// virtual node, and the rank separation is halved to compensate.
fn edgelabel_ranks(zz: &mut Globals, g: GraphId) {
    if (zz.gd(g).has_labels & EDGE_LABEL) != 0 {
        let mut n = agfstnode(zz, g);
        while let Some(nn) = n {
            let mut e = agfstout(zz, g, nn);
            while let Some(ee) = e {
                zz.ed_mut(ee).minlen = zz.ed(ee).minlen.wrapping_mul(2);
                e = agnxtout(zz, g, ee);
            }
            n = agnxtnode(zz, g, nn);
        }
        zz.gd_mut(g).ranksep = (zz.gd(g).ranksep + 1) / 2;
    }
}

/// `collapse_rankset`: PlantUML makes no `rank=same/min/max` subgraphs.
fn collapse_rankset(_kind: i32) {
    unimplemented!("collapse_rankset")
}

/// `rank_set_class`: what kind of set a subgraph is: a cluster or a `rank=` set.
fn rank_set_class(zz: &mut Globals, g: GraphId) -> i32 {
    if is_cluster(zz, g) {
        return CLUSTER;
    }
    maptoken(
        agget_text(zz, g, "rank").as_deref(),
        &["same", "min", "source", "max", "sink"],
        &[SAMERANK, MINRANK, SOURCERANK, MAXRANK, SINKRANK, 0],
    )
}

/// `make_new_cluster`: adds `subg` to `g`'s clusters and returns its number.
fn make_new_cluster(zz: &mut Globals, g: GraphId, subg: GraphId) -> i32 {
    zz.gd_mut(g).n_cluster += 1;
    let cno = zz.gd(g).n_cluster;
    let clust = zz.graph_lists.REALLOC(cno + 1, zz.gd(g).clust);
    zz.gd_mut(g).clust = Some(clust);
    zz.graph_lists.set(clust, cno, Some(subg));
    do_graph_label(zz, subg);
    cno
}

/// `node_induce`: completes cluster `g` with the edges of the root between its nodes. A node may only be in one
/// cluster at a level; Smetana cannot delete it from the others.
fn node_induce(zz: &mut Globals, par: GraphId, g: GraphId) {
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        n = agnxtnode(zz, g, nn);
        if zz.nd(nn).ranktype != 0 {
            unimplemented!("agdelete: a node in two clusters");
        }
        for i in 1..zz.gd(par).n_cluster {
            let clust = cluster(zz, par, i);
            if agcontains(zz, clust, nn) {
                unimplemented!("agdelete: a node in two clusters");
            }
        }
        zz.nd_mut(nn).clust = None;
    }

    let root = dot_root(zz, g);
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        let mut e = agfstout(zz, root, nn);
        while let Some(ee) = e {
            let head = aghead(zz, ee);
            if agcontains(zz, g, head) {
                agsubedge(zz, g, ee, true);
            }
            e = agnxtout(zz, root, ee);
        }
        n = agnxtnode(zz, g, nn);
    }
}

/// `cluster_leader`: picks a node on the cluster's first rank to stand for the collapsed cluster.
fn cluster_leader(zz: &mut Globals, clust: GraphId) {
    let mut leader = None;
    for n in node_list(zz, zz.gd(clust).nlist) {
        if zz.nd(n).rank == 0 && zz.nd(n).node_type == NORMAL {
            leader = Some(n);
        }
    }
    let leader = leader.expect("cluster leader");
    zz.gd_mut(clust).leader = Some(leader);

    let mut n = agfstnode(zz, clust);
    while let Some(nn) = n {
        UF_union(zz, nn, leader);
        zz.nd_mut(nn).ranktype = CLUSTER;
        n = agnxtnode(zz, clust, nn);
    }
}

/// `collapse_cluster`: ranks a cluster locally and collapses it into its leader; `class1` turns the edges into
/// and out of it into slack nodes.
fn collapse_cluster(zz: &mut Globals, g: GraphId, subg: GraphId) {
    if zz.gd(subg).parent.is_some() {
        return;
    }
    zz.gd_mut(subg).parent = Some(g);
    node_induce(zz, g, subg);
    if agfstnode(zz, subg).is_none() {
        return;
    }
    make_new_cluster(zz, g, subg);
    if zz.CL_type != LOCAL {
        unimplemented!("dot_scan_ranks");
    }
    dot1_rank(zz, subg);
    cluster_leader(zz, subg);
}

/// `collapse_sets`: collapses the clusters and rank sets below `g`.
fn collapse_sets(zz: &mut Globals, rg: GraphId, g: GraphId) {
    let mut subg = agfstsubg(zz, g);
    while let Some(s) = subg {
        let c = rank_set_class(zz, s);
        if c == 0 {
            collapse_sets(zz, rg, s);
        } else if c == CLUSTER && zz.CL_type == LOCAL {
            collapse_cluster(zz, rg, s);
        } else {
            collapse_rankset(c);
        }
        subg = agnxtsubg(zz, s);
    }
}

/// `set_minmax`: moves a cluster's (and its sub-clusters') rank range to its leader's rank.
fn set_minmax(zz: &mut Globals, g: GraphId) {
    let leader_rank = zz.nd(zz.gd(g).leader.expect("leader")).rank;
    zz.gd_mut(g).minrank += leader_rank;
    zz.gd_mut(g).maxrank += leader_rank;
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        set_minmax(zz, clust);
    }
}

/// `minmax_edges`: with min/max rank sets, reverses edges against them. PlantUML makes none.
fn minmax_edges(zz: &Globals, g: GraphId) -> point {
    if zz.gd(g).maxset.is_none() && zz.gd(g).minset.is_none() {
        return point::default();
    }
    unimplemented!("minmax_edges")
}

/// `minmax_edges2`: with min/max rank sets, ties the other nodes to them.
fn minmax_edges2(zz: &Globals, g: GraphId, _slen: point) -> bool {
    if zz.gd(g).maxset.is_some() || zz.gd(g).minset.is_some() {
        unimplemented!("minmax_edges2");
    }
    false
}

/// `rank1`: network simplex on each component, balancing ranks only without clusters.
fn rank1(zz: &mut Globals, g: GraphId) {
    let maxiter = i32::MAX;
    if agget(zz, g, "nslimit1").is_some() {
        unimplemented!("nslimit1");
    }
    for c in 0..zz.gd(g).comp.size {
        zz.gd_mut(g).nlist = component(zz, g, c);
        let balance = i32::from(zz.gd(g).n_cluster == 0);
        rank(zz, g, balance, maxiter);
    }
}

/// `expand_ranksets`: gives the nodes of collapsed sets and clusters their ranks (a cluster node's rank is its
/// offset from the leader) and sets the rank ranges.
fn expand_ranksets(zz: &mut Globals, g: GraphId) {
    let Some(first) = agfstnode(zz, g) else {
        zz.gd_mut(g).maxrank = 0;
        zz.gd_mut(g).minrank = 0;
        return;
    };
    zz.gd_mut(g).minrank = MAXSHORT;
    zz.gd_mut(g).maxrank = -1;
    let mut n = Some(first);
    while let Some(nn) = n {
        let leader = UF_find(zz, nn);
        // ND_rank(n) is 0 for a node outside clusters, and the offset from the leader inside one.
        if leader != nn {
            zz.nd_mut(nn).rank += zz.nd(leader).rank;
        }
        let r = zz.nd(nn).rank;
        zz.gd_mut(g).maxrank = max(zz.gd(g).maxrank, r);
        zz.gd_mut(g).minrank = min(zz.gd(g).minrank, r);

        let ranktype = zz.nd(nn).ranktype;
        if ranktype != 0 && ranktype != LEAFSET {
            UF_singleton(zz, nn);
        }
        n = agnxtnode(zz, g, nn);
    }
    if g == dot_root(zz, g) {
        if zz.CL_type != LOCAL {
            unimplemented!("find_clusters");
        }
        for c in 1..=zz.gd(g).n_cluster {
            let clust = cluster(zz, g, c);
            set_minmax(zz, clust);
        }
    }
}

/// `dot1_rank`: ranks `g`.
fn dot1_rank(zz: &mut Globals, g: GraphId) {
    edgelabel_ranks(zz, g);
    collapse_sets(zz, g, g);
    class1_(zz, g);
    let p = minmax_edges(zz, g);
    decompose(zz, g, 0);
    acyclic_(zz, g);
    if minmax_edges2(zz, g, p) {
        decompose(zz, g, 0);
    }
    rank1(zz, g);
    expand_ranksets(zz, g);
    cleanup1(zz, g);
}

/// `dot_rank`: assigns `ND_rank` to every node and the rank ranges of the graph and its clusters.
pub fn dot_rank(zz: &mut Globals, g: GraphId) {
    if agget(zz, g, "newrank").is_some() {
        unimplemented!("newrank");
    }
    dot1_rank(zz, g);
}

/// `is_cluster`: whether a subgraph's name starts with "cluster".
pub fn is_cluster(zz: &Globals, g: GraphId) -> bool {
    agnameof(zz, g).is_some_and(|name| name.starts_with("cluster"))
}

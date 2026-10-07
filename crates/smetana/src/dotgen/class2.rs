//! `dotgen/class2.c`: builds the fast graph that mincross orders, classifying the edges with their final ranks:
//! long edges become chains of virtual nodes (one carrying the label), flat edges stay, multi-edges merge, and
//! clusters are represented by their skeletons.

use crate::cgraph::edge::{agfindedge, agfstout, agnxtout};
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::obj::agroot;
use crate::cgraph::{aghead, agtail};
use crate::common::utils::UF_find;
use crate::core::Globals;
use crate::core::consts::{CLUSTER, CLUSTER_EDGE, IGNORED};
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::dotgen::cluster::{build_skeleton, mark_clusters};
use crate::dotgen::dotinit::dot_root;
use crate::dotgen::fastgr::{
    fast_node, find_fast_edge, flat_edge, merge_oneway, other_edge, virtual_edge, virtual_node,
};
use crate::dotgen::mincross::{rankleader, virtual_weight};
use crate::dotgen::position::ports_eq;
use crate::dotgen::rank::cluster;

/// `label_vnode`: the virtual node of a chain that carries the edge's label.
fn label_vnode(zz: &mut Globals, g: GraphId, orig: EdgeId) -> NodeId {
    let label = zz.ed(orig).label.expect("edge label");
    let dimen = zz.textlabels[label].dimen;
    let v = virtual_node(zz, g);
    zz.nd_mut(v).label = Some(label);
    let root = agroot(zz, v);
    zz.nd_mut(v).lw = f64::from(zz.gd(root).nodesep);
    if !zz.ed(orig).label_ontop {
        let root = agroot(zz, g);
        if zz.gd(root).GD_flip() {
            zz.nd_mut(v).ht = dimen.x;
            zz.nd_mut(v).rw = dimen.y;
        } else {
            zz.nd_mut(v).ht = dimen.y;
            zz.nd_mut(v).rw = dimen.x;
        }
    }
    v
}

/// `incr_width`: widens a virtual node by `g`'s node separation, for one more edge through it.
fn incr_width(zz: &mut Globals, g: GraphId, v: NodeId) {
    let width = f64::from(zz.gd(g).nodesep / 2);
    zz.nd_mut(v).lw += width;
    zz.nd_mut(v).rw += width;
}

/// `plain_vnode`.
fn plain_vnode(zz: &mut Globals, g: GraphId) -> NodeId {
    let v = virtual_node(zz, g);
    incr_width(zz, g, v);
    v
}

/// `leader_of`: the node standing for `v` at its rank: its set's leader, or its cluster's rank leader.
fn leader_of(zz: &mut Globals, v: NodeId) -> NodeId {
    if zz.nd(v).ranktype == CLUSTER {
        let clust = zz.nd(v).clust.expect("cluster");
        rankleader(zz, clust, zz.nd(v).rank).expect("rank leader")
    } else {
        UF_find(zz, v)
    }
}

/// `make_chain`: a chain of virtual nodes and edges from `from` to `to` standing for `orig`, the label's node
/// in the middle.
fn make_chain(zz: &mut Globals, g: GraphId, from: NodeId, to: NodeId, orig: EdgeId) {
    let mut u = from;
    let label_rank = if zz.ed(orig).label.is_some() {
        i32::midpoint(zz.nd(from).rank, zz.nd(to).rank)
    } else {
        -1
    };
    for r in zz.nd(from).rank + 1..=zz.nd(to).rank {
        let v = if r < zz.nd(to).rank {
            let v = if r == label_rank {
                label_vnode(zz, g, orig)
            } else {
                plain_vnode(zz, g)
            };
            zz.nd_mut(v).rank = r;
            v
        } else {
            to
        };
        let e = virtual_edge(zz, u, v, Some(orig));
        virtual_weight(zz, e);
        u = v;
    }
}

/// `interclrep`: represents an edge between clusters by a chain between their rank leaders.
fn interclrep(zz: &mut Globals, g: GraphId, e: EdgeId) {
    let mut t = leader_of(zz, agtail(zz, e));
    let mut h = leader_of(zz, aghead(zz, e));
    if zz.nd(t).rank > zz.nd(h).rank {
        std::mem::swap(&mut t, &mut h);
    }
    if zz.nd(t).clust != zz.nd(h).clust {
        if let Some(ve) = find_fast_edge(zz, t, h) {
            merge_chain(zz, g, e, ve, true);
            return;
        }
        if zz.nd(t).rank == zz.nd(h).rank {
            return;
        }
        make_chain(zz, g, t, h, e);

        let mut ve = zz.ed(e).to_virt;
        while let Some(v) = ve
            && zz.nd(aghead(zz, v)).rank <= zz.nd(h).rank
        {
            zz.ed_mut(v).edge_type = CLUSTER_EDGE;
            ve = zz.nd(aghead(zz, v)).out.get(&zz.edge_lists, 0);
        }
    }
    // Else ignore intra-cluster edges at this point.
}

/// `is_cluster_edge`.
fn is_cluster_edge(zz: &Globals, e: EdgeId) -> bool {
    zz.nd(agtail(zz, e)).ranktype == CLUSTER || zz.nd(aghead(zz, e)).ranktype == CLUSTER
}

/// `merge_chain`: makes the chain starting with `f` stand for `e` too.
pub(crate) fn merge_chain(zz: &mut Globals, g: GraphId, e: EdgeId, f: EdgeId, flag: bool) {
    let lastrank = zz.nd(agtail(zz, e)).rank.max(zz.nd(aghead(zz, e)).rank);
    zz.ed_mut(e).to_virt = Some(f);
    let mut rep = Some(f);
    while let Some(r) = rep {
        // Inter-cluster multi-edges are not counted now.
        if flag {
            zz.ed_mut(r).count = zz.ed(r).count.wrapping_add(zz.ed(e).count);
        }
        zz.ed_mut(r).xpenalty = zz.ed(r).xpenalty.wrapping_add(zz.ed(e).xpenalty);
        zz.ed_mut(r).weight = zz.ed(r).weight.wrapping_add(zz.ed(e).weight);
        let head = aghead(zz, r);
        if zz.nd(head).rank == lastrank {
            break;
        }
        incr_width(zz, g, head);
        rep = zz.nd(head).out.get(&zz.edge_lists, 0);
    }
}

/// `mergeable`: whether `f` duplicates `e`: same ends, label and ports.
pub(crate) fn mergeable(zz: &Globals, e: Option<EdgeId>, f: EdgeId) -> bool {
    e.is_some_and(|e| {
        agtail(zz, e) == agtail(zz, f)
            && aghead(zz, e) == aghead(zz, f)
            && zz.ed(e).label == zz.ed(f).label
            && ports_eq(zz, e, f)
    })
}

/// `class2`.
pub fn class2(zz: &mut Globals, g: GraphId) {
    zz.gd_mut(g).nlist = None;
    zz.gd_mut(g).n_nodes = 0;

    mark_clusters(zz, g);
    for c in 1..=zz.gd(g).n_cluster {
        let clust = cluster(zz, g, c);
        build_skeleton(zz, g, clust);
    }
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        let mut e = agfstout(zz, g, nn);
        while let Some(ee) = e {
            for end in [aghead(zz, ee), agtail(zz, ee)] {
                if zz.nd(end).weight_class <= 2 {
                    zz.nd_mut(end).weight_class += 1;
                }
            }
            e = agnxtout(zz, g, ee);
        }
        n = agnxtnode(zz, g, nn);
    }

    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        if zz.nd(nn).clust.is_none() && nn == UF_find(zz, nn) {
            fast_node(zz, g, nn);
            zz.gd_mut(g).n_nodes += 1;
        }
        let mut prev = None;
        let mut e = agfstout(zz, g, nn);
        while let Some(ee) = e {
            class2_edge(zz, g, ee, &mut prev);
            e = agnxtout(zz, g, ee);
        }
        n = agnxtnode(zz, g, nn);
    }

    // Since decompose() is not called on subgraphs.
    if g != dot_root(zz, g) {
        let list = zz.node_lists.REALLOC(1, zz.gd(g).comp.list);
        zz.gd_mut(g).comp.list = Some(list);
        let nlist = zz.gd(g).nlist;
        zz.node_lists.set(list, 0, nlist);
    }
}

/// The body of `class2`'s edge loop: installs `e` in the fast graph, `prev` being the previous edge of its tail.
fn class2_edge(zz: &mut Globals, g: GraphId, e: EdgeId, prev: &mut Option<EdgeId>) {
    // Already processed.
    if zz.ed(e).to_virt.is_some() {
        *prev = Some(e);
        return;
    }

    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    // Edges involving sub-clusters of g.
    if is_cluster_edge(zz, e) {
        // Following is new cluster multi-edge code.
        if mergeable(zz, *prev, e) {
            let p = prev.expect("previous edge");
            if let Some(pv) = zz.ed(p).to_virt {
                merge_chain(zz, g, e, pv, false);
                other_edge(zz, e);
            } else if zz.nd(tail).rank == zz.nd(head).rank {
                merge_oneway(zz, e, p);
                other_edge(zz, e);
            }
            // Else is an intra-cluster edge.
            return;
        }
        interclrep(zz, g, e);
        *prev = Some(e);
        return;
    }
    // Merge multi-edges.
    if let Some(p) = *prev
        && tail == agtail(zz, p)
        && head == aghead(zz, p)
    {
        if zz.nd(tail).rank == zz.nd(head).rank {
            merge_oneway(zz, e, p);
            other_edge(zz, e);
            return;
        }
        if zz.ed(e).label.is_none() && zz.ed(p).label.is_none() && ports_eq(zz, e, p) {
            if zz.Concentrate {
                zz.ed_mut(e).edge_type = IGNORED;
            } else {
                let pv = zz.ed(p).to_virt.expect("virtual edge");
                merge_chain(zz, g, e, pv, true);
                other_edge(zz, e);
            }
            return;
        }
        // Parallel edges with different labels fall through here.
    }

    // Self edges.
    if tail == head {
        other_edge(zz, e);
        *prev = Some(e);
        return;
    }

    let t = UF_find(zz, tail);
    let h = UF_find(zz, head);

    // Non-leader leaf nodes.
    if tail != t || head != h {
        return;
    }

    // Flat edges.
    if zz.nd(tail).rank == zz.nd(head).rank {
        flat_edge(zz, g, e);
        *prev = Some(e);
        return;
    }

    // Forward edges.
    if zz.nd(head).rank > zz.nd(tail).rank {
        make_chain(zz, g, tail, head, e);
        *prev = Some(e);
        return;
    }

    // Backward edges; avoid opp == e in undirected graphs.
    if let Some(opp) = agfindedge(zz, g, head, tail)
        && aghead(zz, opp) != head
    {
        // Shadows a forward edge.
        if zz.ed(opp).to_virt.is_none() {
            let (otail, ohead) = (agtail(zz, opp), aghead(zz, opp));
            make_chain(zz, g, otail, ohead, opp);
        }
        if zz.ed(e).label.is_none() && zz.ed(opp).label.is_none() && ports_eq(zz, e, opp) {
            if zz.Concentrate {
                zz.ed_mut(e).edge_type = IGNORED;
                zz.ed_mut(opp).conc_opp_flag = true;
            } else {
                other_edge(zz, e);
                let ov = zz.ed(opp).to_virt.expect("virtual edge");
                merge_chain(zz, g, e, ov, true);
            }
            return;
        }
    }
    make_chain(zz, g, head, tail, e);
    *prev = Some(e);
}

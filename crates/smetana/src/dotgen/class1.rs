//! `dotgen/class1.c`: builds the fast graph for ranking from the real edges, merging parallel edges and routing
//! edges into and out of collapsed clusters through slack nodes.

use crate::cgraph::edge::{agfstout, agnxtout};
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::{aghead, agtail};
use crate::common::utils::{UF_find, agxget_text, mapbool};
use crate::core::Globals;
use crate::core::consts::SLACKNODE;
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::dotgen::cluster::mark_clusters;
use crate::dotgen::fastgr::{find_fast_edge, merge_oneway, virtual_edge, virtual_node};
use crate::dotgen::position::make_aux_edge;

/// `nonconstraint_edge`: whether an edge has `constraint=false`.
pub fn nonconstraint_edge(zz: &mut Globals, e: EdgeId) -> bool {
    zz.E_constr
        .and_then(|constr| agxget_text(zz, e, constr))
        .is_some_and(|constr| !constr.is_empty() && !mapbool(Some(&constr)))
}

/// `interclust1`: an edge into or out of a cluster becomes a slack node with two edges, to the representatives
/// of its ends, long enough for the ends' ranks inside their clusters.
fn interclust1(zz: &mut Globals, g: GraphId, t: NodeId, h: NodeId, e: EdgeId) {
    let rank_in_cluster = |zz: &Globals, n: NodeId| match zz.nd(n).clust {
        Some(clust) => zz.nd(n).rank - zz.nd(zz.gd(clust).leader.expect("leader")).rank,
        None => 0,
    };
    let t_rank = rank_in_cluster(zz, agtail(zz, e));
    let h_rank = rank_in_cluster(zz, aghead(zz, e));
    let offset = zz.ed(e).minlen + t_rank - h_rank;
    let (t_len, h_len) = if offset > 0 {
        (0, offset)
    } else {
        (-offset, 0)
    };

    let v = virtual_node(zz, g);
    zz.nd_mut(v).node_type = SLACKNODE;
    let t0 = UF_find(zz, t);
    let h0 = UF_find(zz, h);
    let weight = zz.ed(e).weight;
    let rt = make_aux_edge(zz, v, t0, f64::from(t_len), 10 * weight);
    let rh = make_aux_edge(zz, v, h0, f64::from(h_len), weight);
    zz.ed_mut(rt).to_orig = Some(e);
    zz.ed_mut(rh).to_orig = Some(e);
}

/// `class1`.
pub fn class1_(zz: &mut Globals, g: GraphId) {
    mark_clusters(zz, g);
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        let mut e = agfstout(zz, g, nn);
        while let Some(ee) = e {
            e = agnxtout(zz, g, ee);
            // Skip edges already processed, and edges to ignore in this phase.
            if zz.ed(ee).to_virt.is_some() || nonconstraint_edge(zz, ee) {
                continue;
            }
            let t = UF_find(zz, agtail(zz, ee));
            let h = UF_find(zz, aghead(zz, ee));
            // Skip self, flat and intra-cluster edges.
            if t == h {
                continue;
            }
            // Inter-cluster edges need special treatment.
            if zz.nd(t).clust.is_some() || zz.nd(h).clust.is_some() {
                let (tail, head) = (agtail(zz, ee), aghead(zz, ee));
                interclust1(zz, g, tail, head, ee);
                continue;
            }
            if let Some(rep) = find_fast_edge(zz, t, h) {
                merge_oneway(zz, ee, rep);
            } else {
                virtual_edge(zz, t, h, Some(ee));
            }
        }
        n = agnxtnode(zz, g, nn);
    }
}

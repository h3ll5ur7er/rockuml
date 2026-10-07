//! `dotgen/cluster.c`: clusters in the fast graph.

use crate::cgraph::aghead;
use crate::cgraph::edge::{agfstout, agnxtout};
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::common::utils::{UF_setname, UF_singleton};
use crate::core::Globals;
use crate::core::consts::{CLUSTER, NORMAL, VIRTUAL};
use crate::core::ids::GraphId;

/// `mark_clusters`: marks every node of `g` with its top-level cluster under `g` (`ND_clust`), merges it into
/// the cluster's leader, and marks the virtual nodes of the cluster's edges too.
pub fn mark_clusters(zz: &mut Globals, g: GraphId) {
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
        let clust = zz
            .graph_lists
            .get(zz.gd(g).clust.expect("clusters"), c)
            .expect("cluster");
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

            // Mark the virtual nodes of the cluster's edges.
            let mut orig = agfstout(zz, clust, nn);
            while let Some(o) = orig {
                let mut e = zz.ed(o).to_virt;
                while let Some(ee) = e {
                    let vn = aghead(zz, ee);
                    if zz.nd(vn).node_type != VIRTUAL {
                        break;
                    }
                    zz.nd_mut(vn).clust = Some(clust);
                    e = zz.nd(vn).out.get(&zz.edge_lists, 0);
                    // Trouble if concentrators and clusters are mixed.
                }
                orig = agnxtout(zz, clust, o);
            }
            n = nn_next;
        }
    }
}

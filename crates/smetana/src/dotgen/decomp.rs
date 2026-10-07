//! `dotgen/decomp.c`: splits the fast graph into connected components (`GD_comp`), each a node list linked by
//! `ND_next`.

use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::{aghead, agtail};
use crate::common::utils::UF_find;
use crate::core::Globals;
use crate::core::ids::{GraphId, NodeId};

fn G_decomp(zz: &Globals) -> GraphId {
    zz.G_decomp.expect("G_decomp")
}

/// `begin_component`.
fn begin_component(zz: &mut Globals) {
    let g = G_decomp(zz);
    zz.gd_mut(g).nlist = None;
    zz.Last_node_decomp = None;
}

/// `add_to_component`.
fn add_to_component(zz: &mut Globals, n: NodeId) {
    let g = G_decomp(zz);
    zz.gd_mut(g).n_nodes += 1;
    zz.nd_mut(n).mark = i32::from(zz.Cmark);
    if let Some(last) = zz.Last_node_decomp {
        zz.nd_mut(n).prev = Some(last);
        zz.nd_mut(last).next = Some(n);
    } else {
        zz.nd_mut(n).prev = None;
        zz.gd_mut(g).nlist = Some(n);
    }
    zz.Last_node_decomp = Some(n);
    zz.nd_mut(n).next = None;
}

/// `end_component`.
fn end_component(zz: &mut Globals) {
    let g = G_decomp(zz);
    let i = zz.gd(g).comp.size;
    zz.gd_mut(g).comp.size += 1;
    let list = zz
        .node_lists
        .REALLOC(zz.gd(g).comp.size, zz.gd(g).comp.list);
    zz.gd_mut(g).comp.list = Some(list);
    let nlist = zz.gd(g).nlist;
    zz.node_lists.set(list, i, nlist);
}

/// `search_component`: adds the component of `n` (through all four edge lists) depth first.
fn search_component(zz: &mut Globals, n: NodeId) {
    add_to_component(zz, n);
    let info = zz.nd(n);
    let vec = [info.out, info.in_, info.flat_out, info.flat_in];
    for l in vec {
        if l.list.is_none() {
            continue;
        }
        let mut i = 0;
        while let Some(e) = l.get(&zz.edge_lists, i) {
            let mut other = aghead(zz, e);
            if other == n {
                other = agtail(zz, e);
            }
            if zz.nd(other).mark != i32::from(zz.Cmark) && other == UF_find(zz, other) {
                search_component(zz, other);
            }
            i += 1;
        }
    }
}

/// `decompose`: the connected components of `g`'s fast graph. On pass 0 only set representatives
/// (`UF_find`) take part; later passes go through the cluster rank leaders.
pub fn decompose(zz: &mut Globals, g: GraphId, pass: i32) {
    zz.G_decomp = Some(g);
    zz.Cmark = zz.Cmark.wrapping_add(1);
    if zz.Cmark == 0 {
        zz.Cmark = 1;
    }
    zz.gd_mut(g).comp.size = 0;
    zz.gd_mut(g).n_nodes = 0;
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        let mut v = nn;
        if pass > 0
            && let Some(subg) = zz.nd(v).clust
        {
            let leaders = zz.gd(subg).rankleader.expect("rank leaders");
            v = zz
                .node_lists
                .get(leaders, zz.nd(v).rank)
                .expect("rank leader");
        } else if v != UF_find(zz, v) {
            n = agnxtnode(zz, g, nn);
            continue;
        }
        if zz.nd(v).mark != i32::from(zz.Cmark) {
            begin_component(zz);
            search_component(zz, v);
            end_component(zz);
        }
        n = agnxtnode(zz, g, nn);
    }
}

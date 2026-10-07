//! `dotgen/fastgr.c`: dot's "fast graph", the edge lists (`ND_in`, `ND_out`, `ND_flat_*`, `ND_other`) and node
//! list (`GD_nlist`) the phases work on, with virtual nodes and edges.

use crate::cgraph::obj::agroot;
use crate::cgraph::{AGINEDGE, AGNODE, AGOUTEDGE, M_aghead, M_agtail, aghead, agtail};
use crate::core::Globals;
use crate::core::consts::VIRTUAL;
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::dotgen::dotinit::dot_root;
use crate::h::{alloc_elist, elist, elist_append};

/// Which of a node's edge lists to work on.
#[derive(Clone, Copy)]
pub(crate) enum EdgeList {
    In,
    Out,
    FlatIn,
    FlatOut,
    Other,
}

pub(crate) fn edge_list(zz: &mut Globals, n: NodeId, which: EdgeList) -> &mut elist {
    let info = zz.nd_mut(n);
    match which {
        EdgeList::In => &mut info.in_,
        EdgeList::Out => &mut info.out,
        EdgeList::FlatIn => &mut info.flat_in,
        EdgeList::FlatOut => &mut info.flat_out,
        EdgeList::Other => &mut info.other,
    }
}

/// `elist_append(e, ND_<which>(n))`.
pub(crate) fn append(zz: &mut Globals, e: EdgeId, n: NodeId, which: EdgeList) {
    let mut l = *edge_list(zz, n, which);
    elist_append(&mut zz.edge_lists, e, &mut l);
    *edge_list(zz, n, which) = l;
}

/// `ffe`: the edge from `u` to `v` in one of two lists (`u`'s outgoing, `v`'s incoming), searching the shorter.
fn ffe(zz: &Globals, u: NodeId, uL: elist, v: NodeId, vL: elist) -> Option<EdgeId> {
    if uL.size > 0 && vL.size > 0 {
        let (l, matches): (elist, &dyn Fn(EdgeId) -> bool) = if uL.size < vL.size {
            (uL, &|e| aghead(zz, e) == v)
        } else {
            (vL, &|e| agtail(zz, e) == u)
        };
        let mut i = 0;
        while let Some(e) = l.get(&zz.edge_lists, i) {
            if matches(e) {
                return Some(e);
            }
            i += 1;
        }
    }
    None
}

/// `find_fast_edge`.
pub fn find_fast_edge(zz: &Globals, u: NodeId, v: NodeId) -> Option<EdgeId> {
    ffe(zz, u, zz.nd(u).out, v, zz.nd(v).in_)
}

/// `find_flat_edge`.
pub fn find_flat_edge(zz: &Globals, u: NodeId, v: NodeId) -> Option<EdgeId> {
    ffe(zz, u, zz.nd(u).flat_out, v, zz.nd(v).flat_in)
}

/// `safe_list_append`: appends `e` unless the list has it.
fn safe_list_append(zz: &mut Globals, e: EdgeId, n: NodeId, which: EdgeList) {
    let l = *edge_list(zz, n, which);
    if (0..l.size).any(|i| l.get(&zz.edge_lists, i) == Some(e)) {
        return;
    }
    append(zz, e, n, which);
}

/// `fast_edge`: installs `e` in its tail's out-list and its head's in-list.
pub fn fast_edge(zz: &mut Globals, e: EdgeId) -> EdgeId {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    append(zz, e, tail, EdgeList::Out);
    append(zz, e, head, EdgeList::In);
    e
}

/// `zapinlist`: removes `e` from a list, moving the last edge into its slot. Like Java, it only reads the list
/// while searching it, so an empty list may be unallocated.
pub(crate) fn zapinlist(zz: &mut Globals, n: NodeId, which: EdgeList, e: EdgeId) {
    let l = *edge_list(zz, n, which);
    if let Some(i) = (0..l.size).find(|&i| l.get(&zz.edge_lists, i) == Some(e)) {
        let list = l.list.expect("edge list");
        let size = l.size - 1;
        edge_list(zz, n, which).size = size;
        let last = zz.edge_lists.get(list, size);
        zz.edge_lists.set(list, i, last);
        zz.edge_lists.set(list, size, None);
    }
}

/// `delete_fast_edge`.
pub fn delete_fast_edge(zz: &mut Globals, e: EdgeId) {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    zapinlist(zz, tail, EdgeList::Out, e);
    zapinlist(zz, head, EdgeList::In, e);
}

/// `other_edge`.
pub fn other_edge(zz: &mut Globals, e: EdgeId) {
    let tail = agtail(zz, e);
    append(zz, e, tail, EdgeList::Other);
}

/// `safe_other_edge`.
pub fn safe_other_edge(zz: &mut Globals, e: EdgeId) {
    let tail = agtail(zz, e);
    safe_list_append(zz, e, tail, EdgeList::Other);
}

/// A new edge pair with typed halves and no record data, as `new ST_Agedgepair_s()` plus `AGTYPE` on both
/// halves. Returns its out-half.
pub(crate) fn new_edge_pair(zz: &mut Globals) -> EdgeId {
    let e = zz.new_agedgepair();
    zz.tag_mut(e).objtype = AGOUTEDGE;
    let opp = e.flip();
    zz.tag_mut(opp).objtype = AGINEDGE;
    e
}

/// `new_virtual_edge`: a virtual edge from `u` to `v`, standing for `orig` if given.
pub fn new_virtual_edge(zz: &mut Globals, u: NodeId, v: NodeId, orig: Option<EdgeId>) -> EdgeId {
    let e = new_edge_pair(zz);
    M_agtail(zz, e, u);
    M_aghead(zz, e, v);
    zz.ed_mut(e).edge_type = VIRTUAL;

    if let Some(orig) = orig {
        let seq = zz.tag(orig).seq;
        zz.tag_mut(e).seq = seq;
        zz.tag_mut(e.flip()).seq = seq;
        let o = *zz.ed(orig);
        let info = zz.ed_mut(e);
        info.count = o.count;
        info.xpenalty = o.xpenalty;
        info.weight = o.weight;
        info.minlen = o.minlen;
        let (otail, ohead) = (agtail(zz, orig), aghead(zz, orig));
        if u == otail {
            zz.ed_mut(e).tail_port = o.tail_port;
        } else if u == ohead {
            zz.ed_mut(e).tail_port = o.head_port;
        }
        if v == ohead {
            zz.ed_mut(e).head_port = o.head_port;
        } else if v == otail {
            zz.ed_mut(e).head_port = o.tail_port;
        }
        if zz.ed(orig).to_virt.is_none() {
            zz.ed_mut(orig).to_virt = Some(e);
        }
        zz.ed_mut(e).to_orig = Some(orig);
    } else {
        let info = zz.ed_mut(e);
        info.minlen = 1;
        info.count = 1;
        info.xpenalty = 1;
        info.weight = 1;
    }
    e
}

/// `virtual_edge`: a new virtual edge, installed in the fast graph.
pub fn virtual_edge(zz: &mut Globals, u: NodeId, v: NodeId, orig: Option<EdgeId>) -> EdgeId {
    let e = new_virtual_edge(zz, u, v, orig);
    fast_edge(zz, e)
}

/// `fast_node`: prepends `n` to `g`'s node list.
pub fn fast_node(zz: &mut Globals, g: GraphId, n: NodeId) {
    let next = zz.gd(g).nlist;
    zz.nd_mut(n).next = next;
    if let Some(next) = next {
        zz.nd_mut(next).prev = Some(n);
    }
    zz.gd_mut(g).nlist = Some(n);
    zz.nd_mut(n).prev = None;
}

/// `delete_fast_node`.
pub fn delete_fast_node(zz: &mut Globals, g: GraphId, n: NodeId) {
    let (prev, next) = (zz.nd(n).prev, zz.nd(n).next);
    if let Some(next) = next {
        zz.nd_mut(next).prev = prev;
    }
    match prev {
        Some(prev) => zz.nd_mut(prev).next = next,
        None => zz.gd_mut(g).nlist = next,
    }
}

/// `virtual_node`: a new virtual node of `g`, in its fast graph.
pub fn virtual_node(zz: &mut Globals, g: GraphId) -> NodeId {
    let root = agroot(zz, g);
    let n = zz.new_agnode(root);
    zz.tag_mut(n).objtype = AGNODE;
    let mut in_ = elist::default();
    let mut out = elist::default();
    alloc_elist(&mut zz.edge_lists, 4, &mut in_);
    alloc_elist(&mut zz.edge_lists, 4, &mut out);
    let info = zz.nd_mut(n);
    info.node_type = VIRTUAL;
    info.rw = 1.0;
    info.lw = 1.0;
    info.ht = 1.0;
    info.UF_size = 1;
    info.in_ = in_;
    info.out = out;
    fast_node(zz, g, n);
    zz.gd_mut(g).n_nodes += 1;
    n
}

/// `flat_edge`: installs a flat (same rank) edge.
pub fn flat_edge(zz: &mut Globals, g: GraphId, e: EdgeId) {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    append(zz, e, tail, EdgeList::FlatOut);
    append(zz, e, head, EdgeList::FlatIn);
    zz.gd_mut(g).has_flat_edges = 1;
    let root = dot_root(zz, g);
    zz.gd_mut(root).has_flat_edges = 1;
}

/// `delete_flat_edge`.
pub fn delete_flat_edge(zz: &mut Globals, e: EdgeId) {
    if let Some(orig) = zz.ed(e).to_orig
        && zz.ed(orig).to_virt == Some(e)
    {
        zz.ed_mut(orig).to_virt = None;
    }
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    zapinlist(zz, tail, EdgeList::FlatOut, e);
    zapinlist(zz, head, EdgeList::FlatIn, e);
}

/// `basic_merge`: adds `e`'s count, penalty and weight to `rep` and the virtual edges after it.
fn basic_merge(zz: &mut Globals, e: EdgeId, rep: EdgeId) {
    if zz.ed(rep).minlen < zz.ed(e).minlen {
        zz.ed_mut(rep).minlen = zz.ed(e).minlen;
    }
    let mut rep = Some(rep);
    while let Some(r) = rep {
        zz.ed_mut(r).count = zz.ed(r).count.wrapping_add(zz.ed(e).count);
        zz.ed_mut(r).xpenalty = zz.ed(r).xpenalty.wrapping_add(zz.ed(e).xpenalty);
        zz.ed_mut(r).weight = zz.ed(r).weight.wrapping_add(zz.ed(e).weight);
        rep = zz.ed(r).to_virt;
    }
}

/// `merge_oneway`: makes `rep` stand for `e` too.
pub fn merge_oneway(zz: &mut Globals, e: EdgeId, rep: EdgeId) {
    if Some(rep) == zz.ed(e).to_virt {
        unimplemented!("merge_oneway glitch");
    }
    zz.ed_mut(e).to_virt = Some(rep);
    basic_merge(zz, e, rep);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cgraph::graph::agopen;
    use crate::h::cgraph::Agdirected;

    #[test]
    fn zapinlist_leaves_an_unallocated_empty_list_alone() {
        let mut zz = Globals::open();
        let g = agopen(&mut zz, Some("g"), Agdirected);
        let n = zz.new_agnode(g);
        let e = zz.new_agedgepair();
        assert_eq!(zz.nd(n).out.list, None);
        zapinlist(&mut zz, n, EdgeList::Out, e);
        assert_eq!(zz.nd(n).out, elist::default());
    }

    #[test]
    fn zapinlist_moves_the_last_edge_into_the_gap() {
        let mut zz = Globals::open();
        let g = agopen(&mut zz, Some("g"), Agdirected);
        let n = zz.new_agnode(g);
        let edges = [
            zz.new_agedgepair(),
            zz.new_agedgepair(),
            zz.new_agedgepair(),
        ];
        for e in edges {
            append(&mut zz, e, n, EdgeList::Out);
        }
        zapinlist(&mut zz, n, EdgeList::Out, edges[0]);
        assert_eq!(zz.nd(n).out.edges(&zz.edge_lists), [edges[2], edges[1]]);
    }
}

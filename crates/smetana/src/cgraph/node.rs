//! `node.c`: nodes. Each (sub)graph holds its nodes in two dictionaries: by sequence (creation order, for
//! iteration) and by id (for lookup).

use super::AGNODE;
use super::attr::agnodeattr_init;
use super::edge::agsubrep;
use super::graph::{agnextseq, agparent};
use super::id::agmapnametoid;
use super::obj::agroot;
use super::rec::{Rec, agbindrec};
use crate::cdt::DtArg;
use crate::core::Globals;
use crate::core::ids::{GraphId, NodeId};
use crate::h::cgraph::Agsubnode_s;

/// `agfindnode_by_id`.
pub(crate) fn agfindnode_by_id(zz: &mut Globals, g: GraphId, id: i32) -> Option<NodeId> {
    let sn = zz.graphs[g].n_id.dtsearch(DtArg::template(id))?;
    Some(zz.subnodes[sn].node)
}

/// `agfstnode`: the first node of `g`, in creation order.
pub fn agfstnode(zz: &mut Globals, g: GraphId) -> Option<NodeId> {
    let sn = zz.graphs[g].n_seq.dtfirst()?;
    Some(zz.subnodes[sn].node)
}

/// `agnxtnode`: the node of `g` created after `n`.
pub fn agnxtnode(zz: &mut Globals, g: GraphId, n: NodeId) -> Option<NodeId> {
    let sn = agsubrep(zz, g, n)?;
    let seq = zz.nodes[n].tag.seq;
    let next = zz.graphs[g].n_seq.dtnext(sn, seq)?;
    Some(zz.subnodes[next].node)
}

/// `newnode`: allocates a node of `g`'s root graph; it is installed in the graphs separately.
fn newnode(zz: &mut Globals, g: GraphId, id: i32, seq: i32) -> NodeId {
    let root = agroot(zz, g);
    let n = zz.new_agnode(root);
    let tag = &mut zz.nodes[n].tag;
    tag.objtype = AGNODE;
    tag.id = id;
    tag.seq = seq;
    if zz.graphs[root].desc.has_attrs != 0 {
        agbindrec(zz, n, Rec::Attr);
    }
    n
}

/// `installnode`: adds `n` to `g`'s own dictionaries.
fn installnode(zz: &mut Globals, g: GraphId, n: NodeId) {
    let sn = if g == agroot(zz, g) {
        zz.nodes[n].mainsub
    } else {
        zz.subnodes.push(Agsubnode_s {
            node: n,
            in_id: None,
            out_id: None,
            in_seq: None,
            out_seq: None,
        })
    };
    let tag = zz.nodes[n].tag;
    let graph = &mut zz.graphs[g];
    graph.n_id.dtinsert(sn, tag.id);
    graph.n_seq.dtinsert(sn, tag.seq);
}

/// `installnodetoroot`: adds `n` to `g` and all its ancestors.
fn installnodetoroot(zz: &mut Globals, g: GraphId, n: NodeId) {
    installnode(zz, g, n);
    if let Some(par) = agparent(zz, g) {
        installnodetoroot(zz, par, n);
    }
}

/// `initnode`.
fn initnode(zz: &mut Globals, g: GraphId, n: NodeId) {
    let root = agroot(zz, g);
    if zz.graphs[root].desc.has_attrs != 0 {
        agnodeattr_init(zz, g, n);
    }
}

/// `agidnode`: the node of `g` with this id. Smetana cannot create nodes by id.
pub(crate) fn agidnode(zz: &mut Globals, g: GraphId, id: i32) -> Option<NodeId> {
    agfindnode_by_id(zz, g, id)
}

/// `agnode`: the node of `g` called `name`, created when `cflag` is set (in `g` and all its ancestors).
pub fn agnode(zz: &mut Globals, g: GraphId, name: Option<&str>, cflag: bool) -> Option<NodeId> {
    let root = agroot(zz, g);
    // Probe for an existing node.
    if let Some(id) = agmapnametoid(zz, g, name, false) {
        if let Some(n) = agfindnode_by_id(zz, g, id) {
            return Some(n);
        }
        // It might exist globally, but needs to be inserted locally.
        if cflag
            && g != root
            && let Some(n) = agfindnode_by_id(zz, root, id)
        {
            return agsubnode(zz, g, n, true);
        }
    }
    if cflag && let Some(id) = agmapnametoid(zz, g, name, true) {
        let seq = agnextseq(zz, g, AGNODE);
        let n = newnode(zz, g, id, seq);
        installnodetoroot(zz, g, n);
        initnode(zz, g, n);
        return Some(n);
    }
    None
}

/// `agsubnode`: `n0` as a node of `g`, inserted into `g` (and its ancestors) when `cflag` is set.
pub(crate) fn agsubnode(zz: &mut Globals, g: GraphId, n0: NodeId, cflag: bool) -> Option<NodeId> {
    if agroot(zz, g) != zz.nodes[n0].root {
        return None;
    }
    let mut n = agfindnode_by_id(zz, g, zz.nodes[n0].tag.id);
    if n.is_none()
        && cflag
        && let Some(par) = agparent(zz, g)
    {
        n = agsubnode(zz, par, n0, cflag);
        installnode(zz, g, n.expect("a node of the parent graph"));
    }
    n
}

//! `edge.c`: edges. A graph keeps each node's out- and in-edges as trees detached from the graph's `e_seq`
//! (ordered by the other end's sequence, then the edge's) and `e_id` (by the other end's id, then the edge's)
//! dictionaries, and restores a tree into the dictionary for every operation.

use super::attr::agedgeattr_init;
use super::graph::{agisstrict, agisundirected, agnextseq, agparent};
use super::id::agmapnametoid;
use super::node::agsubnode;
use super::obj::agroot;
use super::rec::{Rec, agbindrec};
use super::{AGEDGE, AGINEDGE, AGMKIN, AGMKOUT, AGOPP, AGOUTEDGE, aghead, agtail};
use crate::cdt::{DtArg, DtKey, LinkId};
use crate::core::Globals;
use crate::core::ids::{EdgeId, GraphId, NodeId, SubnodeId};
use crate::h::cgraph::Agtag_s;

/// An edge's key in `e_seq` (`agedgeseqcmpf`): the node at its other end, then its sequence number.
#[derive(Clone, Debug)]
pub(crate) struct EdgeSeqKey {
    node: NodeId,
    node_seq: i32,
    seq: i32,
}

impl DtKey for EdgeSeqKey {
    fn dtcmp(&self, other: &Self) -> i32 {
        let v = if self.node == other.node {
            self.seq.wrapping_sub(other.seq)
        } else {
            self.node_seq.wrapping_sub(other.node_seq)
        };
        v.signum()
    }
}

/// An edge's key in `e_id` (`agedgeidcmpf`): the id of the node at its other end, then its own id unless either
/// side's `objtype` is 0, which makes the edge id a wildcard.
#[derive(Clone, Debug)]
pub(crate) struct EdgeIdKey {
    node_id: i32,
    objtype: i32,
    id: i32,
}

impl DtKey for EdgeIdKey {
    fn dtcmp(&self, other: &Self) -> i32 {
        let mut v = self.node_id.wrapping_sub(other.node_id);
        if v == 0 {
            v = if self.objtype == 0 || other.objtype == 0 {
                0
            } else {
                self.id.wrapping_sub(other.id)
            };
        }
        v.signum()
    }
}

fn seq_key(zz: &Globals, e: EdgeId) -> EdgeSeqKey {
    let half = zz.edge(e);
    let node = half.node.expect("edge without node");
    EdgeSeqKey {
        node,
        node_seq: zz.nodes[node].tag.seq,
        seq: half.tag.seq,
    }
}

fn id_key(zz: &Globals, e: EdgeId) -> EdgeIdKey {
    let half = zz.edge(e);
    let node = half.node.expect("edge without node");
    EdgeIdKey {
        node_id: zz.nodes[node].tag.id,
        objtype: half.tag.objtype,
        id: half.tag.id,
    }
}

/// Which of a subnode's four edge sets.
#[derive(Clone, Copy)]
enum EdgeSet {
    InId,
    OutId,
    InSeq,
    OutSeq,
}

fn set_of(zz: &mut Globals, sn: SubnodeId, set: EdgeSet) -> &mut Option<LinkId> {
    let sn = &mut zz.subnodes[sn];
    match set {
        EdgeSet::InId => &mut sn.in_id,
        EdgeSet::OutId => &mut sn.out_id,
        EdgeSet::InSeq => &mut sn.in_seq,
        EdgeSet::OutSeq => &mut sn.out_seq,
    }
}

/// Restores one of `sn`'s sequence-ordered sets into `g.e_seq`, runs `op` there and detaches it again.
fn with_seq_set<R>(
    zz: &mut Globals,
    g: GraphId,
    sn: SubnodeId,
    set: EdgeSet,
    op: impl FnOnce(&mut crate::cdt::Dt<EdgeId, EdgeSeqKey>) -> R,
) -> R {
    let list = *set_of(zz, sn, set);
    let d = &mut zz.graphs[g].e_seq;
    d.dtrestore(list);
    let rv = op(d);
    let list = d.dtextract();
    *set_of(zz, sn, set) = list;
    rv
}

/// The same for the id-ordered sets in `g.e_id`.
fn with_id_set<R>(
    zz: &mut Globals,
    g: GraphId,
    sn: SubnodeId,
    set: EdgeSet,
    op: impl FnOnce(&mut crate::cdt::Dt<EdgeId, EdgeIdKey>) -> R,
) -> R {
    let list = *set_of(zz, sn, set);
    let d = &mut zz.graphs[g].e_id;
    d.dtrestore(list);
    let rv = op(d);
    let list = d.dtextract();
    *set_of(zz, sn, set) = list;
    rv
}

/// `agfstout`: the first out-edge of `n` in `g`.
pub fn agfstout(zz: &mut Globals, g: GraphId, n: NodeId) -> Option<EdgeId> {
    let sn = agsubrep(zz, g, n)?;
    with_seq_set(zz, g, sn, EdgeSet::OutSeq, crate::cdt::Dt::dtfirst)
}

/// `agnxtout`: the out-edge of `e`'s tail that follows `e`.
pub fn agnxtout(zz: &mut Globals, g: GraphId, e: EdgeId) -> Option<EdgeId> {
    let n = agtail(zz, e);
    let sn = agsubrep(zz, g, n)?;
    let key = seq_key(zz, e);
    with_seq_set(zz, g, sn, EdgeSet::OutSeq, |d| d.dtnext(e, key))
}

/// `agfstin`: the first in-edge of `n` in `g`.
pub fn agfstin(zz: &mut Globals, g: GraphId, n: NodeId) -> Option<EdgeId> {
    let sn = agsubrep(zz, g, n)?;
    with_seq_set(zz, g, sn, EdgeSet::InSeq, crate::cdt::Dt::dtfirst)
}

/// `agnxtin`: the in-edge of `e`'s head that follows `e`.
pub fn agnxtin(zz: &mut Globals, g: GraphId, e: EdgeId) -> Option<EdgeId> {
    let n = aghead(zz, e);
    let sn = agsubrep(zz, g, n)?;
    let key = seq_key(zz, e);
    with_seq_set(zz, g, sn, EdgeSet::InSeq, |d| d.dtnext(e, key))
}

/// `agfstedge`: the first out-edge of `n`, or else its first in-edge.
pub fn agfstedge(zz: &mut Globals, g: GraphId, n: NodeId) -> Option<EdgeId> {
    agfstout(zz, g, n).or_else(|| agfstin(zz, g, n))
}

/// `agnxtedge`: the edge of `n` after `e`: its out-edges, then its in-edges except loops.
pub fn agnxtedge(zz: &mut Globals, g: GraphId, e: EdgeId, n: NodeId) -> Option<EdgeId> {
    let is_node = |zz: &Globals, rv: EdgeId| zz.edge(rv).node == Some(n);
    if zz.edge(e).tag.objtype == AGOUTEDGE {
        let rv = agnxtout(zz, g, e);
        if rv.is_some() {
            return rv;
        }
        let mut rv = agfstin(zz, g, n);
        while let Some(r) = rv
            && is_node(zz, r)
        {
            rv = agnxtin(zz, g, r);
        }
        rv
    } else {
        // Only see each edge once: ignore loops as in-edges.
        let mut rv = agnxtin(zz, g, e);
        while let Some(r) = rv
            && is_node(zz, r)
        {
            rv = agnxtin(zz, g, r);
        }
        rv
    }
}

/// `agfindedge_by_key`: the edge from `t` to `h` in `g` matching `key` (any edge if `key.objtype` is 0).
fn agfindedge_by_key(
    zz: &mut Globals,
    g: GraphId,
    t: NodeId,
    h: NodeId,
    key: Agtag_s,
) -> Option<EdgeId> {
    let template = EdgeIdKey {
        node_id: zz.nodes[t].tag.id,
        objtype: key.objtype,
        id: key.id,
    };
    let sn = agsubrep(zz, g, h)?;
    with_id_set(zz, g, sn, EdgeSet::InId, |d| {
        d.dtsearch(DtArg::template(template))
    })
}

/// `agsubrep`: the subnode of `n` in `g`.
pub(crate) fn agsubrep(zz: &mut Globals, g: GraphId, n: NodeId) -> Option<SubnodeId> {
    if g == zz.nodes[n].root {
        return Some(zz.nodes[n].mainsub);
    }
    let id = zz.nodes[n].tag.id;
    zz.graphs[g].n_id.dtsearch(DtArg::template(id))
}

/// `installedge`: adds `e` to `g` and its ancestors, up to the first that has it.
fn installedge(zz: &mut Globals, g: GraphId, e: EdgeId) {
    let out = AGMKOUT(zz, e);
    let in_ = AGMKIN(zz, e);
    let t = agtail(zz, e);
    let h = aghead(zz, e);
    let tag = zz.edge(e).tag;
    let (out_seq, out_id) = (seq_key(zz, out), id_key(zz, out));
    let (in_seq, in_id) = (seq_key(zz, in_), id_key(zz, in_));
    let mut g = Some(g);
    while let Some(gg) = g {
        if agfindedge_by_key(zz, gg, t, h, tag).is_some() {
            break;
        }
        let sn = agsubrep(zz, gg, t).expect("tail in graph");
        with_seq_set(zz, gg, sn, EdgeSet::OutSeq, |d| {
            d.dtinsert(out, out_seq.clone())
        });
        with_id_set(zz, gg, sn, EdgeSet::OutId, |d| {
            d.dtinsert(out, out_id.clone())
        });
        let sn = agsubrep(zz, gg, h).expect("head in graph");
        with_seq_set(zz, gg, sn, EdgeSet::InSeq, |d| {
            d.dtinsert(in_, in_seq.clone())
        });
        with_id_set(zz, gg, sn, EdgeSet::InId, |d| {
            d.dtinsert(in_, in_id.clone())
        });
        g = agparent(zz, gg);
    }
}

/// `newedge`.
fn newedge(zz: &mut Globals, g: GraphId, t: NodeId, h: NodeId, id: i32) -> EdgeId {
    agsubnode(zz, g, t, true);
    agsubnode(zz, g, h, true);
    let out = zz.new_agedgepair();
    let in_ = out.flip();
    let seq = agnextseq(zz, g, AGEDGE);
    *zz.edge_mut(in_) = crate::h::cgraph::Agedge_s {
        tag: Agtag_s {
            objtype: AGINEDGE,
            id,
            seq,
            ..Agtag_s::default()
        },
        node: Some(t),
    };
    *zz.edge_mut(out) = crate::h::cgraph::Agedge_s {
        tag: Agtag_s {
            objtype: AGOUTEDGE,
            id,
            seq,
            ..Agtag_s::default()
        },
        node: Some(h),
    };
    installedge(zz, g, out);
    if zz.graphs[g].desc.has_attrs != 0 {
        agbindrec(zz, out, Rec::Attr);
        agedgeattr_init(zz, g, out);
    }
    out
}

/// `ok_to_make_edge`: strict graphs refuse loops and multi-edges.
fn ok_to_make_edge(zz: &mut Globals, g: GraphId, t: NodeId, h: NodeId) -> bool {
    if agisstrict(zz, g) {
        if zz.graphs[g].desc.no_loop != 0 && t == h {
            return false;
        }
        let wildcard = Agtag_s::default();
        if agfindedge_by_key(zz, g, t, h, wildcard).is_some() {
            return false;
        }
    }
    true
}

/// `agedge`: the edge from `t` to `h` called `name` (or, without a name and without `cflag`, any such edge),
/// created when `cflag` is set.
pub fn agedge(
    zz: &mut Globals,
    g: GraphId,
    t: NodeId,
    h: NodeId,
    name: Option<&str>,
    cflag: bool,
) -> Option<EdgeId> {
    let have_id = agmapnametoid(zz, g, name, false);
    if have_id.is_some() || (name.is_none() && (!cflag || agisstrict(zz, g))) {
        // Probe for a pre-existing edge.
        let key = match have_id {
            Some(id) => Agtag_s {
                id,
                objtype: AGEDGE,
                ..Agtag_s::default()
            },
            None => Agtag_s::default(),
        };
        // It might already exist locally.
        let mut e = agfindedge_by_key(zz, g, t, h, key);
        if e.is_none() && agisundirected(zz, g) {
            e = agfindedge_by_key(zz, g, h, t, key);
        }
        if e.is_some() {
            return e;
        }
        if cflag {
            let root = agroot(zz, g);
            let mut e = agfindedge_by_key(zz, root, t, h, key);
            if e.is_none() && agisundirected(zz, g) {
                e = agfindedge_by_key(zz, root, h, t, key);
            }
            if e.is_some() {
                unimplemented!("subedge");
            }
        }
    }
    if cflag
        && ok_to_make_edge(zz, g, t, h)
        && let Some(id) = agmapnametoid(zz, g, name, true)
    {
        return Some(newedge(zz, g, t, h, id));
    }
    None
}

/// `agsubedge`: `e` as an edge of `g`, inserted into `g` (and its ancestors) when `cflag` is set.
pub fn agsubedge(zz: &mut Globals, g: GraphId, e: EdgeId, cflag: bool) -> Option<EdgeId> {
    let t = agsubnode(zz, g, agtail(zz, e), cflag)?;
    let h = agsubnode(zz, g, aghead(zz, e), cflag)?;
    let tag = zz.edge(e).tag;
    let mut rv = agfindedge_by_key(zz, g, t, h, tag);
    if cflag && rv.is_none() {
        installedge(zz, g, e);
        rv = Some(e);
    }
    rv.map(|rv| {
        if zz.edge(rv).tag.objtype == tag.objtype {
            rv
        } else {
            AGOPP(zz, rv)
        }
    })
}

/// `agfindedge` (a macro in Smetana): any edge from `t` to `h` in `g`.
pub fn agfindedge(zz: &mut Globals, g: GraphId, t: NodeId, h: NodeId) -> Option<EdgeId> {
    agedge(zz, g, t, h, None, false)
}

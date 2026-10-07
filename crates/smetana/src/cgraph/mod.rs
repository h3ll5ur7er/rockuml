//! cgraph, Graphviz's graph library, as far as Smetana implements it: graphs, subgraphs, nodes and edges in
//! creation order, string attributes, records and object names.
//!
//! The functions keep cgraph's names and take the layout context first (`agfstnode(zz, g)`). Object arguments are
//! typed ids; functions on "any object" take `impl Into<Agobj>`. Strings are passed as `&str` and interned ones
//! returned as [`StrId`](crate::core::ids::StrId), readable with [`Globals::agstr`].

#![allow(non_snake_case)]

pub mod apply;
pub mod attr;
pub mod edge;
pub mod graph;
pub mod id;
pub mod node;
pub mod obj;
pub mod rec;
pub mod refstr;
pub mod subg;

use crate::core::Globals;
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::h::cgraph::Agtag_s;

pub const AGRAPH: i32 = 0;
pub const AGNODE: i32 = 1;
pub const AGOUTEDGE: i32 = 2;
pub const AGINEDGE: i32 = 3;
pub const AGEDGE: i32 = AGOUTEDGE;

/// `Agobj_t*`: any cgraph object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Agobj {
    Graph(GraphId),
    Node(NodeId),
    Edge(EdgeId),
}

impl From<GraphId> for Agobj {
    fn from(g: GraphId) -> Self {
        Agobj::Graph(g)
    }
}

impl From<NodeId> for Agobj {
    fn from(n: NodeId) -> Self {
        Agobj::Node(n)
    }
}

impl From<EdgeId> for Agobj {
    fn from(e: EdgeId) -> Self {
        Agobj::Edge(e)
    }
}

impl Globals {
    /// `obj->tag`.
    pub fn tag(&self, obj: impl Into<Agobj>) -> &Agtag_s {
        match obj.into() {
            Agobj::Graph(g) => &self.graphs[g].tag,
            Agobj::Node(n) => &self.nodes[n].tag,
            Agobj::Edge(e) => &self.edge(e).tag,
        }
    }

    pub fn tag_mut(&mut self, obj: impl Into<Agobj>) -> &mut Agtag_s {
        match obj.into() {
            Agobj::Graph(g) => &mut self.graphs[g].tag,
            Agobj::Node(n) => &mut self.nodes[n].tag,
            Agobj::Edge(e) => &mut self.edge_mut(e).tag,
        }
    }
}

pub fn AGTYPE(zz: &Globals, obj: impl Into<Agobj>) -> i32 {
    zz.tag(obj).objtype
}

pub fn AGID(zz: &Globals, obj: impl Into<Agobj>) -> i32 {
    zz.tag(obj).id
}

pub fn AGSEQ(zz: &Globals, obj: impl Into<Agobj>) -> i32 {
    zz.tag(obj).seq
}

/// `AGOPP`: the other half of an edge, chosen by the edge's tag like Java (which fails where C would read past
/// the pair).
pub fn AGOPP(zz: &Globals, e: EdgeId) -> EdgeId {
    debug_assert_eq!(
        zz.edge(e).tag.objtype == AGINEDGE,
        e.is_in_half(),
        "edge tag does not match its half"
    );
    e.flip()
}

/// `AGMKOUT`: the out-half of an edge.
pub fn AGMKOUT(zz: &Globals, e: EdgeId) -> EdgeId {
    if zz.edge(e).tag.objtype == AGOUTEDGE {
        e
    } else {
        AGOPP(zz, e)
    }
}

/// `AGMKIN`: the in-half of an edge.
pub fn AGMKIN(zz: &Globals, e: EdgeId) -> EdgeId {
    if zz.edge(e).tag.objtype == AGINEDGE {
        e
    } else {
        AGOPP(zz, e)
    }
}

/// `agtail` / `AGTAIL`.
pub fn agtail(zz: &Globals, e: EdgeId) -> NodeId {
    zz.edge(AGMKIN(zz, e)).node.expect("edge without tail")
}

/// `aghead` / `AGHEAD`.
pub fn aghead(zz: &Globals, e: EdgeId) -> NodeId {
    zz.edge(AGMKOUT(zz, e)).node.expect("edge without head")
}

/// `agopp`.
pub fn agopp(zz: &Globals, e: EdgeId) -> EdgeId {
    AGOPP(zz, e)
}

/// `M_agtail(e, v)`: sets the tail.
pub fn M_agtail(zz: &mut Globals, e: EdgeId, v: NodeId) {
    let half = AGMKIN(zz, e);
    zz.edge_mut(half).node = Some(v);
}

/// `M_aghead(e, v)`: sets the head.
pub fn M_aghead(zz: &mut Globals, e: EdgeId, v: NodeId) {
    let half = AGMKOUT(zz, e);
    zz.edge_mut(half).node = Some(v);
}

/// `MAKEFWDEDGE(new, old)`: turns `new`, the out-half of a scratch pair ([`Globals::new_agedgepair`]), into a
/// virtual copy of `old` pointing the other way. The record is copied whole, so labels and splines stay shared.
pub fn MAKEFWDEDGE(zz: &mut Globals, new: EdgeId, old: EdgeId) {
    *zz.ed_mut(new) = *zz.ed(old);
    *zz.edge_mut(new) = *zz.edge(old);
    let (head, tail) = (aghead(zz, old), agtail(zz, old));
    M_agtail(zz, new, head);
    M_aghead(zz, new, tail);
    let (tail_port, head_port) = (zz.ed(old).tail_port, zz.ed(old).head_port);
    let info = zz.ed_mut(new);
    info.tail_port = head_port;
    info.head_port = tail_port;
    info.edge_type = crate::core::consts::VIRTUAL;
    info.to_orig = Some(old);
}

#[cfg(test)]
mod tests {
    use super::attr::agsafeset;
    use super::edge::agedge;
    use super::graph::agopen;
    use super::node::agnode;
    use super::{MAKEFWDEDGE, aghead, agtail};
    use crate::core::Globals;
    use crate::core::consts::VIRTUAL;
    use crate::h::cgraph::Agdirected;
    use crate::h::{pointf, textlabel_t};

    #[test]
    fn makefwdedge_reverses_a_copy_and_shares_its_label() {
        let mut zz = Globals::open();
        let g = agopen(&mut zz, Some("g"), Agdirected);
        let a = agnode(&mut zz, g, Some("a"), true).unwrap();
        let b = agnode(&mut zz, g, Some("b"), true).unwrap();
        let e = agedge(&mut zz, g, a, b, None, true).unwrap();
        agsafeset(&mut zz, e, "minlen", "2", "");
        let label = zz.textlabels.push(textlabel_t::default());
        zz.ed_mut(e).label = Some(label);
        zz.ed_mut(e).minlen = 2;
        zz.ed_mut(e).tail_port.p = pointf { x: 1.0, y: 2.0 };

        let fwd = zz.new_agedgepair();
        MAKEFWDEDGE(&mut zz, fwd, e);
        assert_eq!((agtail(&zz, fwd), aghead(&zz, fwd)), (b, a));
        assert_eq!((agtail(&zz, e), aghead(&zz, e)), (a, b));
        assert_eq!(zz.ed(fwd).head_port.p, pointf { x: 1.0, y: 2.0 });
        assert_eq!(zz.ed(fwd).minlen, 2);
        assert_eq!(zz.ed(fwd).edge_type, VIRTUAL);
        assert_eq!(zz.ed(fwd).to_orig, Some(e));
        assert_eq!(zz.tag(fwd).seq, zz.tag(e).seq);
        let shared = zz.ed(fwd).label.unwrap();
        zz.textlabels[shared].pos = pointf { x: 3.0, y: 4.0 };
        assert_eq!(zz.textlabels[label].pos, pointf { x: 3.0, y: 4.0 });
    }
}

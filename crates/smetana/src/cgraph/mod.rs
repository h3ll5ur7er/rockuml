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

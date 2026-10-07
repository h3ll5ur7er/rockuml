//! cgraph's objects (`ST_Agraph_s`, `ST_Agnode_s`, `ST_Agedge_s`...).
//!
//! Every object starts with its tag (`Agobj_s.tag`). Java hangs records off `Agobj_s.data` and looks them up by
//! name; here the records cgraph and dot bind (`_AG_strdata`, `_AG_datadict`, `Agraphinfo_t`, `Agnodeinfo_t`,
//! `Agedgeinfo_t`) are fields, and `recs` remembers which have been bound, because binding interns the record's
//! name like Java does.

use crate::cdt::{Dt, JStr, LinkId};
use crate::cgraph::edge::{EdgeIdKey, EdgeSeqKey};
use crate::core::ids::{ClosId, DictId, EdgeId, GraphId, NodeId, StrId, SubnodeId};
use crate::h::{Agedgeinfo_t, Agnodeinfo_t, Agraphinfo_t};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Agtag_s {
    pub objtype: i32,
    pub mtflock: i32,
    pub attrwf: i32,
    pub seq: i32,
    pub id: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Agdesc_s {
    pub directed: i32,
    pub strict: i32,
    pub no_loop: i32,
    pub maingraph: i32,
    pub flatlock: i32,
    pub no_write: i32,
    pub has_attrs: i32,
    pub has_cmpnd: i32,
}

/// `Agdirected`: the description of the graphs PlantUML lays out.
pub const Agdirected: Agdesc_s = Agdesc_s {
    directed: 1,
    strict: 0,
    no_loop: 0,
    maingraph: 1,
    flatlock: 0,
    no_write: 0,
    has_attrs: 0,
    has_cmpnd: 0,
};

/// `ProtoDesc`: the description of the prototype graph that `agattr(NULL, ...)` creates.
pub const ProtoDesc: Agdesc_s = Agdesc_s {
    directed: 1,
    strict: 0,
    no_loop: 1,
    maingraph: 0,
    flatlock: 1,
    no_write: 1,
    has_attrs: 0,
    has_cmpnd: 0,
};

/// An attribute declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Agsym_s {
    pub name: StrId,
    pub defval: StrId,
    pub id: i32,
    pub kind: i32,
    pub fixed: i32,
    pub print: i32,
}

/// An object's attribute values (record `_AG_strdata`), indexed by `Agsym_s.id`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Agattr_s {
    pub dict: Option<DictId>,
    pub str: Vec<Option<StrId>>,
}

/// A graph's attribute declarations (record `_AG_datadict`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Agdatadict_s {
    pub dict_n: DictId,
    pub dict_e: DictId,
    pub dict_g: DictId,
}

/// An interned string with its reference count. `uid` is the `CString` uid of the interned copy: the id of every
/// object named by this string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct refstr_t {
    pub s: String,
    pub refcnt: i32,
    pub uid: i32,
}

/// What a root graph shares with its subgraphs.
#[derive(Debug)]
pub struct Agclos_s {
    pub(crate) strdict: Dt<StrId, JStr>,
    /// The last sequence number given to a graph, a node and an edge.
    pub seq: [i32; 3],
}

/// A node's membership in a graph, holding the node's edge sets in that graph as detached trees of the graph's
/// `e_seq` and `e_id` dictionaries.
#[derive(Clone, Copy, Debug)]
pub struct Agsubnode_s {
    pub node: NodeId,
    pub(crate) in_id: Option<LinkId>,
    pub(crate) out_id: Option<LinkId>,
    pub(crate) in_seq: Option<LinkId>,
    pub(crate) out_seq: Option<LinkId>,
}

/// Bit flags of the records bound to an object.
pub(crate) const REC_ATTR: u8 = 1;
pub(crate) const REC_DATADICT: u8 = 2;
pub(crate) const REC_INFO: u8 = 4;

#[derive(Clone, Debug)]
pub struct Agnode_s {
    pub tag: Agtag_s,
    pub root: GraphId,
    /// The node's membership in its root graph.
    pub mainsub: SubnodeId,
    pub(crate) recs: u8,
    pub attr: Option<Agattr_s>,
    pub info: Agnodeinfo_t,
}

/// One half of an edge pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Agedge_s {
    pub tag: Agtag_s,
    /// The head for an out-edge, the tail for an in-edge.
    pub node: Option<NodeId>,
}

/// An edge: both halves and the records they share (C's `set_data` keeps the halves' `data` equal).
#[derive(Clone, Debug, Default)]
pub struct Agedgepair_s {
    pub out: Agedge_s,
    pub in_: Agedge_s,
    pub(crate) recs: u8,
    pub attr: Option<Agattr_s>,
    pub info: Agedgeinfo_t,
}

impl Agedgepair_s {
    pub(crate) fn half(&self, e: EdgeId) -> &Agedge_s {
        if e.is_in_half() { &self.in_ } else { &self.out }
    }

    pub(crate) fn half_mut(&mut self, e: EdgeId) -> &mut Agedge_s {
        if e.is_in_half() {
            &mut self.in_
        } else {
            &mut self.out
        }
    }
}

#[derive(Debug)]
pub struct Agraph_s {
    pub tag: Agtag_s,
    pub desc: Agdesc_s,
    /// Nodes by sequence (creation order).
    pub(crate) n_seq: Dt<SubnodeId, i32>,
    /// Nodes by id.
    pub(crate) n_id: Dt<SubnodeId, i32>,
    /// Workspace for the nodes' edge sets by sequence.
    pub(crate) e_seq: Dt<EdgeId, EdgeSeqKey>,
    /// Workspace for the nodes' edge sets by id.
    pub(crate) e_id: Dt<EdgeId, EdgeIdKey>,
    /// Subgraphs by id.
    pub(crate) g_dict: Dt<GraphId, i32>,
    pub parent: Option<GraphId>,
    pub root: GraphId,
    pub clos: ClosId,
    pub(crate) recs: u8,
    pub attr: Option<Agattr_s>,
    pub datadict: Option<Agdatadict_s>,
    pub info: Agraphinfo_t,
}

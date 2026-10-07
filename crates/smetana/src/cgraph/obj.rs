//! `obj.c`: operations on any object.

use super::Agobj;
use super::edge::agsubedge;
use super::node::agidnode;
use crate::core::Globals;
use crate::core::ids::GraphId;

/// `agroot`: the root graph an object belongs to.
pub(crate) fn agroot(zz: &Globals, obj: impl Into<Agobj>) -> GraphId {
    match obj.into() {
        Agobj::Graph(g) => zz.graphs[g].root,
        Agobj::Node(n) => zz.nodes[n].root,
        Agobj::Edge(e) => zz.nodes[zz.edge(e).node.expect("edge without node")].root,
    }
}

/// `agraphof`: a graph itself, or the root graph of a node or edge.
pub(crate) fn agraphof(zz: &Globals, obj: impl Into<Agobj>) -> GraphId {
    match obj.into() {
        Agobj::Graph(g) => g,
        other => agroot(zz, other),
    }
}

/// `agcontains`: whether `obj` belongs to `g`.
pub fn agcontains(zz: &mut Globals, g: GraphId, obj: impl Into<Agobj>) -> bool {
    let obj = obj.into();
    if agroot(zz, g) != agroot(zz, obj) {
        return false;
    }
    match obj {
        Agobj::Graph(_) | Agobj::Node(_) => {
            let id = zz.tag(obj).id;
            agidnode(zz, g, id).is_some()
        }
        Agobj::Edge(e) => agsubedge(zz, g, e, false).is_some(),
    }
}

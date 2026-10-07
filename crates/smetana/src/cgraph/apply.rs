//! `apply.c`: applying a function to an object in a graph and all its subgraphs. Smetana only applies to graphs,
//! for which the object in each subgraph is the subgraph itself (`subgraph_search`).

use super::subg::{agfstsubg, agnxtsubg};
use crate::core::Globals;
use crate::core::ids::GraphId;

/// The function applied: `fn(g, obj)`, the `arg` being captured.
pub type Agobjfn<'a> = dyn FnMut(&mut Globals, GraphId, GraphId) + 'a;

/// `rec_apply`: `f` on `g`, then on the subgraphs in id order, depth first (or the reverse if not `preorder`).
fn rec_apply(zz: &mut Globals, g: GraphId, obj: GraphId, f: &mut Agobjfn<'_>, preorder: bool) {
    if preorder {
        f(zz, g, obj);
    }
    let mut sub = agfstsubg(zz, g);
    while let Some(s) = sub {
        rec_apply(zz, s, s, f, preorder);
        sub = agnxtsubg(zz, s);
    }
    if !preorder {
        f(zz, g, obj);
    }
}

/// `agapply` for a graph object. Like Java, it starts from `subgraph_search(g, obj)`, which is `g` whatever
/// `obj` is.
pub fn agapply(
    zz: &mut Globals,
    g: GraphId,
    _obj: GraphId,
    f: &mut Agobjfn<'_>,
    preorder: bool,
) -> i32 {
    rec_apply(zz, g, g, f, preorder);
    0
}

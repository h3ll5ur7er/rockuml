//! `apply.c`: applying a function to an object in a graph and all its subgraphs. Smetana only applies to graphs,
//! for which the object in each subgraph is the subgraph itself (`subgraph_search`).

use super::subg::{agfstsubg, agnxtsubg};
use crate::core::Globals;
use crate::core::ids::GraphId;

/// The function applied: `fn(g, obj)` with `obj` being the graph `g` itself, the `arg` being captured.
pub type Agobjfn<'a> = dyn FnMut(&mut Globals, GraphId) + 'a;

/// `rec_apply`: `f` on `g`, then on the subgraphs in id order, depth first (or the reverse if not `preorder`).
fn rec_apply(zz: &mut Globals, g: GraphId, f: &mut Agobjfn<'_>, preorder: bool) {
    if preorder {
        f(zz, g);
    }
    let mut sub = agfstsubg(zz, g);
    while let Some(s) = sub {
        rec_apply(zz, s, f, preorder);
        sub = agnxtsubg(zz, s);
    }
    if !preorder {
        f(zz, g);
    }
}

/// `agapply` for the graph `g` itself. Like Java, it starts from `subgraph_search(g, obj)`, which is `g`
/// whatever `obj` is, so `obj` is left out. The success code C returns is always 0 here, so it is left out too.
pub fn agapply(zz: &mut Globals, g: GraphId, f: &mut Agobjfn<'_>, preorder: bool) {
    rec_apply(zz, g, f, preorder);
}

//! `subg.c`: subgraphs, kept by each parent in id order.

use super::graph::{agopen1, agparent, new_graph};
use super::id::agmapnametoid;
use crate::cdt::DtArg;
use crate::core::Globals;
use crate::core::ids::GraphId;
use crate::h::cgraph::Agdesc_s;

/// `agfindsubg_by_id`: the child of `g` with this id.
fn agfindsubg_by_id(zz: &mut Globals, g: GraphId, id: i32) -> Option<GraphId> {
    zz.graphs[g].g_dict.dtsearch(DtArg::template(id))
}

/// `localsubg`: the child of `g` with this id, created if needed.
fn localsubg(zz: &mut Globals, g: GraphId, id: i32) -> GraphId {
    if let Some(subg) = agfindsubg_by_id(zz, g, id) {
        return subg;
    }
    let parent = &zz.graphs[g];
    let (desc, clos) = (
        Agdesc_s {
            maingraph: 0,
            ..parent.desc
        },
        parent.clos,
    );
    let subg = new_graph(zz, desc, Some(g), clos);
    zz.graphs[subg].tag.id = id;
    agopen1(zz, subg)
}

/// `agsubg`: the subgraph of `g` called `name`, created when `cflag` is set.
pub fn agsubg(zz: &mut Globals, g: GraphId, name: Option<&str>, cflag: bool) -> Option<GraphId> {
    if name.is_some()
        && let Some(id) = agmapnametoid(zz, g, name, false)
        && let Some(subg) = agfindsubg_by_id(zz, g, id)
    {
        // It already exists.
        return Some(subg);
    }
    if cflag && let Some(id) = agmapnametoid(zz, g, name, true) {
        // Reserve the id.
        return Some(localsubg(zz, g, id));
    }
    None
}

/// `agfstsubg`: the first child of `g`, in id order.
pub fn agfstsubg(zz: &mut Globals, g: GraphId) -> Option<GraphId> {
    zz.graphs[g].g_dict.dtfirst()
}

/// `agnxtsubg`: the next child of `subg`'s parent.
pub fn agnxtsubg(zz: &mut Globals, subg: GraphId) -> Option<GraphId> {
    let g = agparent(zz, subg)?;
    let id = zz.graphs[subg].tag.id;
    zz.graphs[g].g_dict.dtnext(subg, id)
}

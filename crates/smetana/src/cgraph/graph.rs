//! `graph.c`: opening graphs, sequence numbers, counts.

use super::attr::agraphattr_init;
use super::edge::agsubrep;
use super::id::agmapnametoid;
use super::node::{agfstnode, agnxtnode};
use super::{AGEDGE, AGNODE, AGRAPH};
use crate::cdt::{DT_OSET, Dt, LinkId};
use crate::core::Globals;
use crate::core::ids::{ArenaId, ClosId, GraphId, NodeId};
use crate::h::Agraphinfo_t;
use crate::h::cgraph::{Agclos_s, Agdesc_s, Agraph_s, Agtag_s};

/// `agclos`: the resources a new root graph will share with its subgraphs.
fn agclos(zz: &mut Globals) -> ClosId {
    zz.closes.push(Agclos_s {
        strdict: Dt::dtopen(DT_OSET),
        seq: [0; 3],
    })
}

/// Allocates a graph with its dictionaries (`agalloc` plus the `agdtopen` calls of `agopen1`). A root graph is
/// its own root.
pub(crate) fn new_graph(
    zz: &mut Globals,
    desc: Agdesc_s,
    parent: Option<GraphId>,
    clos: ClosId,
) -> GraphId {
    let g = GraphId::from_index(zz.graphs.len());
    let root = parent.map_or(g, |p| zz.graphs[p].root);
    zz.graphs.push(Agraph_s {
        tag: Agtag_s {
            objtype: AGRAPH,
            ..Agtag_s::default()
        },
        desc,
        n_seq: Dt::dtopen(DT_OSET),
        n_id: Dt::dtopen(DT_OSET),
        e_seq: Dt::dtopen(DT_OSET),
        e_id: Dt::dtopen(DT_OSET),
        g_dict: Dt::dtopen(DT_OSET),
        parent,
        root,
        clos,
        recs: 0,
        attr: None,
        datadict: None,
        info: Agraphinfo_t::default(),
    })
}

/// `agopen`: a new root graph.
pub fn agopen(zz: &mut Globals, name: Option<&str>, desc: Agdesc_s) -> GraphId {
    let clos = agclos(zz);
    let g = new_graph(zz, desc, None, clos);
    if let Some(gid) = agmapnametoid(zz, g, name, true) {
        zz.graphs[g].tag.id = gid;
    }
    agopen1(zz, g)
}

/// `agopen1`: links a new graph into its parent and initialises its attributes.
pub(crate) fn agopen1(zz: &mut Globals, g: GraphId) -> GraphId {
    let par = agparent(zz, g);
    if let Some(par) = par {
        zz.graphs[g].tag.seq = agnextseq(zz, par, AGRAPH);
        let id = zz.graphs[g].tag.id;
        zz.graphs[par].g_dict.dtinsert(g, id);
    }
    if par.is_none_or(|p| zz.graphs[p].desc.has_attrs != 0) {
        agraphattr_init(zz, g);
    }
    g
}

/// `agparent`.
pub fn agparent(zz: &Globals, g: GraphId) -> Option<GraphId> {
    zz.graphs[g].parent
}

/// `agnextseq`: the next sequence number for graphs, nodes or edges of `g`'s root graph.
pub fn agnextseq(zz: &mut Globals, g: GraphId, objtype: i32) -> i32 {
    debug_assert!(matches!(objtype, AGRAPH | AGNODE | AGEDGE));
    let clos = zz.graphs[g].clos;
    let seq = &mut zz.closes[clos].seq[objtype as usize];
    *seq += 1;
    *seq
}

/// `agnnodes`.
pub fn agnnodes(zz: &mut Globals, g: GraphId) -> i32 {
    zz.graphs[g].n_id.dtsize_()
}

/// `agnedges`.
pub fn agnedges(zz: &mut Globals, g: GraphId) -> i32 {
    let mut rv = 0;
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        // Must use OUT to get self-arcs.
        rv += agdegree(zz, g, nn, false, true);
        n = agnxtnode(zz, g, nn);
    }
    rv
}

/// `agisdirected`.
pub fn agisdirected(zz: &Globals, g: GraphId) -> bool {
    zz.graphs[g].desc.directed != 0
}

/// `agisundirected`.
pub fn agisundirected(zz: &Globals, g: GraphId) -> bool {
    !agisdirected(zz, g)
}

/// `agisstrict`.
pub fn agisstrict(zz: &Globals, g: GraphId) -> bool {
    zz.graphs[g].desc.strict != 0
}

/// `cnt`: the size of an edge set detached from `g.e_seq`.
fn cnt(zz: &mut Globals, g: GraphId, set: &mut Option<LinkId>) -> i32 {
    let d = &mut zz.graphs[g].e_seq;
    d.dtrestore(*set);
    let rv = d.dtsize_();
    *set = d.dtextract();
    rv
}

/// `agdegree`.
pub fn agdegree(zz: &mut Globals, g: GraphId, n: NodeId, want_in: bool, want_out: bool) -> i32 {
    let Some(sn) = agsubrep(zz, g, n) else {
        return 0;
    };
    let mut rv = 0;
    if want_out {
        let mut set = zz.subnodes[sn].out_seq;
        rv += cnt(zz, g, &mut set);
        zz.subnodes[sn].out_seq = set;
    }
    if want_in {
        let mut set = zz.subnodes[sn].in_seq;
        rv += cnt(zz, g, &mut set);
        zz.subnodes[sn].in_seq = set;
    }
    rv
}

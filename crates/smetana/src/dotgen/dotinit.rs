//! `dotgen/dotinit.c`: dot's entry point (`dot_layout`), the initialisation of its records, and the sequence of
//! phases.

use crate::cgraph::Agobj;
use crate::cgraph::attr::agfindgraphattr;
use crate::cgraph::edge::{agfstout, agnxtout};
use crate::cgraph::graph::agnnodes;
use crate::cgraph::node::{agfstnode, agnxtnode};
use crate::cgraph::obj::{agraphof, agroot};
use crate::cgraph::rec::{Rec, agbindrec};
use crate::cgraph::subg::{agfstsubg, agnxtsubg};
use crate::cgraph::{aghead, agtail};
use crate::common::postproc::dotneato_postprocess;
use crate::common::utils::{
    agget_text, common_init_edge, common_init_node, gv_nodesize, late_int, late_string, mapbool,
    setEdgeType,
};
use crate::core::Globals;
use crate::core::consts::{CL_OFFSET, ET_SPLINE, NEW_RANK};
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::dotgen::aspect::{aspect_t, setAspect};
use crate::dotgen::class1::nonconstraint_edge;
use crate::dotgen::dotsplines::dot_splines;
use crate::dotgen::mincross::dot_mincross;
use crate::dotgen::position::dot_position;
use crate::dotgen::rank::dot_rank;
use crate::dotgen::sameport::dot_sameports;
use crate::h::{alloc_elist, elist};
use crate::pack::{EN_pack_mode, getPack, getPackInfo, getPackModeInfo, pack_info};

/// `dot_init_subg`: binds the dot record of every subgraph and remembers the root of the layout.
pub fn dot_init_subg(zz: &mut Globals, g: GraphId, droot: GraphId) {
    if g != agroot(zz, g) {
        agbindrec(zz, g, Rec::Info);
    }
    if g == droot {
        let root = agroot(zz, g);
        zz.gd_mut(root).dotroot = Some(droot);
    }
    let mut subg = agfstsubg(zz, g);
    while let Some(s) = subg {
        dot_init_subg(zz, s, droot);
        subg = agnxtsubg(zz, s);
    }
}

fn new_elist(zz: &mut Globals, n: i32) -> elist {
    let mut l = elist::default();
    alloc_elist(&mut zz.edge_lists, n, &mut l);
    l
}

/// `dot_init_node`.
fn dot_init_node(zz: &mut Globals, n: NodeId) {
    agbindrec(zz, n, Rec::Info);
    common_init_node(zz, n);
    let flip = zz.gd(agraphof(zz, n)).GD_flip();
    gv_nodesize(zz, n, flip);
    let in_ = new_elist(zz, 4);
    let out = new_elist(zz, 4);
    let flat_in = new_elist(zz, 2);
    let flat_out = new_elist(zz, 2);
    let other = new_elist(zz, 2);
    let info = zz.nd_mut(n);
    info.in_ = in_;
    info.out = out;
    info.flat_in = flat_in;
    info.flat_out = flat_out;
    info.other = other;
    info.UF_size = 1;
}

/// `dot_init_edge`.
fn dot_init_edge(zz: &mut Globals, e: EdgeId) {
    agbindrec(zz, e, Rec::Info);
    common_init_edge(zz, e);

    zz.ed_mut(e).weight = late_int(zz, e, zz.E_weight, 1, 0);
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    let tailgroup = late_string(zz, tail, zz.N_group, Some("")).unwrap_or_default();
    let headgroup = late_string(zz, head, zz.N_group, Some("")).unwrap_or_default();
    zz.ed_mut(e).count = 1;
    zz.ed_mut(e).xpenalty = 1;
    if !tailgroup.is_empty() && tailgroup == headgroup {
        unimplemented!("node groups");
    }
    if nonconstraint_edge(zz, e) {
        unimplemented!("constraint=false");
    }
    zz.ed_mut(e).showboxes = late_int(zz, e, zz.E_showboxes, 0, 0);
    zz.ed_mut(e).minlen = late_int(zz, e, zz.E_minlen, 1, 0);
}

/// `dot_init_node_edge`: the dot records of all nodes, then of all edges.
pub fn dot_init_node_edge(zz: &mut Globals, g: GraphId) {
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        dot_init_node(zz, nn);
        n = agnxtnode(zz, g, nn);
    }
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        let mut e = agfstout(zz, g, nn);
        while let Some(ee) = e {
            dot_init_edge(zz, ee);
            e = agnxtout(zz, g, ee);
        }
        n = agnxtnode(zz, g, nn);
    }
}

/// `attach_phase_attrs`: writing ranks and orders to attributes for the `phase` attribute, which Smetana does not
/// support.
fn attach_phase_attrs(maxphase: i32) -> ! {
    unimplemented!("attach_phase_attrs({maxphase})")
}

/// `dotLayout`: initialisation, then the phases: rank, mincross, position, sameports, splines.
pub fn dotLayout(zz: &mut Globals, g: GraphId) {
    let mut aspect = aspect_t::default();
    let phase = agfindgraphattr(zz, g, "phase");
    let maxphase = late_int(zz, g, phase, -1, 1);

    setEdgeType(zz, g, ET_SPLINE);
    let asp = setAspect(zz, g, &mut aspect);

    dot_init_subg(zz, g, g);
    dot_init_node_edge(zz, g);

    loop {
        dot_rank(zz, g, asp.as_ref());
        if maxphase == 1 {
            attach_phase_attrs(maxphase);
        }
        if aspect.badGraph != 0 {
            unimplemented!("aspect on disconnected graphs or graphs with clusters");
        }
        dot_mincross(zz, g, asp.is_some());
        if maxphase == 2 {
            attach_phase_attrs(maxphase);
        }
        dot_position(zz, g, asp.as_ref());
        if maxphase == 3 {
            attach_phase_attrs(maxphase);
        }
        aspect.nPasses -= 1;
        if aspect.nextIter == 0 || aspect.nPasses == 0 {
            break;
        }
    }
    if (zz.gd(g).flags & NEW_RANK) != 0 {
        unimplemented!("removeFill");
    }
    dot_sameports(zz, g);
    dot_splines(zz, g);
    if mapbool(agget_text(zz, g, "compound").as_deref()) {
        unimplemented!("dot_compoundEdges");
    }
}

/// `doDot`: Smetana supports no packing, so the graph is laid out as a whole.
fn doDot(zz: &mut Globals, g: GraphId) {
    let mut pinfo = pack_info::default();
    let Pack = getPack(zz, g, -1, CL_OFFSET);
    let mode = getPackModeInfo(zz, g, EN_pack_mode::l_undef, &mut pinfo);
    getPackInfo(zz, g, EN_pack_mode::l_node, 8, &mut pinfo);
    if mode == EN_pack_mode::l_undef && Pack < 0 {
        // No pack information: old dot, with components handled during layout.
        dotLayout(zz, g);
    } else {
        unimplemented!("packing");
    }
}

/// `dot_layout`.
pub fn dot_layout(zz: &mut Globals, g: GraphId) {
    if agnnodes(zz, g) != 0 {
        doDot(zz, g);
    }
    dotneato_postprocess(zz, g);
}

/// `dot_root`: the root graph of the layout `p` belongs to.
pub fn dot_root(zz: &Globals, p: impl Into<Agobj>) -> GraphId {
    zz.gd(agroot(zz, p)).dotroot.expect("GD_dotroot")
}

//! `dotgen/position.c`: node coordinates. Only the auxiliary edges ranking needs are ported so far.

use crate::cgraph::{M_aghead, M_agtail};
use crate::core::Globals;
use crate::core::consts::USHRT_MAX;
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::core::jmath::ROUND;
use crate::dotgen::aspect::aspect_t;
use crate::dotgen::fastgr::{fast_edge, new_edge_pair};

/// `largeMinlen`: Smetana cannot lay out edges longer than 65535 points.
fn largeMinlen(l: f64) -> f64 {
    unimplemented!("largeMinlen({l})")
}

/// `make_aux_edge`: an auxiliary edge from `u` to `v` of minimum length `len` and weight `wt`, in the fast graph.
pub fn make_aux_edge(zz: &mut Globals, u: NodeId, v: NodeId, len: f64, wt: i32) -> EdgeId {
    let e = new_edge_pair(zz);
    M_agtail(zz, e, u);
    M_aghead(zz, e, v);
    let len = if len > f64::from(USHRT_MAX) {
        largeMinlen(len)
    } else {
        len
    };
    zz.ed_mut(e).minlen = ROUND(len);
    zz.ed_mut(e).weight = wt;
    fast_edge(zz, e);
    e
}

/// `dot_position`.
pub fn dot_position(_zz: &mut Globals, _g: GraphId, _asp: Option<&aspect_t>) {
    unimplemented!("dot_position")
}

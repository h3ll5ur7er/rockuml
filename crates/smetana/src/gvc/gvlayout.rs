//! `gvc/gvlayout.c`: running a layout job. Smetana only has dot, so there is no engine to select.

#![allow(non_snake_case)]

use crate::cgraph::obj::agroot;
use crate::cgraph::rec::{Rec, agbindrec};
use crate::common::input::graph_init;
use crate::core::Globals;
use crate::core::ids::GraphId;
use crate::dotgen::dotinit::dot_layout;

/// `gvLayoutJobs`: lays out the root graph `g` with dot.
pub fn gvLayoutJobs(zz: &mut Globals, g: GraphId) -> i32 {
    agbindrec(zz, g, Rec::Info);
    if g != agroot(zz, g) {
        unimplemented!("laying out a subgraph");
    }
    // dot's features include LAYOUT_USES_RANKDIR.
    graph_init(zz, g, true);
    dot_layout(zz, g);
    0
}

//! `dotgen/aspect.c`: the `aspect` attribute, which Graphviz 2.38 disabled. Layouts never get aspect data.

use crate::common::utils::agget_text;
use crate::core::Globals;
use crate::core::ids::GraphId;

/// `aspect_t`: the iteration state of aspect-driven layout.
#[derive(Clone, Copy, Debug, Default)]
pub struct aspect_t {
    pub nextIter: i32,
    pub nPasses: i32,
    pub badGraph: i32,
}

/// `setAspect`: resets `adata`; returns the aspect data to lay out with, which is always none.
pub fn setAspect(zz: &mut Globals, g: GraphId, adata: &mut aspect_t) -> Option<aspect_t> {
    if agget_text(zz, g, "aspect").is_some() {
        unimplemented!("the aspect attribute");
    }
    adata.nextIter = 0;
    adata.badGraph = 0;
    None
}

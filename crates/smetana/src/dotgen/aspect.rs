//! `dotgen/aspect.c`: the `aspect` attribute, which Graphviz 2.38 disabled. Without it, `setAspect` returns no
//! aspect data, so `dotLayout` makes a single pass and never balances.

use crate::common::utils::agget_text;
use crate::core::Globals;
use crate::core::ids::GraphId;

/// `setAspect`: rejects the `aspect` attribute, which Smetana does not support.
pub fn setAspect(zz: &mut Globals, g: GraphId) {
    if agget_text(zz, g, "aspect").is_some() {
        unimplemented!("the aspect attribute");
    }
}

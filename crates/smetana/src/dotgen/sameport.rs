//! `dotgen/sameport.c`: merging edge ends with the same `samehead`/`sametail`, which PlantUML never sets.

use crate::cgraph::AGEDGE;
use crate::cgraph::attr::agattr;
use crate::core::Globals;
use crate::core::ids::GraphId;

/// `dot_sameports`.
pub fn dot_sameports(zz: &mut Globals, g: GraphId) {
    zz.E_samehead = agattr(zz, Some(g), AGEDGE, "samehead", None);
    zz.E_sametail = agattr(zz, Some(g), AGEDGE, "sametail", None);
    if zz.E_samehead.is_some() || zz.E_sametail.is_some() {
        unimplemented!("samehead and sametail");
    }
}

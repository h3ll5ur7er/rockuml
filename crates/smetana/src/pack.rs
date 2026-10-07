//! `pack/pack.c`: the packing options of disconnected components, which PlantUML never sets.

#![allow(non_snake_case, non_camel_case_types)]

use crate::common::utils::agget_text;
use crate::core::Globals;
use crate::core::ids::GraphId;

/// `pack_mode`: Smetana supports none of the packing modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EN_pack_mode {
    l_undef,
}

/// `parsePackModeInfo`: the mode, which is `dflt` because any `packmode` is unsupported.
fn parsePackModeInfo(p: Option<&str>, dflt: EN_pack_mode) -> EN_pack_mode {
    if p.is_some_and(|p| !p.is_empty()) {
        unimplemented!("packmode");
    }
    dflt
}

/// `getPackModeInfo`: the `packmode` attribute.
pub fn getPackModeInfo(zz: &mut Globals, g: GraphId, dflt: EN_pack_mode) -> EN_pack_mode {
    let p = agget_text(zz, g, "packmode");
    parsePackModeInfo(p.as_deref(), dflt)
}

/// `getPack`: the `pack` attribute, `not_def` if undeclared. Any `pack` is unsupported, so the default for an
/// empty one is not needed.
pub fn getPack(zz: &mut Globals, g: GraphId, not_def: i32) -> i32 {
    if agget_text(zz, g, "pack").is_some() {
        unimplemented!("pack");
    }
    not_def
}

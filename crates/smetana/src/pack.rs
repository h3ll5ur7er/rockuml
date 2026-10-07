//! `pack/pack.c`: the packing options of disconnected components, which PlantUML never sets.

#![allow(non_snake_case, non_camel_case_types)]

use crate::common::utils::agget_text;
use crate::core::Globals;
use crate::core::ids::GraphId;

/// `pack_mode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EN_pack_mode {
    #[default]
    l_undef,
    l_node,
}

/// `pack_info`, as far as dot reads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct pack_info {
    pub margin: i32,
    pub doSplines: i32,
    pub mode: EN_pack_mode,
    pub sz: i32,
    pub flags: i32,
}

/// `parsePackModeInfo`.
fn parsePackModeInfo(p: Option<&str>, dflt: EN_pack_mode, pinfo: &mut pack_info) -> EN_pack_mode {
    pinfo.flags = 0;
    pinfo.mode = dflt;
    pinfo.sz = 0;
    if p.is_some_and(|p| !p.is_empty()) {
        unimplemented!("packmode");
    }
    pinfo.mode
}

/// `getPackModeInfo`: the `packmode` attribute.
pub fn getPackModeInfo(
    zz: &mut Globals,
    g: GraphId,
    dflt: EN_pack_mode,
    pinfo: &mut pack_info,
) -> EN_pack_mode {
    let p = agget_text(zz, g, "packmode");
    parsePackModeInfo(p.as_deref(), dflt, pinfo)
}

/// `getPack`: the `pack` attribute, `not_def` if undeclared.
pub fn getPack(zz: &mut Globals, g: GraphId, not_def: i32, _dflt: i32) -> i32 {
    if agget_text(zz, g, "pack").is_some() {
        unimplemented!("pack");
    }
    not_def
}

/// `getPackInfo`.
pub fn getPackInfo(
    zz: &mut Globals,
    g: GraphId,
    dflt: EN_pack_mode,
    dfltMargin: i32,
    pinfo: &mut pack_info,
) -> EN_pack_mode {
    pinfo.margin = getPack(zz, g, dfltMargin, dfltMargin);
    pinfo.doSplines = 0;
    getPackModeInfo(zz, g, dflt, pinfo);
    pinfo.mode
}

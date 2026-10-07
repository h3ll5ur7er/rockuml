//! Graphviz's `lib/common` (Smetana's `gen/lib/common`): what all layout engines share.

// Graphviz's names keep this module greppable against Smetana's sources.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(
    clippy::similar_names,
    clippy::manual_midpoint,
    reason = "Graphviz's names, and its `(a + b) / 2`, which `f64::midpoint` need not round alike"
)]

pub mod arrows;
pub mod emit;
pub mod geom;
pub mod routespl;
pub mod shapes_inside;
pub mod splines;
pub mod utils_routing;

use crate::core::Globals;
use crate::core::ids::GraphId;

/// `GD_rankdir(g)`: the rank direction without the flag bits above it.
pub fn GD_rankdir(zz: &Globals, g: GraphId) -> i32 {
    zz.gd(g).rankdir & 0x3
}

/// `GD_flip(g)`: whether ranks run left to right or right to left.
pub fn GD_flip(zz: &Globals, g: GraphId) -> bool {
    GD_rankdir(zz, g) & 1 != 0
}

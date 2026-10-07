//! `gen/lib/common`: the code dot shares with Graphviz's other layouts: graph and node initialisation, shapes,
//! labels, network simplex, spline clipping and routing, and post-processing.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::similar_names, reason = "Graphviz's names")]
#![allow(
    clippy::manual_midpoint,
    reason = "f64::midpoint may round differently from Java's (a + b) / 2"
)]

pub(crate) mod arrows;
pub(crate) mod emit;
pub(crate) mod geom;
pub(crate) mod input;
pub(crate) mod labels;
pub(crate) mod ns;
pub(crate) mod postproc;
pub(crate) mod routespl;
pub(crate) mod shapes;
pub(crate) mod shapes_inside;
pub(crate) mod splines;
pub(crate) mod utils;

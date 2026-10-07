//! `gen/lib/common`: the code dot shares with Graphviz's other layouts: graph and node initialisation, shapes,
//! labels, network simplex, spline clipping and routing, and post-processing.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::similar_names, reason = "Graphviz's names")]
#![allow(
    clippy::manual_midpoint,
    reason = "f64::midpoint may round differently from Java's (a + b) / 2"
)]

pub mod arrows;
pub mod emit;
pub mod geom;
pub mod input;
pub mod labels;
pub mod ns;
pub mod postproc;
pub mod routespl;
pub mod shapes;
pub mod shapes_inside;
pub mod splines;
pub mod utils;

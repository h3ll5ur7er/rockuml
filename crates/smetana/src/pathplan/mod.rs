//! Graphviz's path planner (`lib/pathplan`, Smetana's `gen/lib/pathplan`): the shortest path between two points
//! inside a polygon (`Pshortestpath`) and a piecewise Bézier spline along it that stays clear of barriers
//! (`Proutespline`).
//!
//! The planner's former C statics, which Smetana keeps in `Globals`, live in a [`PathplanContext`]. The output
//! buffers among them (`ops` of shortest.c and route.c, `ispline` of util.c) are not part of it: their contents
//! were only ever read through the output polyline, which here owns its points.

// Graphviz's names keep this module greppable against Smetana's sources.
#![allow(non_camel_case_types, non_snake_case, clippy::similar_names)]

mod route;
mod shortest;
mod solvers;
mod util;

pub use route::Proutespline;
pub use shortest::{PathplanError, Pshortestpath};
pub use solvers::solve3;
pub use util::make_polyline;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pxy_t {
    pub x: f64,
    pub y: f64,
}

pub type Ppoint_t = Pxy_t;
pub type Pvector_t = Pxy_t;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ppoly_t {
    pub ps: Vec<Ppoint_t>,
}

pub type Ppolyline_t = Ppoly_t;

/// A barrier segment for `Proutespline`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pedge_t {
    pub a: Ppoint_t,
    pub b: Ppoint_t,
}

/// The planner's scratch arrays. They only ever grow and are reused from call to call, as in Smetana, where one
/// layout's calls share them.
#[derive(Debug, Default)]
pub struct PathplanContext {
    shortest: shortest::Scratch,
    tnas: Vec<route::tna_t>,
}

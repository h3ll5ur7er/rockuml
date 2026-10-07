//! The runtime that `smetana/core` provides the Java port: the layout context, typed handles for objects,
//! C-style arrays, and Java's arithmetic and C-library replacements.

pub mod carray;
pub mod consts;
mod globals;
pub mod ids;
pub mod jmath;
pub mod jutils;

pub use globals::Globals;

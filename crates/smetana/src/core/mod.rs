//! The runtime that `smetana/core` provides the Java port: the layout context, typed handles for objects,
//! C-style arrays, and Java's arithmetic and C-library replacements.

pub(crate) mod carray;
pub(crate) mod consts;
mod globals;
pub(crate) mod ids;
pub(crate) mod jmath;
pub(crate) mod jutils;

pub use globals::Globals;

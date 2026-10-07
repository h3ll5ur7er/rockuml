//! `lib/dotgen`: the dot layout: ranking, ordering within ranks, positions and edge routes.

#![allow(non_snake_case, non_camel_case_types)]
#![allow(clippy::similar_names, reason = "Graphviz's names")]

pub(crate) mod acyclic;
pub(crate) mod aspect;
pub(crate) mod class1;
pub(crate) mod class2;
pub(crate) mod cluster;
pub(crate) mod decomp;
pub(crate) mod dotinit;
pub(crate) mod dotsplines;
pub(crate) mod fastgr;
pub(crate) mod flat;
pub(crate) mod mincross;
pub(crate) mod position;
pub(crate) mod rank;
pub(crate) mod sameport;

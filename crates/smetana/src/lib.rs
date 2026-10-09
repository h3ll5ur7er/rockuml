//! Graph layout as PlantUML does it without Graphviz: a port of Smetana, PlantUML's Java transliteration of
//! Graphviz `dot` 2.38 (`gen/`, `h/` and `smetana/` in the reference sources). Layouts must match Smetana's
//! coordinates exactly, so the port keeps Graphviz's function names, evaluation order and integer semantics.
//!
//! Like Graphviz and Smetana, the crate is distributed under the Eclipse Public License 1.0 (see `LICENSE`).
//!
//! # Use
//!
//! Build a [`Graph`] with the calls PlantUML makes, in PlantUML's order, then [`Graph::layout`] it and read the
//! coordinates from the [`Drawing`]. Everything else is the ported code, private to the crate.
//!
//! # Conventions for ported code
//!
//! **Context.** Every Java function that takes `Globals zz` takes `zz: &mut Globals` first (`&Globals` when it
//! only reads). [`Globals`](core::Globals) owns all objects in arenas and holds the former C statics; a module
//! with private statics adds its fields there.
//!
//! **Objects are ids.** `ST_Agraph_s`, `ST_Agnode_s` and `ST_Agedge_s` references become
//! [`GraphId`](core::ids::GraphId), [`NodeId`](core::ids::NodeId) and [`EdgeId`](core::ids::EdgeId): `Copy`
//! handles compared with `==` like Java references; `null` is `None`. An `EdgeId` names one half of an edge pair,
//! and `AGOPP`/`AGMKOUT`/`AGMKIN` switch halves ([`cgraph`]). Virtual nodes and edges are made with
//! [`Globals::new_agnode`](core::Globals::new_agnode) and
//! [`Globals::new_agedgepair`](core::Globals::new_agedgepair), the equivalents of `new ST_Agnode_s()` and
//! `new ST_Agedgepair_s()`; they are never in cgraph's dictionaries, exactly as in Java.
//!
//! **Records.** The `GD_`/`ND_`/`ED_` macros become field accesses on the info records ([`h`]), reached through
//! the context:
//!
//! | Java | Rust |
//! |---|---|
//! | `ND_rank(n)` | `zz.nd(n).rank` |
//! | `ND_rank(n, r)` | `zz.nd_mut(n).rank = r` |
//! | `ND_in(n)` (`in` is a keyword) | `zz.nd(n).in_` |
//! | `ED_minlen(e)` | `zz.ed(e).minlen` (both halves share the record) |
//! | `GD_nodesep(g)` | `zz.gd(g).nodesep` |
//! | `GD_rank(g)[r].n` | `zz.rank(g, r).n` / `zz.rank_mut(g, r).n` |
//! | `agtail(e)`, `aghead(e)` | `agtail(zz, e)`, `aghead(zz, e)` |
//! | `M_agtail(e, v)`, `MAKEFWDEDGE(new, old)` | `M_agtail(zz, e, v)`, `MAKEFWDEDGE(zz, new, old)` |
//! | `AGSEQ(n)`, `n.tag.id` | `AGSEQ(zz, n)`, `zz.tag(n).id` |
//! | `AGSEQ(e, s)`, `AGTYPE(e, t)` | `zz.tag_mut(e).seq = s`, `zz.tag_mut(e).objtype = t` |
//! | `VIRTUAL`, `CL_OFFSET`... | [`core::consts`] |
//!
//! Rust evaluates the right side of an assignment first, so `zz.nd_mut(n).rank = zz.nd(m).rank + 1` compiles.
//! Records are `Copy`; to hand one sub-struct and the context to a function, copy it out and write it back
//! (`let mut l = zz.nd(n).out; elist_append(.., &mut l); zz.nd_mut(n).out = l`).
//!
//! **Pointers to shared structs** (text labels, splines, polygons, record fields, flat-edge matrices) are ids into
//! arenas of the context, so that copying a record aliases them exactly like the Java copy (`MAKEFWDEDGE`).
//!
//! **Arrays.** `CArray<T>` and `CArrayOfStar<T>` become [`CArray`](core::carray::CArray) handles (buffer plus
//! offset) into per-type stores of the context (`zz.node_lists`, `zz.edge_lists`, `zz.ranks`, `zz.pointfs`...).
//! They keep C's aliasing: `GD_rank(clust)[r].v` pointing into the root's, `rank[-1]`, and `REALLOC` returning the
//! same array. Element access is `zz.node_lists[v.at(i)]` or `zz.node_lists.get(v, i)`.
//!
//! **Numbers.** `int` is `i32`; where Java can overflow (comparators returning `a - b`, hashes) use wrapping
//! arithmetic. `(int) d` is `d as i32`. Use [`jmath`](core::jmath) for `ROUND`, `POINTS`, `hypot`,
//! `Math.min`/`max` and the transcendental functions, never `f64::round`, `f64::min` or `mul_add`. Sorting is
//! [`qsort`](core::jutils::qsort), Smetana's stable bubble sort.
//!
//! **Unsupported paths.** Code Smetana stubs with `UNSUPPORTED` (or that throws) becomes `unimplemented!()` or a
//! panic; PlantUML never reaches it on valid input. [`Graph::layout`] turns these panics into errors.
//!
//! **Deviations.** Where Java hangs, the port deviates to terminate, and says so where it does.
//!
//! **Names.** Types and functions keep their C/Java names, so `non_camel_case_types`/`non_snake_case` are allowed
//! where they appear.

#![allow(
    clippy::missing_panics_doc,
    reason = "panics stand for the exceptions Smetana throws on input PlantUML never produces"
)]
#![allow(
    rustdoc::private_intra_doc_links,
    reason = "the conventions are for maintainers, who document the private items"
)]

mod api;
mod cdt;
mod cgraph;
mod common;
mod core;
mod dotgen;
mod gvc;
mod h;
mod label;
mod pack;
mod pathplan;

pub use api::{
    Bezier, BoundingBox, Drawing, Edge, EdgeLayout, Graph, Label, LayoutError, Node, NodeLayout,
    Object, Point, Subgraph, SubgraphLayout,
};

/// Not part of the API: the ported functions and records that the integration tests drive one phase at a time,
/// to compare the port's state with Java's traces. The paths mirror the crate's modules.
#[doc(hidden)]
pub mod internals {
    pub mod cgraph {
        pub use crate::cgraph::{
            AGEDGE, AGINEDGE, AGMKOUT, AGNODE, AGOPP, AGOUTEDGE, Agobj, M_aghead, M_agtail, aghead,
            agtail,
        };
        pub mod attr {
            pub use crate::cgraph::attr::{agattr, agget, agsafeset, agxget, agxset};
        }
        pub mod edge {
            pub use crate::cgraph::edge::{
                agedge, agfindedge, agfstedge, agfstin, agfstout, agnxtedge, agnxtin, agnxtout,
                agsubedge,
            };
        }
        pub mod graph {
            pub use crate::cgraph::graph::{agdegree, agnedges, agnnodes, agopen};
        }
        pub mod id {
            pub use crate::cgraph::id::agnameof;
        }
        pub mod node {
            pub use crate::cgraph::node::{agfstnode, agnode, agnxtnode};
        }
        pub mod obj {
            pub use crate::cgraph::obj::agcontains;
        }
        pub mod rec {
            pub use crate::cgraph::rec::{Rec, agbindrec};
        }
        pub mod subg {
            pub use crate::cgraph::subg::{agfstsubg, agnxtsubg, agsubg};
        }
    }
    pub mod common {
        pub mod input {
            pub use crate::common::input::graph_init;
        }
        pub mod postproc {
            pub use crate::common::postproc::dotneato_postprocess;
        }
        pub mod routespl {
            pub use crate::common::routespl::{routepolylines, routesplines, simpleSplineRoute};
        }
        pub mod shapes {
            pub use crate::common::shapes::{bind_shape, portfn};
        }
        pub mod splines {
            pub use crate::common::splines::{beginpath, clip_and_install, endpath, makeSelfEdge};
        }
        pub mod utils {
            pub use crate::common::utils::setEdgeType;
        }
    }
    pub mod core {
        pub use crate::core::Globals;
        pub mod consts {
            pub use crate::core::consts::{ET_SPLINE, VIRTUAL};
        }
        pub mod ids {
            pub use crate::core::ids::{EdgeId, FieldId, GraphId, NodeId, SymId, TextlabelId};
        }
    }
    pub mod dotgen {
        pub mod aspect {
            pub use crate::dotgen::aspect::setAspect;
        }
        pub mod dotinit {
            pub use crate::dotgen::dotinit::{dot_init_node_edge, dot_init_subg};
        }
        pub mod dotsplines {
            pub use crate::dotgen::dotsplines::{dot_splines, spline_merge, swap_ends_p};
        }
        pub mod mincross {
            pub use crate::dotgen::mincross::dot_mincross;
        }
        pub mod position {
            pub use crate::dotgen::position::dot_position;
        }
        pub mod rank {
            pub use crate::dotgen::rank::dot_rank;
        }
        pub mod sameport {
            pub use crate::dotgen::sameport::dot_sameports;
        }
    }
    pub mod gvc {
        pub mod gvlayout {
            pub use crate::gvc::gvlayout::gvLayoutJobs;
        }
    }
    pub mod h {
        pub use crate::h::{
            SHAPE_INFO, bezier, boxf, field_t, path, pathend_t, point, pointf, polygon_t, port,
            splineInfo, splines, textlabel_t,
        };
        pub mod cgraph {
            pub use crate::h::cgraph::Agdirected;
        }
    }
    pub mod label {
        pub use crate::label::{
            Child, RTreeInsert, RTreeOpen, RTreeSearch, Rect_t, hd_hil_s_from_xy, label_params_t,
            object_t, placeLabels, xlabel_t,
        };
    }
    pub mod pathplan {
        pub use crate::pathplan::{
            PathplanContext, PathplanError, Pedge_t, Ppoint_t, Ppoly_t, Ppolyline_t, Proutespline,
            Pshortestpath, solve3,
        };
    }
}

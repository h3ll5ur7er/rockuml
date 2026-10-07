//! The C structs of the `h/` package (`ST_*`), minus the prefix, keeping Graphviz's type and field names.
//!
//! Pointers become typed ids ([`crate::core::ids`]) or [`CArray`] handles, so every struct whose Java version is
//! copied with `___()` is `Copy` here, and a copy aliases what the Java copy aliases. Structs Java allocates
//! individually and shares by reference (text labels, splines, polygons, record fields, flat-edge matrices) live in
//! arenas of [`Globals`](crate::core::Globals) and are referred to by id.
//!
//! The cgraph objects (`Agraph_s`, `Agnode_s`...) are in [`cgraph`]. Structs that belong to a single module and
//! never reach the info records (pathplan's polygons and triangles, xlabels' R-tree, routespl's boxes...) are
//! defined by that module.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

pub mod cgraph;

use crate::core::Globals;
use crate::core::carray::{CArray, CArrays};
use crate::core::ids::{
    AdjmatrixId, EdgeId, FieldId, GraphId, NodeId, PolygonId, ShapeDescId, SplinesId, StrId,
    TextlabelId,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct pointf {
    pub x: f64,
    pub y: f64,
}

/// `pointfof`.
pub fn pointfof(x: f64, y: f64) -> pointf {
    pointf { x, y }
}

/// `add_pointf`.
pub fn add_pointf(p: pointf, q: pointf) -> pointf {
    pointf {
        x: p.x + q.x,
        y: p.y + q.y,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct boxf {
    pub LL: pointf,
    pub UR: pointf,
}

/// An edge end's port. `bp` points at a record field's box in C; it is a copy here because field boxes are final
/// by the time ports are resolved.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(clippy::struct_excessive_bools, reason = "Graphviz's struct")]
pub struct port {
    pub p: pointf,
    pub theta: f64,
    pub bp: Option<boxf>,
    pub defined: bool,
    pub constrained: bool,
    pub clip: bool,
    pub dyna: bool,
    pub order: i32,
    pub side: i32,
    pub name: Option<StrId>,
}

/// `Globals.Center`: the port at a node's center.
pub const Center: port = port {
    p: pointf { x: 0.0, y: 0.0 },
    theta: -1.0,
    bp: None,
    defined: false,
    constrained: false,
    clip: true,
    dyna: false,
    order: 0,
    side: 0,
    name: None,
};

/// A NULL-terminated edge list.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct elist {
    pub size: i32,
    pub list: Option<CArray<Option<EdgeId>>>,
}

impl elist {
    /// `L.list[i]`; the list must be allocated, as Java would throw otherwise.
    pub fn get(&self, lists: &CArrays<Option<EdgeId>>, i: i32) -> Option<EdgeId> {
        lists.get(self.list.expect("elist without list"), i)
    }
}

/// `elist_append`: appends `item`, keeping the list NULL-terminated.
pub fn elist_append(lists: &mut CArrays<Option<EdgeId>>, item: EdgeId, L: &mut elist) {
    let list = lists.REALLOC(L.size + 2, L.list);
    L.list = Some(list);
    lists.set(list, L.size, Some(item));
    L.size += 1;
    lists.set(list, L.size, None);
}

/// `alloc_elist`: an empty list with room for `n` edges.
pub fn alloc_elist(lists: &mut CArrays<Option<EdgeId>>, n: i32, L: &mut elist) {
    L.size = 0;
    L.list = Some(lists.ALLOC(n + 1));
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct nlist_t {
    pub list: Option<CArray<Option<NodeId>>>,
    pub size: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct rank_t {
    pub n: i32,
    pub v: Option<CArray<Option<NodeId>>>,
    pub an: i32,
    pub av: Option<CArray<Option<NodeId>>>,
    pub ht1: f64,
    pub ht2: f64,
    pub pht1: f64,
    pub pht2: f64,
    pub candidate: bool,
    pub valid: i32,
    pub cache_nc: i32,
    pub flat: Option<AdjmatrixId>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct adjmatrix_t {
    pub data: Vec<Vec<i32>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct bezier {
    pub list: Option<CArray<pointf>>,
    pub size: i32,
    pub sflag: i32,
    pub eflag: i32,
    pub sp: pointf,
    pub ep: pointf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct splines {
    pub list: Option<CArray<bezier>>,
    pub size: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct textspan_t {
    pub str: String,
    pub size: pointf,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct textlabel_t {
    pub text: String,
    pub fontname: String,
    pub fontcolor: String,
    pub charset: i32,
    pub fontsize: f64,
    pub dimen: pointf,
    pub space: pointf,
    pub pos: pointf,
    pub span: Option<CArray<textspan_t>>,
    pub nspans: i32,
    pub set: i32,
    pub html: bool,
}

/// `layout_t`: of the drawing parameters, only `quantum` affects a layout Smetana can make; `graph_init` rejects
/// `ratio` and `size`, and the others only matter to renderers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct layout_t {
    pub quantum: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct polygon_t {
    pub regular: bool,
    pub peripheries: i32,
    pub sides: i32,
    pub orientation: f64,
    pub distortion: f64,
    pub skew: f64,
    pub option: i32,
    pub vertices: Option<CArray<pointf>>,
}

/// A record node's field tree.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct field_t {
    pub size: pointf,
    pub b: boxf,
    pub n_flds: i32,
    pub lp: Option<TextlabelId>,
    pub fld: Option<CArray<Option<FieldId>>>,
    pub id: Option<String>,
    pub LR: bool,
    pub sides: i32,
}

/// `ND_shape_info`: what a shape's init function attached to the node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SHAPE_INFO {
    Polygon(PolygonId),
    Field(FieldId),
}

/// Which `shape_functions` table a shape uses (`poly_fns` or `record_fns`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum shape_functions {
    poly_fns,
    record_fns,
}

#[derive(Clone, Debug, PartialEq)]
pub struct shape_desc {
    pub name: &'static str,
    pub fns: shape_functions,
    pub polygon: Option<polygon_t>,
    pub usershape: bool,
}

/// `Agraphinfo_t`, the dot record of a graph or cluster (`GD_*`). `gvc` and `cleanup` are left out: the context
/// is [`Globals`](crate::core::Globals) and nothing is cleaned up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Agraphinfo_t {
    pub drawing: Option<layout_t>,
    pub label: Option<TextlabelId>,
    pub bb: boxf,
    pub border: [pointf; 4],
    pub has_labels: i32,
    pub charset: i32,
    pub rankdir: i32,
    pub ht1: f64,
    pub ht2: f64,
    pub flags: i32,
    pub n_cluster: i32,
    pub clust: Option<CArray<Option<GraphId>>>,
    pub dotroot: Option<GraphId>,
    pub nlist: Option<NodeId>,
    pub rank: Option<CArray<rank_t>>,
    pub parent: Option<GraphId>,
    pub comp: nlist_t,
    pub minset: Option<NodeId>,
    pub maxset: Option<NodeId>,
    pub n_nodes: i32,
    pub minrank: i32,
    pub maxrank: i32,
    pub has_flat_edges: i32,
    pub nodesep: i32,
    pub ranksep: i32,
    pub ln: Option<NodeId>,
    pub rn: Option<NodeId>,
    pub leader: Option<NodeId>,
    pub rankleader: Option<CArray<Option<NodeId>>>,
    pub expanded: bool,
    pub installed: i32,
    pub label_pos: i32,
    pub exact_ranksep: i32,
}

impl Agraphinfo_t {
    /// `GD_rankdir(g)`: the effective rank direction, in the low two bits of `rankdir`.
    pub fn GD_rankdir(&self) -> i32 {
        self.rankdir & 0x3
    }

    /// `GD_flip(g)`: whether ranks run left to right (or right to left).
    pub fn GD_flip(&self) -> bool {
        self.GD_rankdir() & 1 != 0
    }

    /// `GD_realflip(g)`: like `GD_flip` for the requested rank direction, stored in the next two bits.
    pub fn GD_realflip(&self) -> bool {
        (self.rankdir >> 2) & 1 != 0
    }
}

/// `Agnodeinfo_t`, the dot record of a node (`ND_*`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Agnodeinfo_t {
    pub shape: Option<ShapeDescId>,
    pub shape_info: Option<SHAPE_INFO>,
    pub coord: pointf,
    pub width: f64,
    pub height: f64,
    pub ht: f64,
    pub lw: f64,
    pub rw: f64,
    pub label: Option<TextlabelId>,
    pub xlabel: Option<TextlabelId>,
    pub alg: Option<EdgeId>,
    pub id: i32,
    pub has_port: bool,
    pub node_type: i32,
    pub mark: i32,
    pub onstack: i32,
    pub ranktype: i32,
    pub weight_class: i32,
    pub next: Option<NodeId>,
    pub prev: Option<NodeId>,
    pub in_: elist,
    pub out: elist,
    pub flat_out: elist,
    pub flat_in: elist,
    pub other: elist,
    pub clust: Option<GraphId>,
    pub UF_size: i32,
    pub UF_parent: Option<NodeId>,
    pub inleaf: Option<NodeId>,
    pub outleaf: Option<NodeId>,
    pub rank: i32,
    pub order: i32,
    pub mval: f64,
    pub save_in: elist,
    pub save_out: elist,
    pub tree_in: elist,
    pub tree_out: elist,
    pub par: Option<EdgeId>,
    pub low: i32,
    pub lim: i32,
    pub priority: i32,
}

/// `Agedgeinfo_t`, the dot record of an edge (`ED_*`). `MAKEFWDEDGE` copies it whole.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Agedgeinfo_t {
    pub spl: Option<SplinesId>,
    pub tail_port: port,
    pub head_port: port,
    pub label: Option<TextlabelId>,
    pub head_label: Option<TextlabelId>,
    pub tail_label: Option<TextlabelId>,
    pub xlabel: Option<TextlabelId>,
    pub edge_type: i32,
    pub adjacent: i32,
    pub label_ontop: bool,
    pub to_orig: Option<EdgeId>,
    pub dist: f64,
    pub conc_opp_flag: bool,
    pub xpenalty: i32,
    pub weight: i32,
    pub cutvalue: i32,
    pub tree_index: i32,
    pub count: i32,
    pub minlen: i32,
    pub to_virt: Option<EdgeId>,
}

/// `path`: what spline routing needs to know about an edge: its end ports and the corridor of boxes it may use.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct path {
    pub start: port,
    pub end: port,
    pub nbox: i32,
    pub boxes: Vec<boxf>,
    pub data: Option<EdgeId>,
}

/// `pathend_t`: the boxes around one end node of a routed edge.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct pathend_t {
    pub nb: boxf,
    pub sidemask: i32,
    pub boxn: i32,
    pub boxes: [boxf; 20],
}

/// `splineInfo`: the layout engine's answers to the questions spline clipping asks.
#[derive(Clone, Copy)]
pub struct splineInfo {
    pub swapEnds: fn(&Globals, EdgeId) -> bool,
    pub splineMerge: fn(&Globals, NodeId) -> bool,
    pub ignoreSwap: bool,
    pub isOrtho: bool,
}

/// `inside_t`, a C union: the circle around an arrow tip (`a`) or the node and port box a spline is clipped to
/// (`s`). `a.p` points at the circle's centre in C; Smetana only reads it while the centre stays put.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct inside_t {
    pub a_p: pointf,
    pub a_r: f64,
    pub s_n: Option<NodeId>,
    pub s_bp: Option<boxf>,
}

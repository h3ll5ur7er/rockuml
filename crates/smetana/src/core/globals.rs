//! `Globals zz`: the context of one layout. It owns every object (in arenas) and holds what C keeps in statics.
//!
//! Java allocates a `Globals` per layout and passes it as `zz` to almost every function; the port does the same
//! with `zz: &mut Globals`. Statics carry state from one phase into the next (`S_i`, the network simplex's search
//! start, survives from ranking into positioning), so they must stay here rather than become locals.

#![allow(non_snake_case)]

use std::collections::HashMap;

use crate::cdt::{Dicts, JStr};
use crate::core::carray::{CArray, CArrays};
use crate::core::ids::{
    AdjmatrixId, Arena, ArenaId, ClosId, EdgeId, FieldId, GraphId, NodeId, PolygonId, ShapeDescId,
    SplinesId, StrId, SubnodeId, SymId, TextlabelId,
};
use crate::h::cgraph::{
    Agclos_s, Agedge_s, Agedgepair_s, Agnode_s, Agraph_s, Agsubnode_s, Agsym_s, refstr_t,
};
use crate::h::{
    Agedgeinfo_t, Agnodeinfo_t, Agraphinfo_t, adjmatrix_t, bezier, elist, field_t, nlist_t, pointf,
    polygon_t, rank_t, shape_desc, shape_functions, splines, textlabel_t, textspan_t,
};
use crate::pathplan::PathplanContext;

pub struct Globals {
    // cgraph's objects.
    pub graphs: Arena<GraphId, Agraph_s>,
    pub nodes: Arena<NodeId, Agnode_s>,
    /// Edge pairs, indexed by [`EdgeId`]'s pair number.
    pub(crate) edges: Vec<Agedgepair_s>,
    pub subnodes: Arena<SubnodeId, Agsubnode_s>,
    pub syms: Arena<SymId, Agsym_s>,
    pub refstrs: Arena<StrId, refstr_t>,
    pub closes: Arena<ClosId, Agclos_s>,
    /// The attribute dictionaries of all graphs (`Agdatadict_s`), which view their parents'.
    pub(crate) attr_dicts: Dicts<SymId, JStr>,
    /// `CString.UID`: the next `CString` uid. Only the strings that cgraph interns get one here; ids only need
    /// Java's order, and every other `CString` Java creates in between shifts them all alike.
    pub(crate) cstring_uid: i32,
    /// `ctr`: the next anonymous object id (odd, so it never collides with a string uid).
    pub ctr: i32,
    /// `all`: the interned string behind each named object id (`Memory.identityHashCode`).
    pub(crate) all: HashMap<i32, StrId>,
    pub ProtoGraph: Option<GraphId>,

    // Arrays (`CArray`, `CArrayOfStar`).
    pub node_lists: CArrays<Option<NodeId>>,
    pub edge_lists: CArrays<Option<EdgeId>>,
    pub graph_lists: CArrays<Option<GraphId>>,
    pub field_lists: CArrays<Option<FieldId>>,
    pub ranks: CArrays<rank_t>,
    pub pointfs: CArrays<pointf>,
    pub beziers: CArrays<bezier>,
    pub textspans: CArrays<textspan_t>,

    // Structs dot allocates and shares by pointer.
    pub textlabels: Arena<TextlabelId, textlabel_t>,
    pub splines: Arena<SplinesId, splines>,
    pub polygons: Arena<PolygonId, polygon_t>,
    pub fields: Arena<FieldId, field_t>,
    pub adjmatrices: Arena<AdjmatrixId, adjmatrix_t>,
    /// The shapes PlantUML uses: box, ellipse and record, in Java's order.
    pub Shapes: Arena<ShapeDescId, shape_desc>,

    // Attribute symbols that dot looks up once per layout.
    pub G_ordering: Option<SymId>,
    pub G_margin: Option<SymId>,
    pub N_height: Option<SymId>,
    pub N_width: Option<SymId>,
    pub N_shape: Option<SymId>,
    pub N_fontsize: Option<SymId>,
    pub N_fontname: Option<SymId>,
    pub N_fontcolor: Option<SymId>,
    pub N_label: Option<SymId>,
    pub N_xlabel: Option<SymId>,
    pub N_nojustify: Option<SymId>,

    pub N_peripheries: Option<SymId>,
    pub N_ordering: Option<SymId>,
    pub N_orientation: Option<SymId>,
    pub N_fixed: Option<SymId>,
    pub N_group: Option<SymId>,
    pub E_weight: Option<SymId>,
    pub E_minlen: Option<SymId>,
    pub E_fontsize: Option<SymId>,
    pub E_fontname: Option<SymId>,
    pub E_fontcolor: Option<SymId>,
    pub E_label: Option<SymId>,
    pub E_xlabel: Option<SymId>,
    pub E_dir: Option<SymId>,

    pub E_arrowsz: Option<SymId>,
    pub E_constr: Option<SymId>,
    pub E_label_float: Option<SymId>,
    pub E_samehead: Option<SymId>,
    pub E_sametail: Option<SymId>,
    pub E_arrowhead: Option<SymId>,
    pub E_arrowtail: Option<SymId>,
    pub E_headlabel: Option<SymId>,
    pub E_taillabel: Option<SymId>,
    pub E_labelfontsize: Option<SymId>,
    pub E_labelfontname: Option<SymId>,
    pub E_labelfontcolor: Option<SymId>,
    pub E_labeldistance: Option<SymId>,
    pub E_labelangle: Option<SymId>,
    pub E_tailclip: Option<SymId>,
    pub E_headclip: Option<SymId>,

    // The dot phases' statics, as declared in Globals.java, and PlantUML's own. The scratch state of shapes.c and
    // pathplan follows.
    pub CL_type: i32,
    pub Concentrate: bool,
    pub MaxIter: i32,
    pub State: i32,
    pub EdgeLabelsDone: i32,
    pub N_nodes: i32,
    pub Minrank: i32,
    pub Maxrank: i32,
    pub S_i: i32,
    pub Search_size: i32,
    pub Tree_node: nlist_t,
    pub Tree_edge: elist,
    pub Enter: Option<EdgeId>,
    pub Low: i32,
    pub Lim: i32,
    pub Slack: i32,
    pub Rankdir: i32,
    pub Flip: bool,
    pub Offset: pointf,
    pub MinQuit: i32,
    pub Convergence: f64,
    pub Root: Option<GraphId>,
    pub GlobalMinRank: i32,
    pub GlobalMaxRank: i32,
    pub ReMincross: bool,
    pub TI_list: Vec<i32>,
    pub Last_node_decomp: Option<NodeId>,
    /// A Java `char`: increments wrap at 16 bits.
    pub Cmark: u16,
    pub Count: Vec<i32>,
    pub C: i32,
    pub G_ns: Option<GraphId>,
    pub G_decomp: Option<GraphId>,
    /// PlantUML's: the flat edge label nodes that lost a constraint in `make_LR_constraints`, without duplicates.
    pub skippedConstraintLabelVnodes: Vec<NodeId>,

    // shapes.c: `poly_inside`'s cache of the last node it tested.
    pub lastn: Option<NodeId>,
    pub last: i32,
    pub outp: i32,
    pub sides: i32,
    pub vertex: Option<CArray<pointf>>,
    pub xsize: f64,
    pub ysize: f64,
    pub scalex: f64,
    pub scaley: f64,
    pub box_URx: f64,
    pub box_URy: f64,
    /// The path planner's scratch arrays, shared by all routes of a layout.
    pub pathplan: PathplanContext,
}

impl Globals {
    /// `Globals.open()`.
    #[allow(clippy::too_many_lines, reason = "one line per static")]
    pub fn open() -> Self {
        let mut shapes = Arena::default();
        shapes.push(shape_desc {
            name: "box",
            fns: shape_functions::poly_fns,
            polygon: Some(polygon_t {
                peripheries: 1,
                sides: 4,
                ..polygon_t::default()
            }),
            usershape: false,
        });
        shapes.push(shape_desc {
            name: "ellipse",
            fns: shape_functions::poly_fns,
            polygon: Some(polygon_t {
                peripheries: 1,
                sides: 1,
                ..polygon_t::default()
            }),
            usershape: false,
        });
        shapes.push(shape_desc {
            name: "record",
            fns: shape_functions::record_fns,
            polygon: None,
            usershape: false,
        });
        Self {
            graphs: Arena::default(),
            nodes: Arena::default(),
            edges: Vec::new(),
            subnodes: Arena::default(),
            syms: Arena::default(),
            refstrs: Arena::default(),
            closes: Arena::default(),
            attr_dicts: Dicts::default(),
            cstring_uid: 100,
            ctr: 1,
            all: HashMap::new(),
            ProtoGraph: None,
            node_lists: CArrays::default(),
            edge_lists: CArrays::default(),
            graph_lists: CArrays::default(),
            field_lists: CArrays::default(),
            ranks: CArrays::default(),
            pointfs: CArrays::default(),
            beziers: CArrays::default(),
            textspans: CArrays::default(),
            textlabels: Arena::default(),
            splines: Arena::default(),
            polygons: Arena::default(),
            fields: Arena::default(),
            adjmatrices: Arena::default(),
            Shapes: shapes,
            G_ordering: None,
            G_margin: None,
            N_height: None,
            N_width: None,
            N_shape: None,
            N_fontsize: None,
            N_fontname: None,
            N_fontcolor: None,
            N_label: None,
            N_xlabel: None,
            N_nojustify: None,

            N_peripheries: None,
            N_ordering: None,
            N_orientation: None,
            N_fixed: None,
            N_group: None,
            E_weight: None,
            E_minlen: None,
            E_fontsize: None,
            E_fontname: None,
            E_fontcolor: None,
            E_label: None,
            E_xlabel: None,
            E_dir: None,

            E_arrowsz: None,
            E_constr: None,
            E_label_float: None,
            E_samehead: None,
            E_sametail: None,
            E_arrowhead: None,
            E_arrowtail: None,
            E_headlabel: None,
            E_taillabel: None,
            E_labelfontsize: None,
            E_labelfontname: None,
            E_labelfontcolor: None,
            E_labeldistance: None,
            E_labelangle: None,
            E_tailclip: None,
            E_headclip: None,
            CL_type: 0,
            Concentrate: false,
            MaxIter: 0,
            State: 0,
            EdgeLabelsDone: 0,
            N_nodes: 0,
            Minrank: 0,
            Maxrank: 0,
            S_i: 0,
            Search_size: 0,
            Tree_node: nlist_t::default(),
            Tree_edge: elist::default(),
            Enter: None,
            Low: 0,
            Lim: 0,
            Slack: 0,
            Rankdir: 0,
            Flip: false,
            Offset: pointf::default(),
            MinQuit: 0,
            Convergence: 0.0,
            Root: None,
            GlobalMinRank: 0,
            GlobalMaxRank: 0,
            ReMincross: false,
            TI_list: Vec::new(),
            Last_node_decomp: None,
            Cmark: 0,
            Count: Vec::new(),
            C: 0,
            G_ns: None,
            G_decomp: None,
            skippedConstraintLabelVnodes: Vec::new(),
            lastn: None,
            last: 0,
            outp: 0,
            sides: 0,
            vertex: None,
            xsize: 0.0,
            ysize: 0.0,
            scalex: 0.0,
            scaley: 0.0,
            box_URx: 0.0,
            box_URy: 0.0,
            pathplan: PathplanContext::default(),
        }
    }

    /// `GD_*(g)`: the dot record of a graph.
    pub fn gd(&self, g: GraphId) -> &Agraphinfo_t {
        &self.graphs[g].info
    }

    pub fn gd_mut(&mut self, g: GraphId) -> &mut Agraphinfo_t {
        &mut self.graphs[g].info
    }

    /// `ND_*(n)`: the dot record of a node.
    pub fn nd(&self, n: NodeId) -> &Agnodeinfo_t {
        &self.nodes[n].info
    }

    pub fn nd_mut(&mut self, n: NodeId) -> &mut Agnodeinfo_t {
        &mut self.nodes[n].info
    }

    /// `ED_*(e)`: the dot record of an edge, shared by both halves.
    pub fn ed(&self, e: EdgeId) -> &Agedgeinfo_t {
        &self.edges[e.pair()].info
    }

    pub fn ed_mut(&mut self, e: EdgeId) -> &mut Agedgeinfo_t {
        &mut self.edges[e.pair()].info
    }

    /// `GD_rank(g)[r]`.
    pub fn rank(&self, g: GraphId, r: i32) -> &rank_t {
        &self.ranks[self.gd(g).rank.expect("GD_rank is NULL").at(r)]
    }

    pub fn rank_mut(&mut self, g: GraphId, r: i32) -> &mut rank_t {
        let rank = self.gd(g).rank.expect("GD_rank is NULL");
        &mut self.ranks[rank.at(r)]
    }

    /// One half of an edge.
    pub fn edge(&self, e: EdgeId) -> &Agedge_s {
        self.edges[e.pair()].half(e)
    }

    pub fn edge_mut(&mut self, e: EdgeId) -> &mut Agedge_s {
        self.edges[e.pair()].half_mut(e)
    }

    pub(crate) fn edgepair(&self, e: EdgeId) -> &Agedgepair_s {
        &self.edges[e.pair()]
    }

    pub(crate) fn edgepair_mut(&mut self, e: EdgeId) -> &mut Agedgepair_s {
        &mut self.edges[e.pair()]
    }

    /// `new ST_Agedgepair_s()`: a pair with blank tags and no nodes. Returns its out-half.
    pub fn new_agedgepair(&mut self) -> EdgeId {
        self.edges.push(Agedgepair_s::default());
        EdgeId::out_of_pair(self.edges.len() - 1)
    }

    /// `new ST_Agnode_s()` in graph `root`, with a blank tag (dot's virtual nodes).
    pub fn new_agnode(&mut self, root: GraphId) -> NodeId {
        let mainsub = self.subnodes.push(Agsubnode_s {
            node: NodeId::from_index(self.nodes.len()),
            in_id: None,
            out_id: None,
            in_seq: None,
            out_seq: None,
        });
        self.nodes.push(Agnode_s {
            tag: crate::h::cgraph::Agtag_s::default(),
            root,
            mainsub,
            recs: 0,
            attr: None,
            info: Agnodeinfo_t::default(),
        })
    }

    /// The text of an interned string.
    pub fn agstr(&self, s: StrId) -> &str {
        &self.refstrs[s].s
    }

    /// Hands out the next `CString` uid.
    pub(crate) fn next_cstring_uid(&mut self) -> i32 {
        let uid = self.cstring_uid;
        self.cstring_uid += 2;
        uid
    }
}

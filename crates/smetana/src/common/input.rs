//! `common/input.c`: a graph's layout parameters (`graph_init`) and the labels of graphs and clusters.

use crate::cgraph::attr::{agattr, agfindedgeattr, agfindgraphattr, agfindnodeattr, agget};
use crate::cgraph::obj::agroot;
use crate::cgraph::refstr::aghtmlstr;
use crate::cgraph::{AGNODE, AGRAPH};
use crate::common::labels::make_label;
use crate::common::shapes::PAD;
use crate::common::utils::{
    agget_text, late_double, late_nnstring, late_string, mapbool, maptoken,
};
use crate::core::Globals;
use crate::core::consts::{
    BOTTOM_IX, DEFAULT_NODESEP, DEFAULT_RANKSEP, GLOBAL, GRAPH_LABEL, LABEL_AT_BOTTOM,
    LABEL_AT_TOP, LEFT_IX, LOCAL, LT_HTML, LT_NONE, MIN_NODESEP, MIN_RANKSEP, NOCLUST,
    NODENAME_ESC, RANKDIR_LR, RANKDIR_TB, RIGHT_IX, TOP_IX,
};
use crate::core::ids::GraphId;
use crate::core::jmath::POINTS;
use crate::core::jutils::atof;
use crate::h::layout_t;

/// `GVBEGIN`: the initial job state.
const GVBEGIN: i32 = 0;

/// Panics for graph attributes Smetana does not implement, where Java throws.
fn unsupported_graph_attr(zz: &mut Globals, g: GraphId, name: &str, if_empty: bool) {
    if agget_text(zz, g, name).is_some_and(|p| if_empty || !p.is_empty()) {
        unimplemented!("graph attribute {name}");
    }
}

/// `graph_init`: reads the graph attributes every layout uses and looks up the attributes of nodes and edges.
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
pub fn graph_init(zz: &mut Globals, g: GraphId, use_rankdir: bool) {
    zz.gd_mut(g).drawing = Some(layout_t::default());
    unsupported_graph_attr(zz, g, "fontpath", true);

    // findCharset: Smetana only knows UTF-8.
    zz.gd_mut(g).charset = 0;
    let quantum_sym = agfindgraphattr(zz, g, "quantum");
    let quantum = late_double(zz, g, quantum_sym, 0.0, 0.0);
    drawing(zz, g).quantum = quantum;

    // rankdir=LR is only defined in dot, so other layouts ignore it unless asked. The effective rankdir is in
    // the low two bits, the requested one in the next two.
    let mut rankdir = RANKDIR_TB;
    if agget_text(zz, g, "rankdir").as_deref() == Some("LR") {
        rankdir = RANKDIR_LR;
    }
    zz.gd_mut(g).rankdir = if use_rankdir {
        (rankdir << 2) | rankdir
    } else {
        rankdir << 2
    };

    let nodesep_sym = agfindgraphattr(zz, g, "nodesep");
    let xf = late_double(zz, g, nodesep_sym, DEFAULT_NODESEP, MIN_NODESEP);
    zz.gd_mut(g).nodesep = POINTS(xf);

    let ranksep_sym = agfindgraphattr(zz, g, "ranksep");
    let xf = match late_string(zz, g, ranksep_sym, None) {
        Some(p) => {
            let xf = atof(&p);
            if p == "equally" {
                zz.gd_mut(g).exact_ranksep = 1;
            }
            if xf < MIN_RANKSEP { MIN_RANKSEP } else { xf }
        }
        None => DEFAULT_RANKSEP,
    };
    zz.gd_mut(g).ranksep = POINTS(xf);

    // setRatio, and the size and page of getdoubles2ptf.
    unsupported_graph_attr(zz, g, "ratio", false);
    unsupported_graph_attr(zz, g, "size", true);
    unsupported_graph_attr(zz, g, "page", true);

    let p = agget_text(zz, g, "clusterrank");
    zz.CL_type = maptoken(
        p.as_deref(),
        &["local", "global", "none"],
        &[LOCAL, GLOBAL, NOCLUST, LOCAL],
    );
    zz.Concentrate = mapbool(agget_text(zz, g, "concentrate").as_deref());
    zz.State = GVBEGIN;
    zz.EdgeLabelsDone = 0;

    do_graph_label(zz, g);

    zz.G_ordering = agfindgraphattr(zz, g, "ordering");
    zz.G_margin = agfindgraphattr(zz, g, "margin");

    // The attributes of nodes.
    zz.N_height = agfindnodeattr(zz, g, "height");
    zz.N_width = agfindnodeattr(zz, g, "width");
    zz.N_shape = agfindnodeattr(zz, g, "shape");
    zz.N_fontsize = agfindnodeattr(zz, g, "fontsize");
    zz.N_fontname = agfindnodeattr(zz, g, "fontname");
    zz.N_fontcolor = agfindnodeattr(zz, g, "fontcolor");
    zz.N_label = agfindnodeattr(zz, g, "label");
    if zz.N_label.is_none() {
        zz.N_label = agattr(zz, Some(g), AGNODE, "label", Some(NODENAME_ESC));
    }
    zz.N_xlabel = agfindnodeattr(zz, g, "xlabel");

    zz.N_ordering = agfindnodeattr(zz, g, "ordering");

    // The attributes of polygon shapes.
    zz.N_peripheries = agfindnodeattr(zz, g, "peripheries");
    zz.N_orientation = agfindnodeattr(zz, g, "orientation");
    zz.N_fixed = agfindnodeattr(zz, g, "fixedsize");
    zz.N_nojustify = agfindnodeattr(zz, g, "nojustify");
    zz.N_group = agfindnodeattr(zz, g, "group");

    // The attributes of edges.
    zz.E_weight = agfindedgeattr(zz, g, "weight");
    zz.E_fontsize = agfindedgeattr(zz, g, "fontsize");
    zz.E_fontname = agfindedgeattr(zz, g, "fontname");
    zz.E_fontcolor = agfindedgeattr(zz, g, "fontcolor");
    zz.E_label = agfindedgeattr(zz, g, "label");
    zz.E_xlabel = agfindedgeattr(zz, g, "xlabel");
    zz.E_label_float = agfindedgeattr(zz, g, "labelfloat");
    zz.E_dir = agfindedgeattr(zz, g, "dir");
    zz.E_arrowhead = agfindedgeattr(zz, g, "arrowhead");
    zz.E_arrowtail = agfindedgeattr(zz, g, "arrowtail");
    zz.E_headlabel = agfindedgeattr(zz, g, "headlabel");
    zz.E_taillabel = agfindedgeattr(zz, g, "taillabel");
    zz.E_labelfontsize = agfindedgeattr(zz, g, "labelfontsize");
    zz.E_labelfontname = agfindedgeattr(zz, g, "labelfontname");
    zz.E_labelfontcolor = agfindedgeattr(zz, g, "labelfontcolor");
    zz.E_labeldistance = agfindedgeattr(zz, g, "labeldistance");
    zz.E_labelangle = agfindedgeattr(zz, g, "labelangle");
    zz.E_minlen = agfindedgeattr(zz, g, "minlen");

    zz.E_arrowsz = agfindedgeattr(zz, g, "arrowsize");
    zz.E_constr = agfindedgeattr(zz, g, "constraint");
    zz.E_tailclip = agfindedgeattr(zz, g, "tailclip");
    zz.E_headclip = agfindedgeattr(zz, g, "headclip");

    // init_xdot: Smetana draws no background.
    unsupported_graph_attr(zz, g, "_background", false);
    unsupported_graph_attr(zz, g, "_draw_", false);
    unsupported_graph_attr(zz, g, "id", false);
}

fn drawing(zz: &mut Globals, g: GraphId) -> &mut layout_t {
    zz.gd_mut(g).drawing.as_mut().expect("GD_drawing")
}

/// `do_graph_label`: the label of a graph or cluster, and the room a cluster keeps for it.
pub(crate) fn do_graph_label(zz: &mut Globals, sg: GraphId) {
    let Some(str) = agget(zz, sg, "label").filter(|&s| !zz.agstr(s).is_empty()) else {
        return;
    };
    let root = zz.graphs[sg].root;
    zz.gd_mut(root).has_labels |= GRAPH_LABEL;

    let kind = if aghtmlstr(zz, str) != 0 {
        LT_HTML
    } else {
        LT_NONE
    };
    let fontsize_sym = agattr(zz, Some(sg), AGRAPH, "fontsize", None);
    let fontsize = late_double(zz, sg, fontsize_sym, 14.0, 1.0);
    let fontname_sym = agattr(zz, Some(sg), AGRAPH, "fontname", None);
    let fontname = late_nnstring(zz, sg, fontname_sym, "Times-Roman");
    let fontcolor_sym = agattr(zz, Some(sg), AGRAPH, "fontcolor", None);
    let fontcolor = late_nnstring(zz, sg, fontcolor_sym, "black");
    let text = zz.agstr(str).to_owned();
    let label = make_label(zz, sg.into(), &text, kind, fontsize, &fontname, &fontcolor);
    zz.gd_mut(sg).label = Some(label);

    // The label's position.
    let pos = agget_text(zz, sg, "labelloc");
    if sg == agroot(zz, sg) {
        unimplemented!("root graph labels");
    }
    let pos_flag = if pos.is_some_and(|p| p.starts_with('b')) {
        LABEL_AT_BOTTOM
    } else {
        LABEL_AT_TOP
    };
    if agget(zz, sg, "labeljust").is_some() {
        unimplemented!("labeljust");
    }
    zz.gd_mut(sg).label_pos = pos_flag;

    // A cluster's border keeps room for its label.
    let mut dimen = zz.textlabels[label].dimen;
    PAD(&mut dimen);
    let label_at_top = (zz.gd(sg).label_pos & LABEL_AT_TOP) != 0;
    if zz.gd(root).GD_flip() {
        // When rotated, the labels will be restored to TOP or BOTTOM.
        let pos_ix = if label_at_top { RIGHT_IX } else { LEFT_IX };
        let border = &mut zz.gd_mut(sg).border[pos_ix as usize];
        border.x = dimen.y;
        border.y = dimen.x;
    } else {
        let pos_ix = if label_at_top { TOP_IX } else { BOTTOM_IX };
        zz.gd_mut(sg).border[pos_ix as usize] = dimen;
    }
}

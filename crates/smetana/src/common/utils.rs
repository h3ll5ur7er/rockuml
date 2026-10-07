//! `common/utils.c`: node queues, attribute defaults (`late_*`), union-find, keyword maps and the parts of node
//! and edge initialisation that all layouts share.

use crate::cgraph::attr::{agget, agxget};
use crate::cgraph::obj::agraphof;
use crate::cgraph::refstr::aghtmlstr;
use crate::cgraph::{Agobj, aghead, agtail};
use crate::common::labels::make_label;
use crate::common::shapes::{EN_shape_kind, bind_shape, initfn, portfn, shapeOf};
use crate::core::Globals;
use crate::core::consts::{
    DEFAULT_FONTSIZE, DEFAULT_NODEHEIGHT, DEFAULT_NODESHAPE, DEFAULT_NODEWIDTH, EDGE_LABEL,
    ET_NONE, HEAD_LABEL, LT_HTML, LT_NONE, LT_RECD, MIN_FONTSIZE, MIN_NODEHEIGHT, MIN_NODEWIDTH,
    NORMAL, TAIL_LABEL,
};
use crate::core::ids::{EdgeId, GraphId, NodeId, SymId, TextlabelId};
use crate::core::jmath::INCH2PS;
use crate::core::jutils::{atof, atoi, strcmp};
use crate::h::{boxf, pointf, port, splines, textlabel_t};

/// `HEAD_ID` and `TAIL_ID`: the attributes naming an edge's ports.
const HEAD_ID: &str = "headport";
const TAIL_ID: &str = "tailport";

/// `nodequeue`: a ring buffer of nodes. It holds `sz` nodes but cannot tell a full buffer from an empty one,
/// exactly like Graphviz's.
pub(crate) struct nodequeue {
    store: Vec<Option<NodeId>>,
    head: usize,
    tail: usize,
}

/// `new_queue`.
pub(crate) fn new_queue(sz: i32) -> nodequeue {
    let sz = if sz <= 1 { 2 } else { sz as usize };
    nodequeue {
        store: vec![None; sz],
        head: 0,
        tail: 0,
    }
}

/// `enqueue`.
pub(crate) fn enqueue(q: &mut nodequeue, n: NodeId) {
    q.store[q.tail] = Some(n);
    q.tail += 1;
    if q.tail >= q.store.len() {
        q.tail = 0;
    }
}

/// `dequeue`.
pub(crate) fn dequeue(q: &mut nodequeue) -> Option<NodeId> {
    if q.head == q.tail {
        return None;
    }
    let n = q.store[q.head];
    q.head += 1;
    if q.head >= q.store.len() {
        q.head = 0;
    }
    n
}

/// `agxget` as text.
pub(crate) fn agxget_text(zz: &mut Globals, obj: impl Into<Agobj>, sym: SymId) -> Option<String> {
    agxget(zz, obj, sym).map(|s| zz.agstr(s).to_owned())
}

/// `agget` as text.
pub(crate) fn agget_text(zz: &mut Globals, obj: impl Into<Agobj>, name: &str) -> Option<String> {
    agget(zz, obj, name).map(|s| zz.agstr(s).to_owned())
}

/// `late_int`: the integer value of `attr` on `obj`, `def` if unset or empty, at least `low`.
pub fn late_int(
    zz: &mut Globals,
    obj: impl Into<Agobj>,
    attr: Option<SymId>,
    def: i32,
    low: i32,
) -> i32 {
    let Some(attr) = attr else { return def };
    let Some(p) = agxget_text(zz, obj, attr).filter(|p| !p.is_empty()) else {
        return def;
    };
    let rv = atoi(&p);
    if rv < low { low } else { rv }
}

/// `late_double`: the floating-point value of `attr` on `obj`, `def` if unset or empty, at least `low`.
pub fn late_double(
    zz: &mut Globals,
    obj: impl Into<Agobj>,
    attr: Option<SymId>,
    def: f64,
    low: f64,
) -> f64 {
    let Some(attr) = attr else { return def };
    let Some(p) = agxget_text(zz, obj, attr).filter(|p| !p.is_empty()) else {
        return def;
    };
    let rv = atof(&p);
    if rv < low { low } else { rv }
}

/// `late_string`: the value of `attr` on `obj`, `def` if the attribute is not declared.
pub fn late_string(
    zz: &mut Globals,
    obj: impl Into<Agobj>,
    attr: Option<SymId>,
    def: Option<&str>,
) -> Option<String> {
    match attr {
        None => def.map(str::to_owned),
        Some(attr) => agxget_text(zz, obj, attr),
    }
}

/// `late_nnstring`: like [`late_string`], with `def` also replacing an empty value.
pub fn late_nnstring(
    zz: &mut Globals,
    obj: impl Into<Agobj>,
    attr: Option<SymId>,
    def: &str,
) -> String {
    late_string(zz, obj, attr, Some(def))
        .filter(|rv| !rv.is_empty())
        .unwrap_or_else(|| def.to_owned())
}

/// `UF_find`: the representative of `n`'s set, halving the path on the way.
pub fn UF_find(zz: &mut Globals, mut n: NodeId) -> NodeId {
    while let Some(parent) = zz.nd(n).UF_parent
        && parent != n
    {
        if let Some(grandparent) = zz.nd(parent).UF_parent {
            zz.nd_mut(n).UF_parent = Some(grandparent);
        }
        n = zz.nd(n).UF_parent.expect("UF parent");
    }
    n
}

/// `UF_union`: merges the sets of `u` and `v` and returns the new representative.
pub fn UF_union(zz: &mut Globals, u: NodeId, mut v: NodeId) -> NodeId {
    if u == v {
        return u;
    }
    if zz.nd(u).UF_parent.is_none() {
        zz.nd_mut(u).UF_parent = Some(u);
        zz.nd_mut(u).UF_size = 1;
    } else {
        unimplemented!("UF_union of a non-singleton u");
    }
    if zz.nd(v).UF_parent.is_none() {
        zz.nd_mut(v).UF_parent = Some(v);
        zz.nd_mut(v).UF_size = 1;
    } else {
        v = UF_find(zz, v);
    }
    if zz.nd(u).id > zz.nd(v).id {
        unimplemented!("UF_union with ND_id(u) > ND_id(v)");
    }
    zz.nd_mut(v).UF_parent = Some(u);
    zz.nd_mut(u).UF_size += zz.nd(v).UF_size;
    u
}

/// `UF_singleton`.
pub fn UF_singleton(zz: &mut Globals, u: NodeId) {
    let info = zz.nd_mut(u);
    info.UF_size = 1;
    info.UF_parent = None;
    info.ranktype = NORMAL;
}

/// `UF_setname`: makes `v` the parent of the representative `u`.
pub fn UF_setname(zz: &mut Globals, u: NodeId, v: NodeId) {
    zz.nd_mut(u).UF_parent = Some(v);
    zz.nd_mut(v).UF_size += zz.nd(u).UF_size;
}

/// `maptoken`: the value of the first name equal to `p`, or the last value (one past the names) otherwise.
pub fn maptoken(p: Option<&str>, name: &[&str], val: &[i32]) -> i32 {
    let i = name
        .iter()
        .position(|q| p.is_some_and(|p| strcmp(p, q) == 0))
        .unwrap_or(name.len());
    val[i]
}

/// `strcasecmp(a, b) == 0`, with Java's `Character.toLowerCase`.
fn strcaseeq(a: &str, b: &str) -> bool {
    let lower = |s: &str| -> Vec<char> {
        s.chars()
            .map(|c| c.to_lowercase().next().unwrap_or(c))
            .collect()
    };
    lower(a) == lower(b)
}

/// `mapBool`.
pub fn mapBool(p: Option<&str>, dflt: bool) -> bool {
    let Some(p) = p.filter(|p| !p.is_empty()) else {
        return dflt;
    };
    if strcaseeq(p, "false") || strcaseeq(p, "no") {
        false
    } else if strcaseeq(p, "true") || strcaseeq(p, "yes") {
        true
    } else if p.starts_with(|c: char| c.is_ascii_digit()) {
        atoi(p) != 0
    } else {
        dflt
    }
}

/// `mapbool`.
pub fn mapbool(p: Option<&str>) -> bool {
    mapBool(p, false)
}

/// `common_init_node`: size, shape and label of a node, then the shape's own initialisation.
pub fn common_init_node(zz: &mut Globals, n: NodeId) {
    let width = late_double(zz, n, zz.N_width, DEFAULT_NODEWIDTH, MIN_NODEWIDTH);
    zz.nd_mut(n).width = width;
    let height = late_double(zz, n, zz.N_height, DEFAULT_NODEHEIGHT, MIN_NODEHEIGHT);
    zz.nd_mut(n).height = height;
    let shape_name = late_nnstring(zz, n, zz.N_shape, DEFAULT_NODESHAPE);
    let shape = bind_shape(zz, &shape_name);
    zz.nd_mut(n).shape = Some(shape);
    let label_sym = zz.N_label.expect("N_label");
    let str = agxget(zz, n, label_sym).expect("node label");
    let fontsize = late_double(zz, n, zz.N_fontsize, DEFAULT_FONTSIZE, MIN_FONTSIZE);
    let fontname = late_nnstring(zz, n, zz.N_fontname, "Times-Roman");
    let fontcolor = late_nnstring(zz, n, zz.N_fontcolor, "black");
    let kind = (if aghtmlstr(zz, str) != 0 {
        LT_HTML
    } else {
        LT_NONE
    }) | (if shapeOf(zz, n) == EN_shape_kind::SH_RECORD {
        LT_RECD
    } else {
        LT_NONE
    });
    let text = zz.agstr(str).to_owned();
    let label = make_label(zz, n.into(), &text, kind, fontsize, &fontname, &fontcolor);
    zz.nd_mut(n).label = Some(label);
    if let Some(xlabel) = zz.N_xlabel
        && agxget_text(zz, n, xlabel).is_some_and(|s| !s.is_empty())
    {
        unimplemented!("node xlabel");
    }
    let showboxes = late_int(zz, n, zz.N_showboxes, 0, 0);
    zz.nd_mut(n).showboxes = showboxes;
    initfn(zz, n);
}

/// `fontinfo`.
#[derive(Default)]
struct fontinfo {
    fontsize: f64,
    fontname: Option<String>,
    fontcolor: Option<String>,
}

/// `initFontEdgeAttr`.
fn initFontEdgeAttr(zz: &mut Globals, e: EdgeId, fi: &mut fontinfo) {
    fi.fontsize = late_double(zz, e, zz.E_fontsize, DEFAULT_FONTSIZE, MIN_FONTSIZE);
    fi.fontname = Some(late_nnstring(zz, e, zz.E_fontname, "Times-Roman"));
    fi.fontcolor = Some(late_nnstring(zz, e, zz.E_fontcolor, "black"));
}

/// `initFontLabelEdgeAttr`.
fn initFontLabelEdgeAttr(zz: &mut Globals, e: EdgeId, fi: &mut fontinfo, lfi: &mut fontinfo) {
    if fi.fontname.is_none() {
        initFontEdgeAttr(zz, e, fi);
    }
    lfi.fontsize = late_double(zz, e, zz.E_labelfontsize, fi.fontsize, MIN_FONTSIZE);
    let fontname = fi.fontname.as_deref().expect("font name");
    lfi.fontname = Some(late_nnstring(zz, e, zz.E_labelfontname, fontname));
    let fontcolor = fi.fontcolor.as_deref().expect("font color");
    lfi.fontcolor = Some(late_nnstring(zz, e, zz.E_labelfontcolor, fontcolor));
}

/// `noClip`: whether an edge end must not be clipped to its node.
fn noClip(zz: &mut Globals, e: EdgeId, sym: Option<SymId>) -> bool {
    // mapbool isn't a good fit, because "" must mean true.
    sym.and_then(|sym| agxget_text(zz, e, sym))
        .is_some_and(|str| !str.is_empty() && !mapbool(Some(&str)))
}

/// `chkPort`: resolves a port name with the node's shape. `name` is the port attribute (`None` when the
/// attribute is not declared, which C treats as "").
fn chkPort(zz: &mut Globals, n: NodeId, name: Option<crate::core::ids::StrId>) -> port {
    let s = name.map_or_else(String::new, |s| zz.agstr(s).to_owned());
    if s.contains(':') {
        unimplemented!("ports with compass points");
    }
    let mut pt = portfn(zz, n, &s, None);
    pt.name = name;
    pt
}

/// The label of an edge built from attribute `sym`, if it has a non-empty value.
fn edge_label_text(zz: &mut Globals, e: EdgeId, sym: Option<SymId>) -> Option<String> {
    sym.and_then(|sym| agxget_text(zz, e, sym))
        .filter(|s| !s.is_empty())
}

/// `common_init_edge`: an edge's labels and ports. Returns whether it has a label.
pub fn common_init_edge(zz: &mut Globals, e: EdgeId) -> i32 {
    let mut r = 0;
    let mut fi = fontinfo::default();
    let mut lfi = fontinfo::default();
    let sg = agraphof(zz, agtail(zz, e));

    if let Some(str) = edge_label_text(zz, e, zz.E_label) {
        r = 1;
        initFontEdgeAttr(zz, e, &mut fi);
        let kind = edge_label_kind(zz, e, zz.E_label);
        let label = make_label(
            zz,
            e.into(),
            &str,
            kind,
            fi.fontsize,
            fi.fontname.as_deref().expect("font name"),
            fi.fontcolor.as_deref().expect("font color"),
        );
        zz.ed_mut(e).label = Some(label);
        zz.gd_mut(sg).has_labels |= EDGE_LABEL;
        let ontop = late_string(zz, e, zz.E_label_float, Some("false"));
        zz.ed_mut(e).label_ontop = mapbool(ontop.as_deref());
    }

    if edge_label_text(zz, e, zz.E_xlabel).is_some() {
        unimplemented!("edge xlabel");
    }

    if let Some(str) = edge_label_text(zz, e, zz.E_headlabel) {
        initFontLabelEdgeAttr(zz, e, &mut fi, &mut lfi);
        let kind = edge_label_kind(zz, e, zz.E_headlabel);
        let label = make_label(
            zz,
            e.into(),
            &str,
            kind,
            lfi.fontsize,
            lfi.fontname.as_deref().expect("font name"),
            lfi.fontcolor.as_deref().expect("font color"),
        );
        zz.ed_mut(e).head_label = Some(label);
        zz.gd_mut(sg).has_labels |= HEAD_LABEL;
    }
    if let Some(str) = edge_label_text(zz, e, zz.E_taillabel) {
        initFontLabelEdgeAttr(zz, e, &mut fi, &mut lfi);
        let kind = edge_label_kind(zz, e, zz.E_taillabel);
        let label = make_label(
            zz,
            e.into(),
            &str,
            kind,
            lfi.fontsize,
            lfi.fontname.as_deref().expect("font name"),
            lfi.fontcolor.as_deref().expect("font color"),
        );
        zz.ed_mut(e).tail_label = Some(label);
        zz.gd_mut(sg).has_labels |= TAIL_LABEL;
    }

    // Ports beginning with a colon (tailport=":abc") are still accepted, but deprecated.
    let str = agget(zz, e, TAIL_ID);
    let tail = agtail(zz, e);
    if str.is_some_and(|s| !zz.agstr(s).is_empty()) {
        zz.nd_mut(tail).has_port = true;
    }
    let tail_port = chkPort(zz, tail, str);
    zz.ed_mut(e).tail_port = tail_port;
    if noClip(zz, e, zz.E_tailclip) {
        unimplemented!("tailclip=false");
    }
    let str = agget(zz, e, HEAD_ID);
    let head = aghead(zz, e);
    if str.is_some_and(|s| !zz.agstr(s).is_empty()) {
        zz.nd_mut(head).has_port = true;
    }
    let head_port = chkPort(zz, head, str);
    zz.ed_mut(e).head_port = head_port;
    if noClip(zz, e, zz.E_headclip) {
        unimplemented!("headclip=false");
    }
    r
}

/// `aghtmlstr(str) ? LT_HTML : LT_NONE` for an edge label attribute.
fn edge_label_kind(zz: &mut Globals, e: EdgeId, sym: Option<SymId>) -> i32 {
    let str = agxget(zz, e, sym.expect("label attribute")).expect("label value");
    if aghtmlstr(zz, str) != 0 {
        LT_HTML
    } else {
        LT_NONE
    }
}

/// `edgeType`: Smetana only supports graphs without a `splines` value.
fn edgeType(_s: &str, _dflt: i32) -> i32 {
    unimplemented!("splines attribute")
}

/// `setEdgeType`: the graph's edge routing type from its `splines` attribute, `dflt` if undeclared.
pub fn setEdgeType(zz: &mut Globals, g: GraphId, dflt: i32) {
    let s = agget_text(zz, g, "splines");
    let et = match s.as_deref() {
        None => dflt,
        Some("") => ET_NONE,
        Some(s) => edgeType(s, dflt),
    };
    zz.gd_mut(g).flags |= et;
}

/// `gv_nodesize`: a node's half widths and height in points, from its size in inches.
pub fn gv_nodesize(zz: &mut Globals, n: NodeId, flip: bool) {
    let info = zz.nd_mut(n);
    if flip {
        let w = INCH2PS(info.height);
        info.rw = w / 2.0;
        info.lw = w / 2.0;
        info.ht = INCH2PS(info.width);
    } else {
        let w = INCH2PS(info.width);
        info.rw = w / 2.0;
        info.lw = w / 2.0;
        info.ht = INCH2PS(info.height);
    }
}

/// `updateBB` (`utils.c`): grows the graph's bounding box to contain the label.
pub(crate) fn updateBB(zz: &mut Globals, g: GraphId, lp: TextlabelId) {
    let bb = addLabelBB(zz.gd(g).bb, &zz.textlabels[lp], zz.gd(g).GD_flip());
    zz.gd_mut(g).bb = bb;
}

/// `addLabelBB` (`utils.c`).
pub(crate) fn addLabelBB(mut bb: boxf, lp: &textlabel_t, flipxy: bool) -> boxf {
    let p = lp.pos;
    let (width, height) = if flipxy {
        (lp.dimen.y, lp.dimen.x)
    } else {
        (lp.dimen.x, lp.dimen.y)
    };
    let min = p.x - width / 2.0;
    let max = p.x + width / 2.0;
    if min < bb.LL.x {
        bb.LL.x = min;
    }
    if max > bb.UR.x {
        bb.UR.x = max;
    }
    let min = p.y - height / 2.0;
    let max = p.y + height / 2.0;
    if min < bb.LL.y {
        bb.LL.y = min;
    }
    if max > bb.UR.y {
        bb.UR.y = max;
    }
    bb
}

/// `late_bool` (`utils.c`), for attributes PlantUML never sets.
pub(crate) fn late_bool(attr: Option<SymId>, def: i32) -> bool {
    if attr.is_none() {
        return def != 0;
    }
    unimplemented!("late_bool on a declared attribute")
}

/// `dotneato_closest` (`utils.c`): the point of the bezier segment nearest to `pt`, found by bisection. Smetana
/// only implements the first step, so it throws unless that step already decides.
pub(crate) fn dotneato_closest(zz: &Globals, spl: &splines, pt: pointf) -> pointf {
    let list = spl.list.expect("spline list");
    let mut besti = -1;
    let mut bestj = -1;
    let mut bestdist2 = 1e+38;
    for i in 0..spl.size {
        let bz = zz.beziers.get(list, i);
        for j in 0..bz.size {
            let b = zz.pointfs.get(bz.list.expect("bezier points"), j);
            let d2 = DIST2(b, pt);
            if bestj == -1 || d2 < bestdist2 {
                besti = i;
                bestj = j;
                bestdist2 = d2;
            }
        }
    }

    let bz = zz.beziers.get(list, besti);
    if bestj == bz.size - 1 {
        bestj -= 1;
    }
    let j = 3 * (bestj / 3);
    let points = bz.list.expect("bezier points");
    let c: [pointf; 4] = std::array::from_fn(|k| zz.pointfs.get(points, j + k as i32));
    let dlow2 = DIST2(c[0], pt);
    let dhigh2 = DIST2(c[3], pt);
    // The first step of the bisection over [0, 1].
    let pt2 = Bezier(&c, 0.5);
    if (dlow2 - dhigh2).abs() < 1.0 {
        return pt2;
    }
    unimplemented!("dotneato_closest beyond its first bisection step")
}

/// `Bezier` (`utils.c`) of degree 3, without the halves: the point at `t` by de Casteljau's algorithm.
pub(crate) fn Bezier(V: &[pointf; 4], t: f64) -> pointf {
    const W: usize = 5 + 1;
    let degree: usize = 3;
    let mut tx = [0.0; W * W];
    let mut ty = [0.0; W * W];
    for j in 0..=degree {
        tx[j] = V[j].x;
        ty[j] = V[j].y;
    }
    for i in 1..=degree {
        for j in 0..=degree - i {
            tx[i * W + j] = (1.0 - t) * tx[(i - 1) * W + j] + t * tx[(i - 1) * W + j + 1];
            ty[i * W + j] = (1.0 - t) * ty[(i - 1) * W + j] + t * ty[(i - 1) * W + j + 1];
        }
    }
    pointf {
        x: tx[degree * W],
        y: ty[degree * W],
    }
}

/// `DIST2`.
pub(crate) fn DIST2(p: pointf, q: pointf) -> f64 {
    let a = p.x - q.x;
    let b = p.y - q.y;
    a * a + b * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_queue_looks_empty() {
        let mut zz = Globals::open();
        let g = crate::cgraph::graph::agopen(&mut zz, Some("g"), crate::h::cgraph::Agdirected);
        let a = zz.new_agnode(g);
        let b = zz.new_agnode(g);
        let mut q = new_queue(2);
        enqueue(&mut q, a);
        assert_eq!(dequeue(&mut q), Some(a));
        enqueue(&mut q, a);
        enqueue(&mut q, b);
        assert_eq!(dequeue(&mut q), None);
    }

    #[test]
    fn maptoken_falls_back_to_the_last_value() {
        let names = ["local", "global", "none"];
        let codes = [100, 101, 102, 100];
        assert_eq!(maptoken(Some("global"), &names, &codes), 101);
        assert_eq!(maptoken(Some("other"), &names, &codes), 100);
        assert_eq!(maptoken(None, &names, &codes), 100);
    }

    #[test]
    fn mapbool_reads_words_and_numbers() {
        assert!(mapbool(Some("TRUE")));
        assert!(mapbool(Some("yes")));
        assert!(!mapbool(Some("No")));
        assert!(mapbool(Some("2")));
        assert!(!mapbool(Some("0")));
        assert!(!mapbool(Some("maybe")));
        assert!(mapBool(None, true));
    }
}

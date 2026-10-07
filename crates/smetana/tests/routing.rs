//! Replays the calls Smetana made to the spline routing building blocks while laying out the random graphs and the
//! corpus cases (data/routing-*.txt, recorded by tools/oracle/smetana-unit/routing.sh) and requires bit-identical
//! results: `clip_and_install` (with shape and arrow clipping), `beginpath`/`endpath`, `routesplines`,
//! `simpleSplineRoute`, `makeSelfEdge` and the shapes' port functions.
//!
//! Each call comes with the state of the nodes and edges it reads, where that state differs from what the replay
//! already has; the replay rebuilds those objects, makes the call on one context per layout (so that caches and
//! the path planner's scratch arrays carry over as in Java), and compares the results and the objects' state.

#![allow(non_snake_case, clippy::similar_names, reason = "Graphviz's names")]

use std::collections::HashMap;
use std::fmt::Write as _;
use std::panic::{self, AssertUnwindSafe};

use smetana::cgraph::attr::{agattr, agxget, agxset};
use smetana::cgraph::edge::agedge;
use smetana::cgraph::graph::agopen;
use smetana::cgraph::node::agnode;
use smetana::cgraph::{
    AGEDGE, AGINEDGE, AGMKOUT, AGOPP, AGOUTEDGE, M_aghead, M_agtail, aghead, agtail,
};
use smetana::common::routespl::{routepolylines, routesplines, simpleSplineRoute};
use smetana::common::shapes::{bind_shape, portfn};
use smetana::common::splines::{beginpath, clip_and_install, endpath, makeSelfEdge};
use smetana::core::Globals;
use smetana::core::ids::{EdgeId, FieldId, GraphId, NodeId, SymId};
use smetana::dotgen::dotsplines::{spline_merge, swap_ends_p};
use smetana::h::cgraph::Agdirected;
use smetana::h::{
    SHAPE_INFO, bezier, boxf, field_t, path, pathend_t, pointf, polygon_t, port, splineInfo,
    splines, textlabel_t,
};

const FIXTURES: [(&str, &str); 3] = [
    (
        "routing-random.txt",
        include_str!("data/routing-random.txt"),
    ),
    (
        "routing-corpus.txt",
        include_str!("data/routing-corpus.txt"),
    ),
    (
        "routing-synthetic.txt",
        include_str!("data/routing-synthetic.txt"),
    ),
];

/// The attributes `arrow_flags` and `arrow_length` read, in the fixture's order.
const SYMS: [&str; 4] = ["dir", "arrowhead", "arrowtail", "arrowsize"];

// ---------------------------------------------------------------- tokens

struct Tokens<'a> {
    t: Vec<&'a str>,
    i: usize,
}

impl<'a> Tokens<'a> {
    fn new(line: &'a str) -> Self {
        Self {
            t: line.split(' ').collect(),
            i: 0,
        }
    }

    fn next(&mut self) -> &'a str {
        let t = self.t[self.i];
        self.i += 1;
        t
    }

    fn expect(&mut self, word: &str) {
        let t = self.next();
        assert_eq!(t, word, "fixture token");
    }

    /// The value after the keyword `word`.
    fn after(&mut self, word: &str) -> &'a str {
        self.expect(word);
        self.next()
    }

    fn f(&mut self) -> f64 {
        double(self.next())
    }

    fn i(&mut self) -> i32 {
        self.next().parse().expect("integer")
    }

    fn b(&mut self) -> bool {
        self.i() != 0
    }

    fn pt(&mut self) -> pointf {
        pointf {
            x: self.f(),
            y: self.f(),
        }
    }

    fn bx(&mut self) -> boxf {
        boxf {
            LL: self.pt(),
            UR: self.pt(),
        }
    }

    fn points(&mut self) -> Vec<pointf> {
        let n = self.i();
        (0..n).map(|_| self.pt()).collect()
    }

    fn port(&mut self) -> port {
        let p = self.pt();
        let theta = self.f();
        let bp = if self.t[self.i] == "-" {
            self.i += 1;
            None
        } else {
            Some(self.bx())
        };
        port {
            p,
            theta,
            bp,
            defined: self.b(),
            constrained: self.b(),
            clip: self.b(),
            dyna: self.b(),
            order: self.i(),
            side: self.i(),
            name: None,
        }
    }
}

/// Java's `Double.toString`, which Rust parses to the same bits, except for its names of the non-finite values.
fn double(s: &str) -> f64 {
    match s {
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        "NaN" => f64::NAN,
        _ => s.parse().unwrap_or_else(|_| panic!("number {s:?}")),
    }
}

fn unquote(s: &str) -> String {
    let inner = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .expect("quoted");
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => out.push(' '),
                Some(other) => out.push(other),
                None => panic!("dangling escape"),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace(' ', "\\s")
    )
}

/// Whether two lines say the same: token by token, numbers by their bits.
fn same(expected: &str, actual: &str, skip: &[usize]) -> bool {
    let (e, a): (Vec<&str>, Vec<&str>) =
        (expected.split(' ').collect(), actual.split(' ').collect());
    e.len() == a.len() && e.iter().zip(&a).enumerate().all(|(i, (e, a))| {
        skip.contains(&i)
            || e == a
            || matches!((number(e), number(a)), (Some(x), Some(y)) if x.to_bits() == y.to_bits())
    })
}

fn number(s: &str) -> Option<f64> {
    s.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == 'I' || c == 'N')
        .then(|| s.parse().ok().or_else(|| (s.len() > 1).then(|| double(s))))
        .flatten()
}

// ---------------------------------------------------------------- serialisation, as RoutingDump writes it

fn pt(p: pointf) -> String {
    format!("{:?} {:?}", p.x, p.y)
}

fn bx(b: boxf) -> String {
    format!("{} {}", pt(b.LL), pt(b.UR))
}

fn points(ps: &[pointf]) -> String {
    let mut s = ps.len().to_string();
    for p in ps {
        write!(s, " {}", pt(*p)).unwrap();
    }
    s
}

fn b(v: bool) -> u8 {
    u8::from(v)
}

fn port_text(p: &port) -> String {
    format!(
        "{} {:?} {} {} {} {} {} {} {}",
        pt(p.p),
        p.theta,
        p.bp.map_or_else(|| "-".to_string(), bx),
        b(p.defined),
        b(p.constrained),
        b(p.clip),
        b(p.dyna),
        p.order,
        p.side
    )
}

/// The end's boxes as Java writes them, with a placeholder for the node point `np`, which the port drops because
/// nothing reads it.
fn endp_text(endp: &pathend_t) -> String {
    format!(
        "{} _ _ {} {} {}",
        bx(endp.nb),
        endp.sidemask,
        endp.boxn,
        bx(endp.boxes[0])
    )
}

fn boxes_text(P: &path) -> String {
    let mut s = P.nbox.to_string();
    for b in &P.boxes[..P.nbox as usize] {
        write!(s, " {}", bx(*b)).unwrap();
    }
    s
}

fn bezier_text(zz: &Globals, bz: &bezier) -> String {
    let list = bz.list.expect("bezier points");
    let ps: Vec<pointf> = (0..bz.size).map(|i| zz.pointfs.get(list, i)).collect();
    format!(
        "bezier {} {} {} {} {}",
        bz.sflag,
        bz.eflag,
        pt(bz.sp),
        pt(bz.ep),
        points(&ps)
    )
}

// ---------------------------------------------------------------- the replayed layouts

fn sinfo(t: &mut Tokens) -> splineInfo {
    splineInfo {
        swapEnds: swap_ends_p,
        splineMerge: spline_merge,
        ignoreSwap: t.after("ignoreSwap") == "1",
        isOrtho: t.after("isOrtho") == "1",
    }
}

struct Layout {
    zz: Globals,
    root: GraphId,
    nodes: HashMap<usize, NodeId>,
    edges: HashMap<usize, EdgeId>,
    syms: [Option<SymId>; 4],
    shapes: HashMap<usize, String>,
    broken: bool,
}

impl Layout {
    fn new() -> Self {
        let mut zz = Globals::open();
        let root = agopen(&mut zz, Some("g"), Agdirected);
        Self {
            zz,
            root,
            nodes: HashMap::new(),
            edges: HashMap::new(),
            syms: [None; 4],
            shapes: HashMap::new(),
            broken: false,
        }
    }

    fn node(&self, i: &str) -> NodeId {
        self.nodes[&i.parse::<usize>().expect("node index")]
    }

    fn edge(&self, i: &str) -> EdgeId {
        self.edges[&i.parse::<usize>().expect("edge index")]
    }

    fn apply_syms(&mut self, line: &str) {
        let mut t = Tokens::new(line);
        t.expect("syms");
        for (k, name) in SYMS.iter().enumerate() {
            if t.after(name) == "1" {
                let sym = agattr(&mut self.zz, Some(self.root), AGEDGE, name, Some(""));
                self.syms[k] = sym;
            }
        }
        let zz = &mut self.zz;
        [zz.E_dir, zz.E_arrowhead, zz.E_arrowtail, zz.E_arrowsz] = self.syms;
    }

    fn apply_graph(&mut self, line: &str) {
        let mut t = Tokens::new(line);
        t.expect("graph");
        let gd = self.zz.gd_mut(self.root);
        gd.rankdir = t.after("rankdir").parse().expect("rankdir");
        gd.flags = t.after("flags").parse().expect("flags");
        t.expect("bb");
        gd.bb = t.bx();
    }

    fn field(&mut self, t: &mut Tokens) -> FieldId {
        let b = t.bx();
        let sides = t.i();
        let id = match t.next() {
            "-" => None,
            q => Some(unquote(q)),
        };
        let n_flds = t.i();
        let subfields: Vec<Option<FieldId>> = (0..n_flds).map(|_| Some(self.field(t))).collect();
        let fld = (n_flds > 0).then(|| {
            let fld = self.zz.field_lists.ALLOC(n_flds);
            self.zz.field_lists.copy_from(fld, &subfields);
            fld
        });
        self.zz.fields.push(field_t {
            b,
            n_flds,
            fld,
            id,
            sides,
            ..field_t::default()
        })
    }

    fn apply_node(&mut self, line: &str) {
        let mut t = Tokens::new(line);
        t.expect("node");
        let i: usize = t.next().parse().expect("node index");
        let node_type = t.after("type").parse().expect("type");
        if !self.nodes.contains_key(&i) {
            let n = if node_type == 0 {
                agnode(&mut self.zz, self.root, Some(&format!("n{i}")), true).expect("node")
            } else {
                self.zz.new_agnode(self.root)
            };
            self.nodes.insert(i, n);
            let shape = self.shapes.remove(&i).expect("shape line before node line");
            self.apply_shape(n, &shape);
        }
        let n = self.nodes[&i];
        let rank = t.after("rank").parse().expect("rank");
        let order = t.after("order").parse().expect("order");
        t.expect("coord");
        let coord = t.pt();
        let zz = &mut self.zz;
        let nd = zz.nd_mut(n);
        nd.node_type = node_type;
        nd.rank = rank;
        nd.order = order;
        nd.coord = coord;
        nd.lw = double(t.after("lw"));
        nd.rw = double(t.after("rw"));
        nd.ht = double(t.after("ht"));
        nd.width = double(t.after("width"));
        nd.height = double(t.after("height"));
        nd.in_.size = t.after("in").parse().expect("in");
        nd.out.size = t.after("out").parse().expect("out");
    }

    fn apply_shape(&mut self, n: NodeId, line: &str) {
        let mut t = Tokens::new(line);
        t.expect("shape");
        t.next();
        let name = t.next();
        if name == "-" {
            return;
        }
        let shape = bind_shape(&self.zz, name);
        let info = match t.next() {
            "poly" => {
                let sides = t.i();
                let peripheries = t.i();
                let option = t.i();
                let count = t.i();
                let vertices: Vec<pointf> = (0..count).map(|_| t.pt()).collect();
                let list = self.zz.pointfs.ALLOC(count);
                self.zz.pointfs.copy_from(list, &vertices);
                SHAPE_INFO::Polygon(self.zz.polygons.push(polygon_t {
                    sides,
                    peripheries,
                    option,
                    vertices: Some(list),
                    ..polygon_t::default()
                }))
            }
            "record" => SHAPE_INFO::Field(self.field(&mut t)),
            other => panic!("shape kind {other}"),
        };
        let label = self.zz.textlabels.push(textlabel_t::default());
        let nd = self.zz.nd_mut(n);
        nd.shape = Some(shape);
        nd.shape_info = Some(info);
        nd.label = Some(label);
    }

    /// Creates the edge if needed: real edges (with attributes) in cgraph, the others as scratch pairs.
    fn create_edge(&mut self, line: &str) {
        let mut t = Tokens::new(line);
        t.expect("edge");
        let i: usize = t.next().parse().expect("edge index");
        let tail = self.node(t.after("tail"));
        let head = self.node(t.after("head"));
        if self.edges.contains_key(&i) {
            return;
        }
        let e = if line.contains(" attrs ") {
            let e = agedge(&mut self.zz, self.root, tail, head, None, true).expect("edge");
            AGMKOUT(&self.zz, e)
        } else {
            let e = self.zz.new_agedgepair();
            self.zz.tag_mut(e).objtype = AGOUTEDGE;
            let ein = AGOPP(&self.zz, e);
            self.zz.tag_mut(ein).objtype = AGINEDGE;
            e
        };
        self.edges.insert(i, e);
    }

    fn apply_edge(&mut self, line: &str) {
        let mut t = Tokens::new(line);
        t.expect("edge");
        let e = self.edge(t.next());
        let tail = self.node(t.after("tail"));
        let head = self.node(t.after("head"));
        if !line.contains(" attrs ") {
            M_agtail(&mut self.zz, e, tail);
            M_aghead(&mut self.zz, e, head);
        }
        let edge_type = t.after("type").parse().expect("type");
        let to_orig = match t.after("orig") {
            "-" => None,
            j => Some(self.edge(j)),
        };
        t.expect("tport");
        let tail_port = t.port();
        t.expect("hport");
        let head_port = t.port();
        t.expect("label");
        let label = if t.t[t.i] == "-" {
            t.i += 1;
            None
        } else {
            let dimen = t.pt();
            let pos = t.pt();
            let set = t.i();
            let l = self
                .zz
                .ed(e)
                .label
                .unwrap_or_else(|| self.zz.textlabels.push(textlabel_t::default()));
            let label = &mut self.zz.textlabels[l];
            label.dimen = dimen;
            label.pos = pos;
            label.set = set;
            Some(l)
        };
        let spl = match t.after("spl") {
            "-" => None,
            k => {
                let k: i32 = k.parse().expect("spline count");
                match self.zz.ed(e).spl {
                    Some(s) if self.zz.splines[s].size == k => Some(s),
                    _ => {
                        let list = self.zz.beziers.ALLOC(k);
                        Some(self.zz.splines.push(splines {
                            list: Some(list),
                            size: k,
                        }))
                    }
                }
            }
        };
        if t.i < t.t.len() {
            t.expect("attrs");
            for sym in self.syms {
                let value = t.next();
                if let Some(sym) = sym {
                    agxset(&mut self.zz, e, sym, &unquote(value));
                }
            }
        }
        let ed = self.zz.ed_mut(e);
        ed.edge_type = edge_type;
        ed.to_orig = to_orig;
        ed.tail_port = tail_port;
        ed.head_port = head_port;
        ed.label = label;
        ed.spl = spl;
    }

    fn graph_text(&self) -> String {
        let gd = self.zz.gd(self.root);
        format!(
            "graph rankdir {} flags {} bb {}",
            gd.rankdir,
            gd.flags,
            bx(gd.bb)
        )
    }

    fn node_text(&self, i: usize) -> String {
        let nd = self.zz.nd(self.nodes[&i]);
        format!(
            "node {i} type {} rank {} order {} coord {} lw {:?} rw {:?} ht {:?} width {:?} height {:?} in {} out {}",
            nd.node_type,
            nd.rank,
            nd.order,
            pt(nd.coord),
            nd.lw,
            nd.rw,
            nd.ht,
            nd.width,
            nd.height,
            nd.in_.size,
            nd.out.size
        )
    }

    fn index_of_node(&self, n: NodeId) -> usize {
        *self
            .nodes
            .iter()
            .find(|&(_, &m)| m == n)
            .expect("known node")
            .0
    }

    fn index_of_edge(&self, e: EdgeId) -> usize {
        *self
            .edges
            .iter()
            .find(|&(_, &f)| f == e)
            .expect("known edge")
            .0
    }

    fn edge_text(&mut self, i: usize) -> String {
        let e = self.edges[&i];
        let zz = &self.zz;
        let ed = *zz.ed(e);
        let mut s = format!(
            "edge {i} tail {} head {} type {} orig {} tport {} hport {} label {} spl {}",
            self.index_of_node(agtail(zz, e)),
            self.index_of_node(aghead(zz, e)),
            ed.edge_type,
            ed.to_orig
                .map_or_else(|| "-".to_string(), |o| self.index_of_edge(o).to_string()),
            port_text(&ed.tail_port),
            port_text(&ed.head_port),
            ed.label.map_or_else(
                || "-".to_string(),
                |l| {
                    let l = &zz.textlabels[l];
                    format!("{} {} {}", pt(l.dimen), pt(l.pos), l.set)
                }
            ),
            ed.spl
                .map_or_else(|| "-".to_string(), |s| zz.splines[s].size.to_string())
        );
        if ed.to_orig.is_none() {
            s.push_str(" attrs");
            for sym in self.syms {
                let value = sym.map_or_else(
                    || "-".to_string(),
                    |sym| {
                        let v = agxget(&mut self.zz, e, sym).expect("attribute value");
                        quote(self.zz.agstr(v))
                    },
                );
                write!(s, " {value}").unwrap();
            }
        }
        s
    }

    /// The Bézier the call appended to the splines of `e`'s real edge.
    fn last_bezier(&self, mut e: EdgeId) -> String {
        while self.zz.ed(e).edge_type != 0 {
            e = self.zz.ed(e).to_orig.expect("ED_to_orig");
        }
        let spl = self.zz.splines[self.zz.ed(e).spl.expect("ED_spl")];
        let bz = self
            .zz
            .beziers
            .get(spl.list.expect("beziers"), spl.size - 1);
        bezier_text(&self.zz, &bz)
    }
}

// ---------------------------------------------------------------- the calls

/// Makes the call, returning the result lines to compare with the `=>` lines, each with token indices to skip.
#[allow(clippy::too_many_lines, reason = "one arm per recorded function")]
fn call(layout: &mut Layout, line: &str) -> Vec<(String, Vec<usize>)> {
    let mut t = Tokens::new(line);
    let kind = t.next();
    match kind {
        "clip_and_install" => {
            let fe = layout.edge(t.after("fe"));
            let hn = layout.node(t.after("hn"));
            let info = sinfo(&mut t);
            t.expect("ps");
            let mut ps = t.points();
            let pn = ps.len() as i32;
            clip_and_install(&mut layout.zz, fe, hn, &mut ps, pn, &info);
            vec![
                (format!("=> ps {}", points(&ps)), vec![]),
                (format!("=> {}", layout.last_bezier(fe)), vec![]),
            ]
        }
        "beginpath" | "endpath" => {
            let e = layout.edge(t.after("e"));
            let et = t.after("et").parse().expect("et");
            let merge = t.after("merge") == "1";
            t.expect("endp");
            let nb = t.bx();
            t.pt();
            let mut endp = pathend_t {
                nb,
                sidemask: t.i(),
                boxn: t.i(),
                ..pathend_t::default()
            };
            endp.boxes[0] = t.bx();
            let mut P = path::default();
            let begin = kind == "beginpath";
            if begin {
                beginpath(&mut layout.zz, &mut P, e, et, &mut endp, merge);
            } else {
                endpath(&mut layout.zz, &mut P, e, et, &mut endp, merge);
            }
            let p = if begin { &P.start } else { &P.end };
            // Java reuses P: an unconstrained end keeps an old theta, and endpath leaves nbox alone.
            // Tokens 13 and 14 are `np`.
            let mut skip = vec![13, 14];
            if !p.constrained {
                skip.push(4);
            }
            if !begin {
                skip.push(7);
            }
            let text = format!(
                "=> port {} {:?} {} nbox {} endp {}",
                pt(p.p),
                p.theta,
                b(p.constrained),
                P.nbox,
                endp_text(&endp)
            );
            vec![(text, skip)]
        }
        "routesplines" => {
            let e = layout.edge(t.after("e"));
            let polyline = t.after("polyline") == "1";
            let mut P = path::default();
            t.expect("start");
            P.start.p = t.pt();
            P.start.theta = t.f();
            P.start.constrained = t.b();
            t.expect("end");
            P.end.p = t.pt();
            P.end.theta = t.f();
            P.end.constrained = t.b();
            t.expect("boxes");
            P.nbox = t.i();
            P.boxes = (0..P.nbox).map(|_| t.bx()).collect();
            P.data = Some(e);
            let ps = if polyline {
                routepolylines(&mut layout.zz, &mut P)
            } else {
                routesplines(&mut layout.zz, &mut P)
            };
            vec![
                (
                    ps.map_or_else(
                        || "=> null".to_string(),
                        |ps| format!("=> points {}", points(&ps)),
                    ),
                    vec![],
                ),
                (
                    format!(
                        "=> start {} end {} boxes {}",
                        pt(P.start.p),
                        pt(P.end.p),
                        boxes_text(&P)
                    ),
                    vec![],
                ),
            ]
        }
        "simpleSplineRoute" => {
            t.expect("tp");
            let tp = t.pt();
            t.expect("hp");
            let hp = t.pt();
            let polyline = t.after("polyline") == "1";
            t.expect("poly");
            let poly = t.points();
            let ps = simpleSplineRoute(&mut layout.zz, tp, hp, &poly, polyline);
            vec![(
                ps.map_or_else(
                    || "=> null".to_string(),
                    |ps| format!("=> points {}", points(&ps)),
                ),
                vec![],
            )]
        }
        "makeSelfEdge" => {
            let sizex = double(t.after("sizex"));
            let sizey = double(t.after("sizey"));
            let info = sinfo(&mut t);
            let cnt: i32 = t.after("edges").parse().expect("count");
            let edges: Vec<EdgeId> = (0..cnt).map(|_| layout.edge(t.next())).collect();
            makeSelfEdge(&mut layout.zz, &edges, 0, cnt, sizex, sizey, &info);
            edges
                .iter()
                .map(|&e| (format!("=> {}", layout.last_bezier(e)), vec![]))
                .collect()
        }
        "poly_port" | "record_port" => {
            let n = layout.node(t.after("n"));
            let name = unquote(t.after("name"));
            let compass = match t.after("compass") {
                "-" => None,
                q => Some(unquote(q)),
            };
            let p = portfn(&mut layout.zz, n, &name, compass.as_deref());
            vec![(format!("=> port {}", port_text(&p)), vec![])]
        }
        other => panic!("unknown call {other}"),
    }
}

/// Runs `f`, returning its panic message if it panics, without printing the panic.
fn panic_message<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    panic::set_hook(hook);
    result.map_err(|payload| {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
            .unwrap_or_default()
    })
}

const CALLS: [&str; 8] = [
    "clip_and_install",
    "beginpath",
    "endpath",
    "routesplines",
    "simpleSplineRoute",
    "makeSelfEdge",
    "poly_port",
    "record_port",
];

/// Replays one recorded call; returns the differences.
fn replay(layout: &mut Layout, lines: &[(usize, &str)], failures: &mut Vec<String>) {
    let at = lines[0].0;
    let mut state = Vec::new();
    let mut call_line = None;
    let mut results = Vec::new();
    let mut afters = Vec::new();
    for &(n, line) in lines {
        let first = line.split(' ').next().unwrap_or_default();
        if CALLS.contains(&first) {
            call_line = Some((n, line));
        } else if let Some(r) = line.strip_prefix("=> ") {
            results.push((n, format!("=> {r}")));
        } else if let Some(a) = line.strip_prefix("after ") {
            afters.push((n, a));
        } else {
            state.push(line);
        }
    }
    let (call_at, call_text) = call_line.unwrap_or_else(|| panic!("line {at}: no call"));
    for line in state.iter().filter(|l| l.starts_with("syms ")) {
        layout.apply_syms(line);
    }
    for line in &state {
        match line.split(' ').next() {
            Some("graph") => layout.apply_graph(line),
            Some("shape") => {
                let i: usize = line.split(' ').nth(1).unwrap().parse().unwrap();
                layout.shapes.insert(i, (*line).to_string());
            }
            _ => {}
        }
    }
    for line in state.iter().filter(|l| l.starts_with("node ")) {
        layout.apply_node(line);
    }
    let edges: Vec<&&str> = state.iter().filter(|l| l.starts_with("edge ")).collect();
    for line in &edges {
        layout.create_edge(line);
    }
    for line in &edges {
        layout.apply_edge(line);
    }
    let actual = match panic_message(|| call(layout, call_text)) {
        Ok(actual) => actual,
        Err(message) => {
            failures.push(format!(
                "line {call_at}: {call_text:.80}: panicked: {message}"
            ));
            layout.broken = true;
            return;
        }
    };
    if actual.len() != results.len() {
        failures.push(format!(
            "line {call_at}: {} results, expected {}",
            actual.len(),
            results.len()
        ));
        return;
    }
    for ((n, expected), (got, skip)) in results.iter().zip(&actual) {
        if !same(expected, got, skip) {
            failures.push(format!("line {n}: expected {expected}\n   but got {got}"));
        }
    }
    for (n, after) in afters {
        let mut t = Tokens::new(after);
        let got = match t.next() {
            "graph" => layout.graph_text(),
            "node" => layout.node_text(t.next().parse().unwrap()),
            "edge" => layout.edge_text(t.next().parse().unwrap()),
            other => panic!("line {n}: after {other}"),
        };
        if !same(after, &got, &[]) {
            failures.push(format!(
                "line {n}: expected after {after}\n   but got {got}"
            ));
        }
    }
}

#[test]
fn routes_like_smetana() {
    let mut report = String::new();
    let mut total_failures = 0;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (name, fixture) in FIXTURES {
        let mut layouts: HashMap<usize, Layout> = HashMap::new();
        let mut current = 0;
        let mut event: Vec<(usize, &str)> = Vec::new();
        let mut failures = Vec::new();
        for (index, line) in fixture.lines().enumerate() {
            if line.starts_with('#') {
                continue;
            }
            if let Some(i) = line.strip_prefix("layout ") {
                current = i.parse().expect("layout index");
                continue;
            }
            if line != "end" {
                event.push((index + 1, line));
                continue;
            }
            let layout = layouts.entry(current).or_insert_with(Layout::new);
            if !layout.broken {
                if let Some(kind) = event
                    .iter()
                    .find_map(|(_, l)| CALLS.iter().find(|c| l.split(' ').next() == Some(c)))
                {
                    *counts.entry(kind).or_default() += 1;
                }
                replay(layout, &event, &mut failures);
            }
            event.clear();
        }
        total_failures += failures.len();
        for f in failures.iter().take(20) {
            writeln!(report, "{name} {f}").unwrap();
        }
    }
    assert!(
        total_failures == 0,
        "{total_failures} differences:\n{report}"
    );
    for kind in CALLS {
        assert!(
            counts.get(kind).copied().unwrap_or(0) > 0,
            "no {kind} calls replayed"
        );
    }
}

//! Lays out every Smetana trace's graph phase by phase and compares the state after each phase with Java's
//! trace: `phase rank` (each graph's and cluster's rank range, then every real node's rank), `phase mincross`
//! (each rank's nodes left to right), `phase position` (every node's coordinates and size, each graph's
//! bounding box), `phase splines` (every edge's splines and labels) and, for the whole layout with
//! post-processing, `phase final`, both through the ported functions and through the public API.

mod trace;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};

use smetana::internals::cgraph::id::agnameof;
use smetana::internals::cgraph::node::{agfstnode, agnxtnode};
use smetana::internals::cgraph::rec::{Rec, agbindrec};
use smetana::internals::common::input::graph_init;
use smetana::internals::common::utils::setEdgeType;
use smetana::internals::core::Globals;
use smetana::internals::core::consts::{ET_SPLINE, VIRTUAL};
use smetana::internals::core::ids::{GraphId, NodeId, TextlabelId};
use smetana::internals::dotgen::aspect::setAspect;
use smetana::internals::dotgen::dotinit::{dot_init_node_edge, dot_init_subg};
use smetana::internals::dotgen::dotsplines::dot_splines;
use smetana::internals::dotgen::mincross::dot_mincross;
use smetana::internals::dotgen::position::dot_position;
use smetana::internals::dotgen::rank::dot_rank;
use smetana::internals::dotgen::sameport::dot_sameports;
use smetana::internals::gvc::gvlayout::gvLayoutJobs;
use smetana::{Drawing, Graph, Label, Node, Object, Subgraph};
use trace::{Call, Replay};

/// `gvLayoutJobs` → `dot_layout` → `doDot` → `dotLayout`, up to `dot_rank`.
fn layout_until_rank(r: &mut Replay) {
    let (zz, g) = (&mut r.zz, r.root);
    agbindrec(zz, g, Rec::Info);
    graph_init(zz, g, true);
    setEdgeType(zz, g, ET_SPLINE);
    setAspect(zz, g);
    dot_init_subg(zz, g, g);
    dot_init_node_edge(zz, g);
    dot_rank(zz, g);
}

/// `dotLayout` up to `dot_mincross`.
fn layout_until_mincross(r: &mut Replay) {
    layout_until_rank(r);
    dot_mincross(&mut r.zz, r.root);
}

fn graph_ranks(zz: &Globals, g: GraphId, out: &mut String) {
    let info = zz.gd(g);
    let name = trace::quote(&agnameof(zz, g).expect("graph name"));
    writeln!(
        out,
        "graph {name} minrank {} maxrank {}",
        info.minrank, info.maxrank
    )
    .unwrap();
    for c in 1..=info.n_cluster {
        let clust = zz
            .graph_lists
            .get(info.clust.expect("clusters"), c)
            .expect("cluster");
        graph_ranks(zz, clust, out);
    }
}

/// The `phase rank` section as the Java tracer writes it.
fn dump_ranks(r: &mut Replay) -> String {
    let mut out = String::new();
    graph_ranks(&r.zz, r.root, &mut out);
    let mut n = agfstnode(&mut r.zz, r.root);
    while let Some(nn) = n {
        let name = trace::quote(&agnameof(&r.zz, nn).expect("node name"));
        writeln!(out, "node {name} rank {}", r.zz.nd(nn).rank).unwrap();
        n = agnxtnode(&mut r.zz, r.root, nn);
    }
    out
}

/// Java's `phase rank` section, without the calls PlantUML makes during later phases.
fn expected_ranks(trace: &trace::Trace) -> String {
    let (_, lines) = trace
        .phases
        .iter()
        .find(|(phase, _)| phase == "rank")
        .expect("phase rank");
    lines
        .iter()
        .filter(|l| l.starts_with("graph ") || l.starts_with("node "))
        .fold(String::new(), |mut out, l| {
            out.push_str(l);
            out.push('\n');
            out
        })
}

/// Node names as the Java tracer writes them: real nodes by quoted name, virtual nodes as `v1`, `v2`... in the
/// order the trace first meets them, keeping their names for the rest of the trace.
#[derive(Default)]
struct Names {
    names: HashMap<NodeId, String>,
    virtuals: usize,
}

impl Names {
    fn of(&mut self, zz: &Globals, n: NodeId) -> &str {
        self.names.entry(n).or_insert_with(|| {
            if zz.nd(n).node_type == VIRTUAL {
                self.virtuals += 1;
                format!("v{}", self.virtuals)
            } else {
                trace::quote(&agnameof(zz, n).expect("node name"))
            }
        })
    }
}

/// The nodes of `GD_rank(g)[r]`, left to right.
fn rank_nodes(zz: &Globals, g: GraphId, r: i32) -> Vec<NodeId> {
    let rank = zz.rank(g, r);
    let v = rank.v.expect("rank");
    (0..rank.n)
        .map(|i| zz.node_lists.get(v, i).expect("node in rank"))
        .collect()
}

/// The `phase mincross` section as the Java tracer writes it.
fn dump_orders(zz: &Globals, root: GraphId, names: &mut Names) -> String {
    let mut out = String::new();
    let info = zz.gd(root);
    for rank in info.minrank..=info.maxrank {
        write!(out, "rank {rank}").unwrap();
        for n in rank_nodes(zz, root, rank) {
            write!(out, " {}", names.of(zz, n)).unwrap();
        }
        out.push('\n');
    }
    out
}

/// A graph's or edge's label as the Java tracer writes it.
fn dump_label(
    zz: &Globals,
    out: &mut String,
    reference: &str,
    kind: &str,
    label: Option<TextlabelId>,
) {
    if let Some(l) = label {
        let l = &zz.textlabels[l];
        writeln!(
            out,
            "{reference} {kind} pos {:?} {:?} dimen {:?} {:?} set {}",
            l.pos.x, l.pos.y, l.dimen.x, l.dimen.y, l.set
        )
        .unwrap();
    }
}

/// Each graph's bounding box and label, the root first, then the clusters depth-first.
fn dump_graph_boxes(zz: &Globals, g: GraphId, out: &mut String) {
    let info = zz.gd(g);
    let reference = format!(
        "graph {}",
        trace::quote(&agnameof(zz, g).expect("graph name"))
    );
    let bb = info.bb;
    writeln!(
        out,
        "{reference} bb {:?} {:?} {:?} {:?}",
        bb.LL.x, bb.LL.y, bb.UR.x, bb.UR.y
    )
    .unwrap();
    dump_label(zz, out, &reference, "label", info.label);
    for c in 1..=info.n_cluster {
        let clust = zz
            .graph_lists
            .get(info.clust.expect("clusters"), c)
            .expect("cluster");
        dump_graph_boxes(zz, clust, out);
    }
}

/// The `phase position` section as the Java tracer writes it.
fn dump_positions(zz: &Globals, root: GraphId, names: &mut Names) -> String {
    let mut out = String::new();
    let info = zz.gd(root);
    for rank in info.minrank..=info.maxrank {
        for n in rank_nodes(zz, root, rank) {
            let i = zz.nd(n);
            writeln!(
                out,
                "node {} rank {rank} coord {:?} {:?} lw {:?} rw {:?} ht {:?}",
                names.of(zz, n),
                i.coord.x,
                i.coord.y,
                i.lw,
                i.rw,
                i.ht
            )
            .unwrap();
        }
    }
    dump_graph_boxes(zz, root, &mut out);
    out
}

/// Java's `phase position` section, with its doubles written as Rust writes them (Java's `Double.toString`
/// differs in form, not in value).
fn expected_positions(trace: &trace::Trace) -> String {
    let (_, lines) = trace
        .phases
        .iter()
        .find(|(phase, _)| phase == "position")
        .expect("phase position");
    lines
        .iter()
        .filter(|l| l.starts_with("node ") || l.starts_with("graph "))
        .fold(String::new(), |mut out, l| {
            let mut tokens = l.split(' ');
            let head: Vec<&str> = tokens.by_ref().take(2).collect();
            out.push_str(&head.join(" "));
            let mut previous = "";
            for t in tokens {
                out.push(' ');
                // Ranks and the label's `set` are integers.
                match t.parse::<f64>() {
                    Ok(x) if previous != "rank" && previous != "set" => {
                        write!(out, "{x:?}").unwrap();
                    }
                    _ => out.push_str(t),
                }
                previous = t;
            }
            out.push('\n');
            out
        })
}

/// Java's `phase mincross` section.
fn expected_orders(trace: &trace::Trace) -> String {
    let (_, lines) = trace
        .phases
        .iter()
        .find(|(phase, _)| phase == "mincross")
        .expect("phase mincross");
    lines
        .iter()
        .filter(|l| l.starts_with("rank "))
        .fold(String::new(), |mut out, l| {
            out.push_str(l);
            out.push('\n');
            out
        })
}

/// Node sizes in inches as the `phase final` section has them: initialisation sets them for good.
fn dump_sizes(r: &mut Replay) -> String {
    let mut out = String::new();
    let mut n = agfstnode(&mut r.zz, r.root);
    while let Some(nn) = n {
        let name = trace::quote(&agnameof(&r.zz, nn).expect("node name"));
        let info = r.zz.nd(nn);
        writeln!(
            out,
            "node {name} width {:?} height {:?}",
            info.width, info.height
        )
        .unwrap();
        n = agnxtnode(&mut r.zz, r.root, nn);
    }
    out
}

/// The node sizes of Java's `phase final` section.
fn expected_sizes(trace: &trace::Trace) -> String {
    let (_, lines) = trace
        .phases
        .iter()
        .find(|(phase, _)| phase == "final")
        .expect("phase final");
    lines
        .iter()
        .filter(|l| l.starts_with("node "))
        .fold(String::new(), |mut out, l| {
            let t: Vec<&str> = l.split(' ').collect();
            let number = |s: &str| s.parse::<f64>().expect("number");
            let (width, height) = (number(t[6]), number(t[8]));
            writeln!(out, "node {} width {width:?} height {height:?}", t[1]).unwrap();
            out
        })
}

/// Label sizes as the `phase final` section has them: the labels of clusters, then of edges.
fn dump_label_sizes(r: &mut Replay) -> String {
    let zz = &r.zz;
    let mut out = String::new();
    let mut graphs = vec![r.root];
    while let Some(g) = graphs.pop() {
        let info = zz.gd(g);
        if let Some(l) = info.label {
            let name = trace::quote(&agnameof(zz, g).expect("graph name"));
            let d = zz.textlabels[l].dimen;
            writeln!(out, "graph {name} label dimen {:?} {:?}", d.x, d.y).unwrap();
        }
        for c in (1..=info.n_cluster).rev() {
            graphs.push(
                zz.graph_lists
                    .get(info.clust.expect("clusters"), c)
                    .expect("cluster"),
            );
        }
    }
    for (i, &e) in r.edges.iter().enumerate() {
        let info = zz.ed(e);
        for (kind, label) in [
            ("label", info.label),
            ("head_label", info.head_label),
            ("tail_label", info.tail_label),
        ] {
            if let Some(l) = label {
                let d = zz.textlabels[l].dimen;
                writeln!(out, "edge e{} {kind} dimen {:?} {:?}", i + 1, d.x, d.y).unwrap();
            }
        }
    }
    out
}

/// The label sizes of Java's `phase final` section.
fn expected_label_sizes(trace: &trace::Trace) -> String {
    let (_, lines) = trace
        .phases
        .iter()
        .find(|(phase, _)| phase == "final")
        .expect("phase final");
    lines
        .iter()
        .filter_map(|l| {
            let (head, rest) = l.split_once(" pos ")?;
            let (_, dimen) = rest.split_once(" dimen ")?;
            let mut d = dimen.split(' ').map(|s| s.parse::<f64>().expect("number"));
            let (w, h) = (d.next()?, d.next()?);
            Some(format!("{head} dimen {w:?} {h:?}\n"))
        })
        .collect()
}

/// Replays every trace's input through the ported cgraph, lays its graph out with `run`, then compares the text
/// it returns with `expected`'s.
fn check_all(run: fn(&mut Replay) -> String, expected: fn(&trace::Trace) -> String) {
    check_all_traces(
        |t| {
            let mut replay = trace::replay(&t.input);
            run(&mut replay)
        },
        expected,
    );
}

/// Runs `run` on every trace, then compares the text it returns with `expected`'s.
fn check_all_traces(run: impl Fn(&trace::Trace) -> String, expected: fn(&trace::Trace) -> String) {
    let traces = trace::files(&trace::repo_tests_dir().join("smetana"), "trace");
    assert!(!traces.is_empty(), "no traces");
    let mut failures = Vec::new();
    for path in &traces {
        let trace = trace::parse(&std::fs::read_to_string(path).unwrap());
        let want = expected(&trace);
        let got = catch_unwind(AssertUnwindSafe(|| run(&trace)));
        match got {
            Ok(got) if got == want => {}
            Ok(got) => {
                let first = got
                    .lines()
                    .zip(want.lines())
                    .find(|(g, w)| g != w)
                    .map_or_else(
                        || "different line counts".to_owned(),
                        |(g, w)| format!("got {g:?}, want {w:?}"),
                    );
                failures.push(format!("{}: {first}", path.display()));
            }
            Err(_) => failures.push(format!("{}: panicked", path.display())),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} traces differ:\n{}",
        failures.len(),
        traces.len(),
        failures.join("\n")
    );
}

#[test]
fn ranks_match_java() {
    check_all(
        |r| {
            layout_until_rank(r);
            dump_ranks(r)
        },
        expected_ranks,
    );
}

#[test]
fn mincross_orders_match_java() {
    check_all(
        |r| {
            layout_until_mincross(r);
            dump_orders(&r.zz, r.root, &mut Names::default())
        },
        expected_orders,
    );
}

#[test]
fn positions_match_java() {
    check_all(
        |r| {
            layout_until_mincross(r);
            // Virtual nodes are named in the mincross dump first.
            let mut names = Names::default();
            dump_orders(&r.zz, r.root, &mut names);
            dot_position(&mut r.zz, r.root);
            dump_positions(&r.zz, r.root, &mut names)
        },
        expected_positions,
    );
}

#[test]
fn node_sizes_match_java() {
    check_all(
        |r| {
            layout_until_rank(r);
            dump_sizes(r)
        },
        expected_sizes,
    );
}

#[test]
fn label_sizes_match_java() {
    check_all(
        |r| {
            layout_until_rank(r);
            dump_label_sizes(r)
        },
        expected_label_sizes,
    );
}

/// Every edge's splines and labels, as the Java tracer writes the `phase splines` and `phase final` sections.
fn dump_edges(r: &Replay, out: &mut String) {
    let zz = &r.zz;
    for (i, &e) in r.edges.iter().enumerate() {
        let reference = format!("edge e{}", i + 1);
        let info = zz.ed(e);
        match info.spl {
            None => writeln!(out, "{reference} spl none").unwrap(),
            Some(spl) => {
                let spl = zz.splines[spl];
                for j in 0..spl.size {
                    let bz = zz.beziers.get(spl.list.expect("beziers"), j);
                    write!(
                        out,
                        "{reference} bezier {j} sflag {} eflag {} sp {:?} {:?} ep {:?} {:?} points {}",
                        bz.sflag, bz.eflag, bz.sp.x, bz.sp.y, bz.ep.x, bz.ep.y, bz.size
                    )
                    .unwrap();
                    for k in 0..bz.size {
                        let p = zz.pointfs.get(bz.list.expect("points"), k);
                        write!(out, " {:?} {:?}", p.x, p.y).unwrap();
                    }
                    out.push('\n');
                }
            }
        }
        dump_label(zz, out, &reference, "label", info.label);
        dump_label(zz, out, &reference, "head_label", info.head_label);
        dump_label(zz, out, &reference, "tail_label", info.tail_label);
        dump_label(zz, out, &reference, "xlabel", info.xlabel);
    }
}

/// A Java phase section with its doubles (the tokens with a point or an exponent) written as Rust writes them.
fn expected_section(trace: &trace::Trace, phase: &str) -> String {
    let (_, lines) = trace
        .phases
        .iter()
        .find(|(p, _)| p == phase)
        .unwrap_or_else(|| panic!("phase {phase}"));
    lines.iter().fold(String::new(), |mut out, l| {
        let tokens: Vec<String> = l
            .split(' ')
            .map(|t| match t.parse::<f64>() {
                Ok(x) if t.contains(['.', 'E']) => format!("{x:?}"),
                _ => t.to_owned(),
            })
            .collect();
        out.push_str(&tokens.join(" "));
        out.push('\n');
        out
    })
}

#[test]
fn splines_match_java() {
    check_all(
        |r| {
            layout_until_mincross(r);
            dot_position(&mut r.zz, r.root);
            dot_sameports(&mut r.zz, r.root);
            dot_splines(&mut r.zz, r.root);
            let mut out = String::new();
            dump_edges(r, &mut out);
            out
        },
        |t| expected_section(t, "splines"),
    );
}

/// The `phase final` section as the Java tracer writes it: what PlantUML reads back.
fn dump_final(r: &mut Replay) -> String {
    let mut out = String::new();
    dump_graph_boxes(&r.zz, r.root, &mut out);
    let mut n = agfstnode(&mut r.zz, r.root);
    while let Some(nn) = n {
        let name = trace::quote(&agnameof(&r.zz, nn).expect("node name"));
        let i = r.zz.nd(nn);
        writeln!(
            out,
            "node {name} coord {:?} {:?} width {:?} height {:?} lw {:?} rw {:?} ht {:?}",
            i.coord.x, i.coord.y, i.width, i.height, i.lw, i.rw, i.ht
        )
        .unwrap();
        n = agnxtnode(&mut r.zz, r.root, nn);
    }
    dump_edges(r, &mut out);
    out
}

#[test]
fn whole_layouts_match_java() {
    check_all(
        |r| {
            gvLayoutJobs(&mut r.zz, r.root);
            dump_final(r)
        },
        |t| expected_section(t, "final"),
    );
}

/// API objects with their trace names, in creation order.
type Named<T> = Vec<(String, T)>;

/// The object named `name`.
fn find<T: Copy>(list: &[(String, T)], name: &str) -> T {
    list.iter()
        .find(|(n, _)| n == name)
        .map_or_else(|| panic!("unknown {name}"), |&(_, x)| x)
}

/// Makes the calls of a trace's input section through the public API, as PlantUML made them, and lays the graph
/// out. Returns the drawing with the trace's names for its subgraphs and nodes.
fn layout_through_api(input: &[Call]) -> (Drawing, Named<Subgraph>, Named<Node>) {
    let mut graph = Graph::new();
    let Some(Call::Agopen(root)) = input.first() else {
        panic!("a trace starts with agopen")
    };
    let mut subgraphs = vec![(root.clone(), graph.root())];
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for c in &input[1..] {
        match c {
            Call::Agopen(_) => panic!("a second agopen"),
            Call::Agsubg { graph: g, name } => {
                let subg = graph.subgraph(find(&subgraphs, g), name);
                subgraphs.push((name.clone(), subg));
            }
            Call::Agnode { graph: g, name } => {
                let n = graph.node(find(&subgraphs, g), name);
                nodes.push((name.clone(), n));
            }
            Call::Agedge {
                graph: g,
                tail,
                head,
                name,
            } => {
                assert!(name.is_none(), "PlantUML's edges are anonymous");
                let (t, h) = (find(&nodes, tail), find(&nodes, head));
                edges.push(graph.edge(find(&subgraphs, g), t, h));
            }
            Call::Agsafeset {
                object,
                name,
                value,
                def,
            } => {
                let obj: Object = match object {
                    trace::Object::Graph(g) => find(&subgraphs, g).into(),
                    trace::Object::Node(n) => find(&nodes, n).into(),
                    trace::Object::Edge(i) => edges[i - 1].into(),
                };
                let size = value
                    .strip_prefix("_dim_")
                    .and_then(|v| v.strip_suffix('_'))
                    .and_then(|v| v.split_once('_'))
                    .and_then(|(w, h)| Some((w.parse::<i32>().ok()?, h.parse::<i32>().ok()?)));
                match size {
                    Some((w, h)) if def.is_empty() => {
                        graph.set_label_size(obj, name, f64::from(w), f64::from(h));
                    }
                    _ if def.is_empty() => graph.set(obj, name, value),
                    _ => graph.set_with_default(obj, name, value, def),
                }
            }
            Call::GvContext => graph.gv_context(),
            Call::GvLayoutJobs(_) => break,
        }
    }
    let drawing = graph.layout().unwrap_or_else(|e| panic!("{e}"));
    (drawing, subgraphs, nodes)
}

/// A label as the Java tracer writes it.
fn api_label(out: &mut String, reference: &str, kind: &str, label: Option<&Label>) {
    if let Some(l) = label {
        writeln!(
            out,
            "{reference} {kind} pos {:?} {:?} dimen {:?} {:?} set {}",
            l.pos.x,
            l.pos.y,
            l.size.x,
            l.size.y,
            i32::from(l.placed)
        )
        .unwrap();
    }
}

/// The `phase final` section from a drawing, without node widths in points (which PlantUML does not read), and
/// with the graphs sorted: the API lists them in creation order, the tracer in dot's cluster order.
fn dump_drawing(trace: &trace::Trace) -> String {
    let (drawing, subgraphs, nodes) = layout_through_api(&trace.input);
    let mut graphs = Vec::new();
    for (name, g) in &subgraphs {
        let reference = format!("graph {}", trace::quote(name));
        let layout = drawing.subgraph(*g);
        let (ll, ur) = (layout.bb.lower_left, layout.bb.upper_right);
        let mut out = format!(
            "{reference} bb {:?} {:?} {:?} {:?}
",
            ll.x, ll.y, ur.x, ur.y
        );
        api_label(&mut out, &reference, "label", layout.label.as_ref());
        graphs.extend(out.lines().map(str::to_owned));
    }
    graphs.sort();
    let mut out = graphs.join(
        "
",
    ) + "
";
    for (name, n) in &nodes {
        let layout = drawing.node(*n);
        writeln!(
            out,
            "node {} coord {:?} {:?} width {:?} height {:?}",
            trace::quote(name),
            layout.center.x,
            layout.center.y,
            layout.width,
            layout.height
        )
        .unwrap();
    }
    for (i, e) in drawing.edges().iter().enumerate() {
        let reference = format!("edge e{}", i + 1);
        if e.beziers.is_empty() {
            writeln!(out, "{reference} spl none").unwrap();
        }
        for (j, bz) in e.beziers.iter().enumerate() {
            write!(
                out,
                "{reference} bezier {j} sflag {} eflag {} sp {:?} {:?} ep {:?} {:?} points {}",
                bz.sflag,
                bz.eflag,
                bz.sp.x,
                bz.sp.y,
                bz.ep.x,
                bz.ep.y,
                bz.points.len()
            )
            .unwrap();
            for p in &bz.points {
                write!(out, " {:?} {:?}", p.x, p.y).unwrap();
            }
            out.push('\n');
        }
        api_label(&mut out, &reference, "label", e.label.as_ref());
        api_label(&mut out, &reference, "head_label", e.head_label.as_ref());
        api_label(&mut out, &reference, "tail_label", e.tail_label.as_ref());
    }
    out
}

/// Java's `phase final` section in [`dump_drawing`]'s form.
fn expected_drawing(trace: &trace::Trace) -> String {
    let section = expected_section(trace, "final");
    let mut graphs: Vec<&str> = section
        .lines()
        .filter(|l| l.starts_with("graph "))
        .collect();
    graphs.sort_unstable();
    let mut out = graphs.join(
        "
",
    ) + "
";
    for l in section.lines().filter(|l| !l.starts_with("graph ")) {
        let l = if l.starts_with("node ") {
            l.split_once(" lw ").map_or(l, |(head, _)| head)
        } else {
            l
        };
        out.push_str(l);
        out.push('\n');
    }
    out
}

#[test]
fn layouts_through_the_api_match_java() {
    check_all_traces(dump_drawing, expected_drawing);
}

/// Graphs that make Java's Smetana loop forever in mincross (`tests/smetana-hangs`, random seeds traced until
/// they hung): their ranks still match Java's, and the layout terminates with a sane drawing.
#[test]
fn layouts_that_hang_in_java_terminate() {
    let traces = trace::files(&trace::repo_tests_dir().join("smetana-hangs"), "trace");
    assert!(!traces.is_empty(), "no traces");
    for path in traces {
        let trace = trace::parse(&std::fs::read_to_string(&path).unwrap());
        let mut replay = trace::replay(&trace.input);
        layout_until_rank(&mut replay);
        assert_eq!(
            dump_ranks(&mut replay),
            expected_ranks(&trace),
            "{}",
            path.display()
        );

        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || send.send(layout_through_api(&trace.input).0));
        let drawing = receive
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap_or_else(|_| panic!("{}: no layout within a minute", path.display()));
        let finite = |p: &smetana::Point| p.x.is_finite() && p.y.is_finite();
        assert!(
            drawing.nodes().iter().all(|n| finite(&n.center)),
            "{}",
            path.display()
        );
        for e in drawing.edges() {
            assert!(
                !e.beziers.is_empty(),
                "{}: an edge without spline",
                path.display()
            );
            assert!(
                e.beziers.iter().flat_map(|b| &b.points).all(finite),
                "{}",
                path.display()
            );
        }
    }
}

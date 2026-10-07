//! Lays out every Smetana trace's graph phase by phase and compares the state after each phase with Java's
//! trace: `phase rank` (each graph's and cluster's rank range, then every real node's rank), `phase mincross`
//! (each rank's nodes left to right) and `phase position` (every node's coordinates and size, each graph's
//! bounding box).

mod trace;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};

use smetana::cgraph::id::agnameof;
use smetana::cgraph::node::{agfstnode, agnxtnode};
use smetana::cgraph::rec::{Rec, agbindrec};
use smetana::common::input::graph_init;
use smetana::common::utils::setEdgeType;
use smetana::core::Globals;
use smetana::core::consts::{ET_SPLINE, VIRTUAL};
use smetana::core::ids::{GraphId, NodeId, TextlabelId};
use smetana::dotgen::aspect::{aspect_t, setAspect};
use smetana::dotgen::dotinit::{dot_init_node_edge, dot_init_subg};
use smetana::dotgen::mincross::dot_mincross;
use smetana::dotgen::position::dot_position;
use smetana::dotgen::rank::dot_rank;
use trace::Replay;

/// `gvLayoutJobs` → `dot_layout` → `doDot` → `dotLayout`, up to `dot_rank`.
fn layout_until_rank(r: &mut Replay) {
    let (zz, g) = (&mut r.zz, r.root);
    agbindrec(zz, g, Rec::Info);
    graph_init(zz, g, true);
    setEdgeType(zz, g, ET_SPLINE);
    let mut aspect = aspect_t::default();
    let asp = setAspect(zz, g, &mut aspect);
    dot_init_subg(zz, g, g);
    dot_init_node_edge(zz, g);
    dot_rank(zz, g, asp.as_ref());
}

/// `dotLayout` up to `dot_mincross`.
fn layout_until_mincross(r: &mut Replay) {
    layout_until_rank(r);
    // PlantUML sets no aspect ratio, so `dotLayout` never asks for balancing.
    dot_mincross(&mut r.zz, r.root, false);
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

/// Lays out every trace's graph with `run`, then compares the text it returns with `expected`'s.
fn check_all(run: fn(&mut Replay) -> String, expected: fn(&trace::Trace) -> String) {
    let traces = trace::files(&trace::repo_tests_dir().join("smetana"), "trace");
    assert!(!traces.is_empty(), "no traces");
    let mut failures = Vec::new();
    for path in &traces {
        let trace = trace::parse(&std::fs::read_to_string(path).unwrap());
        let want = expected(&trace);
        let got = catch_unwind(AssertUnwindSafe(|| {
            let mut replay = trace::replay(&trace.input);
            run(&mut replay)
        }));
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
            dot_position(&mut r.zz, r.root, None);
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

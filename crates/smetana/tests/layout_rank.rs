//! Lays out every Smetana trace's graph up to and including `dot_rank` and compares the ranks with Java's
//! (`phase rank`: each graph's and cluster's rank range, then every real node's rank).

mod trace;

use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};

use smetana::cgraph::id::agnameof;
use smetana::cgraph::node::{agfstnode, agnxtnode};
use smetana::cgraph::rec::{Rec, agbindrec};
use smetana::common::input::graph_init;
use smetana::common::utils::setEdgeType;
use smetana::core::Globals;
use smetana::core::consts::ET_SPLINE;
use smetana::core::ids::GraphId;
use smetana::dotgen::aspect::{aspect_t, setAspect};
use smetana::dotgen::dotinit::{dot_init_node_edge, dot_init_subg};
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

/// Lays out every trace's graph with `layout`, then compares `dump`'s text with `expected`'s.
fn check_all(
    layout: fn(&mut Replay),
    dump: fn(&mut Replay) -> String,
    expected: fn(&trace::Trace) -> String,
) {
    let traces = trace::files(&trace::repo_tests_dir().join("smetana"), "trace");
    assert!(!traces.is_empty(), "no traces");
    let mut failures = Vec::new();
    for path in &traces {
        let trace = trace::parse(&std::fs::read_to_string(path).unwrap());
        let want = expected(&trace);
        let got = catch_unwind(AssertUnwindSafe(|| {
            let mut replay = trace::replay(&trace.input);
            layout(&mut replay);
            dump(&mut replay)
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
    check_all(layout_until_rank, dump_ranks, expected_ranks);
}

#[test]
fn node_sizes_match_java() {
    check_all(layout_until_rank, dump_sizes, expected_sizes);
}

#[test]
fn label_sizes_match_java() {
    check_all(layout_until_rank, dump_label_sizes, expected_label_sizes);
}

//! Replays the input section of every Smetana trace through the port's cgraph and checks the graph against the
//! trace and against a dump of Java's cgraph after the same calls (`tests/smetana-cgraph`, made by
//! `tools/oracle/cgraph-dumps.sh`): counts, the iteration orders of nodes, edges and subgraphs, the order of
//! object ids, wildcard edge lookups, edges induced into subgraphs, membership and attribute values.

mod trace;

use std::fmt::Write as _;
use std::path::Path;

use smetana::cgraph::attr::agget;
use smetana::cgraph::edge::{
    agfindedge, agfstedge, agfstin, agfstout, agnxtedge, agnxtin, agnxtout, agsubedge,
};
use smetana::cgraph::graph::{agdegree, agnedges, agnnodes};
use smetana::cgraph::id::agnameof;
use smetana::cgraph::node::{agfstnode, agnxtnode};
use smetana::cgraph::obj::agcontains;
use smetana::cgraph::subg::{agfstsubg, agnxtsubg};
use smetana::cgraph::{AGINEDGE, Agobj, aghead, agtail};
use smetana::core::Globals;
use smetana::core::ids::{EdgeId, GraphId, NodeId};
use trace::{Call, Object, Replay};

fn name(zz: &Globals, obj: impl Into<Agobj>) -> String {
    trace::quote(&agnameof(zz, obj).expect("named object"))
}

fn edge(zz: &Globals, e: Option<EdgeId>) -> String {
    e.map_or_else(
        || "none".to_owned(),
        |e| {
            let tag = zz.tag(e);
            format!(
                "e{}{}",
                tag.seq,
                if tag.objtype == AGINEDGE { '<' } else { '>' }
            )
        },
    )
}

fn nodes(zz: &mut Globals, g: GraphId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut n = agfstnode(zz, g);
    while let Some(nn) = n {
        out.push(nn);
        n = agnxtnode(zz, g, nn);
    }
    out
}

/// The subgraphs below `g`, depth first in `agfstsubg` order.
fn subgraphs(zz: &mut Globals, g: GraphId, out: &mut Vec<GraphId>) {
    let mut sub = agfstsubg(zz, g);
    while let Some(s) = sub {
        out.push(s);
        subgraphs(zz, s, out);
        sub = agnxtsubg(zz, s);
    }
}

fn edge_lists(zz: &mut Globals, g: GraphId, n: NodeId) -> String {
    let mut line = String::from(" out");
    let mut e = agfstout(zz, g, n);
    while let Some(ee) = e {
        write!(line, " {}", edge(zz, Some(ee))).unwrap();
        e = agnxtout(zz, g, ee);
    }
    line.push_str(" in");
    let mut e = agfstin(zz, g, n);
    while let Some(ee) = e {
        write!(line, " {}", edge(zz, Some(ee))).unwrap();
        e = agnxtin(zz, g, ee);
    }
    line.push_str(" edges");
    let mut e = agfstedge(zz, g, n);
    while let Some(ee) = e {
        write!(line, " {}", edge(zz, Some(ee))).unwrap();
        e = agnxtedge(zz, g, ee, n);
    }
    let (din, dout) = (
        agdegree(zz, g, n, true, false),
        agdegree(zz, g, n, false, true),
    );
    write!(line, " degree {din} {dout}").unwrap();
    line
}

/// The dump `CgraphDump.java` writes; both make the same cgraph calls in the same order, since iterating and
/// searching reshape cgraph's splay trees.
fn dump(r: &mut Replay, input: &[Call]) -> String {
    let zz = &mut r.zz;
    let root = r.root;
    let mut out = String::from("cgraph-dump 1\n");
    let (nn, ne) = (agnnodes(zz, root), agnedges(zz, root));
    writeln!(out, "graph {} nodes {nn} edges {ne}", name(zz, root)).unwrap();
    let mut subs = Vec::new();
    subgraphs(zz, root, &mut subs);
    for &s in &subs {
        let parent = zz.graphs[s].parent.expect("subgraph parent");
        write!(
            out,
            "subgraph {} parent {} seq {} nodes",
            name(zz, s),
            name(zz, parent),
            zz.tag(s).seq
        )
        .unwrap();
        for n in nodes(zz, s) {
            write!(out, " {}", name(zz, n)).unwrap();
        }
        out.push('\n');
    }
    let mut graphs: Vec<GraphId> = std::iter::once(root).chain(subs.iter().copied()).collect();
    graphs.sort_by_key(|&g| zz.tag(g).id);
    out.push_str("graph ids");
    for g in graphs {
        write!(out, " {}", name(zz, g)).unwrap();
    }
    let mut by_id = nodes(zz, root);
    by_id.sort_by_key(|&n| zz.tag(n).id);
    out.push_str("\nnode ids");
    for n in by_id {
        write!(out, " {}", name(zz, n)).unwrap();
    }
    out.push('\n');
    for g in std::iter::once(root).chain(subs.iter().copied()) {
        for n in nodes(zz, g) {
            let lists = edge_lists(zz, g, n);
            writeln!(out, "in {} node {}{lists}", name(zz, g), name(zz, n)).unwrap();
        }
    }
    for &e in &r.edges {
        let (t, h) = (agtail(zz, e), aghead(zz, e));
        let (forward, backward) = (agfindedge(zz, root, t, h), agfindedge(zz, root, h, t));
        writeln!(
            out,
            "find {} forward {} backward {}",
            edge(zz, Some(e)),
            edge(zz, forward),
            edge(zz, backward)
        )
        .unwrap();
    }
    dump_membership(zz, root, &subs, &r.edges, &mut out);
    dump_attributes(r, input, &mut out);
    out
}

/// What rank's `node_induce` does to every subgraph, then which objects each contains.
fn dump_membership(
    zz: &mut Globals,
    root: GraphId,
    subs: &[GraphId],
    edges: &[EdgeId],
    out: &mut String,
) {
    for &s in subs {
        write!(out, "induce {}", name(zz, s)).unwrap();
        for n in nodes(zz, s) {
            let mut e = agfstout(zz, root, n);
            while let Some(ee) = e {
                if agcontains(zz, s, aghead(zz, ee)) {
                    let induced = agsubedge(zz, s, ee, true);
                    write!(out, " {}", edge(zz, induced)).unwrap();
                }
                e = agnxtout(zz, root, ee);
            }
        }
        out.push('\n');
        for n in nodes(zz, s) {
            let lists = edge_lists(zz, s, n);
            writeln!(out, "in {} node {}{lists}", name(zz, s), name(zz, n)).unwrap();
        }
    }
    for &s in subs {
        write!(out, "contains {}", name(zz, s)).unwrap();
        for n in nodes(zz, root) {
            out.push_str(if agcontains(zz, s, n) { " 1" } else { " 0" });
        }
        for &e in edges {
            out.push_str(if agcontains(zz, s, e) { " 1" } else { " 0" });
        }
        out.push('\n');
    }
}

/// The final value of every attribute the input set, in the order first set.
fn dump_attributes(r: &mut Replay, input: &[Call], out: &mut String) {
    let zz = &mut r.zz;
    let mut seen: Vec<(&Object, &String)> = Vec::new();
    for c in input {
        if let Call::Agsafeset { object, name, .. } = c
            && !seen.contains(&(object, name))
        {
            seen.push((object, name));
        }
    }
    for (object, attr) in seen {
        let obj = match object {
            Object::Graph(n) => Agobj::from(r.graphs[n]),
            Object::Node(n) => r.nodes[n].into(),
            Object::Edge(i) => r.edges[i - 1].into(),
        };
        let value =
            agget(zz, obj, attr).map_or_else(|| "null".to_owned(), |v| trace::quote(zz.agstr(v)));
        let label = match obj {
            Agobj::Edge(e) => edge(zz, Some(e)),
            other => name(zz, other),
        };
        writeln!(out, "attr {label} {} {value}", trace::quote(attr)).unwrap();
    }
}

/// The trace's own record of the layout's input, independent of the Java dump: every real node in creation order
/// (phase rank), the clusters depth first (phase rank, in `GD_clust` order, which follows `agfstsubg`), and the
/// edges numbered 1 to n.
fn check_against_trace(path: &Path, r: &mut Replay, trace: &trace::Trace) {
    let rank = &trace
        .phases
        .iter()
        .find(|(p, _)| p == "rank")
        .expect("phase rank")
        .1;
    let zz = &mut r.zz;
    let want_nodes: Vec<&str> = rank
        .iter()
        .filter_map(|l| l.strip_prefix("node ")?.rsplit_once(" rank "))
        .map(|(n, _)| n)
        .collect();
    let have_nodes: Vec<String> = nodes(zz, r.root).into_iter().map(|n| name(zz, n)).collect();
    assert_eq!(have_nodes, want_nodes, "{}: node order", path.display());

    let want_graphs: Vec<&str> = rank
        .iter()
        .filter_map(|l| l.strip_prefix("graph ")?.split_once(" minrank "))
        .map(|(g, _)| g)
        .collect();
    let mut subs = Vec::new();
    subgraphs(zz, r.root, &mut subs);
    let mut have_graphs = vec![name(zz, r.root)];
    have_graphs.extend(
        subs.into_iter()
            .map(|s| name(zz, s))
            .filter(|n| n.starts_with("\"cluster")),
    );
    assert_eq!(
        have_graphs,
        want_graphs,
        "{}: cluster order",
        path.display()
    );

    let mut seqs: Vec<i32> = Vec::new();
    for n in nodes(zz, r.root) {
        let mut e = agfstout(zz, r.root, n);
        while let Some(ee) = e {
            seqs.push(zz.tag(ee).seq);
            e = agnxtout(zz, r.root, ee);
        }
    }
    seqs.sort_unstable();
    let want: Vec<i32> = (1..=i32::try_from(r.edges.len()).unwrap()).collect();
    assert_eq!(seqs, want, "{}: edges", path.display());
}

#[test]
fn every_trace_replays_like_java() {
    let traces = trace::files(&trace::repo_tests_dir().join("smetana"), "trace");
    assert!(!traces.is_empty(), "no traces");
    for path in traces {
        let text = std::fs::read_to_string(&path).unwrap();
        let trace = trace::parse(&text);
        let mut replay = trace::replay(&trace.input);
        check_against_trace(&path, &mut replay, &trace);
        let mut replay = trace::replay(&trace.input);
        compare_with_java_dump(
            &path,
            &trace::repo_tests_dir().join("smetana"),
            &mut replay,
            &trace.input,
        );
    }
}

/// Synthetic inputs that exercise what the corpus does not: multi-edges both ways, loops, edges made in
/// subgraphs, nodes added to subgraphs later, local attribute defaults, and names interned (or freed) before the
/// objects they name.
#[test]
fn synthetic_inputs_replay_like_java() {
    let dir = trace::repo_tests_dir()
        .join("smetana-cgraph")
        .join("synthetic");
    let inputs = trace::files(&dir, "trace");
    assert!(!inputs.is_empty(), "no synthetic inputs");
    for path in inputs {
        let trace = trace::parse(&std::fs::read_to_string(&path).unwrap());
        let mut replay = trace::replay(&trace.input);
        compare_with_java_dump(&path, &dir, &mut replay, &trace.input);
    }
}

fn compare_with_java_dump(path: &Path, traces_root: &Path, r: &mut Replay, input: &[Call]) {
    let relative = path
        .strip_prefix(traces_root)
        .expect("trace below its root");
    let dumps_root = trace::repo_tests_dir().join("smetana-cgraph");
    let dump_path = if traces_root.starts_with(&dumps_root) {
        path.with_extension("dump")
    } else {
        dumps_root.join(relative).with_extension("dump")
    };
    let want = std::fs::read_to_string(&dump_path).unwrap_or_else(|e| {
        panic!(
            "{}: {e} (run tools/oracle/cgraph-dumps.sh)",
            dump_path.display()
        )
    });
    let have = dump(r, input);
    if have != want {
        let line = have
            .lines()
            .zip(want.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        panic!(
            "{}: cgraph differs from Java at dump line {}\n  rust: {}\n  java: {}",
            path.display(),
            line + 1,
            have.lines().nth(line).unwrap_or("<end>"),
            want.lines().nth(line).unwrap_or("<end>")
        );
    }
}

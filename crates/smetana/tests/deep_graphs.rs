//! Large PlantUML-shaped graphs lay out on a 1 MB stack, wasm's default and close to the Windows main thread's,
//! even in a debug build: dot's depth-first searches (acyclic, decomposition, network simplex, flat edge order)
//! go as deep as the graph.

use std::thread;
use std::time::Instant;

use smetana::{Drawing, Graph, Node, Subgraph};

const STACK: usize = 1 << 20;
const NODES: usize = 2000;

/// A class box as PlantUML adds it.
fn class(graph: &mut Graph, parent: Subgraph, i: usize) -> Node {
    let n = graph.node(parent, &format!("sh{i:04}"));
    graph.set(n, "shape", "box");
    graph.set(n, "width", "1.2");
    graph.set(n, "height", "0.6666666666666666");
    n
}

/// A link as PlantUML adds it: `minlen` is the arrow's length less one.
fn link(graph: &mut Graph, tail: Node, head: Node, minlen: &str) {
    let e = graph.edge(graph.root(), tail, head);
    graph.set(e, "arrowtail", "none");
    graph.set(e, "arrowhead", "none");
    graph.set(e, "minlen", minlen);
}

/// Builds a graph with `build` and lays it out on a thread with a 1 MB stack, reporting the time it took.
fn lay_out_on_small_stack(name: &'static str, build: fn(&mut Graph)) -> Drawing {
    let drawing = thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || {
            let mut graph = Graph::new();
            graph.set(graph.root(), "margin", "16");
            build(&mut graph);
            graph.gv_context();
            let start = Instant::now();
            let drawing = graph.layout();
            eprintln!("{name}: laid out in {:?}", start.elapsed());
            drawing
        })
        .expect("thread")
        .join()
        .unwrap_or_else(|_| panic!("{name}: the layout panicked"));
    drawing.unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Every node centre and spline point is a number.
fn assert_sane(drawing: &Drawing) {
    let finite = |p: &smetana::Point| p.x.is_finite() && p.y.is_finite();
    assert!(drawing.nodes().iter().all(|n| finite(&n.center)));
    for e in drawing.edges() {
        assert!(!e.beziers.is_empty(), "an edge without spline");
        assert!(e.beziers.iter().flat_map(|b| &b.points).all(finite));
    }
}

/// `a1 --> a2 --> ... --> a2000`: one rank per node.
#[test]
fn a_long_chain() {
    let drawing = lay_out_on_small_stack("chain", |graph| {
        let root = graph.root();
        let nodes: Vec<Node> = (0..NODES).map(|i| class(graph, root, i)).collect();
        for pair in nodes.windows(2) {
            link(graph, pair[0], pair[1], "1");
        }
    });
    assert_sane(&drawing);
}

/// `a1 - a2 - ... - a2000`: one rank, held together by flat edges.
#[test]
fn a_long_flat_chain() {
    let drawing = lay_out_on_small_stack("flat chain", |graph| {
        let root = graph.root();
        let nodes: Vec<Node> = (0..NODES).map(|i| class(graph, root, i)).collect();
        for pair in nodes.windows(2) {
            link(graph, pair[0], pair[1], "0");
        }
    });
    assert_sane(&drawing);
}

/// A tree of 2000 classes, each with up to three children.
#[test]
fn a_wide_tree() {
    let drawing = lay_out_on_small_stack("tree", |graph| {
        let root = graph.root();
        let nodes: Vec<Node> = (0..NODES).map(|i| class(graph, root, i)).collect();
        for (i, &child) in nodes.iter().enumerate().skip(1) {
            link(graph, nodes[(i - 1) / 3], child, "1");
        }
    });
    assert_sane(&drawing);
}

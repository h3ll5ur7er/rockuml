//! Smetana layout traces (`tests/smetana/**/NN.trace`, format in `tools/oracle/README.md`): parsing, and replaying
//! their input section through the port's cgraph.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use smetana::cgraph::attr::{agattr, agsafeset};
use smetana::cgraph::edge::agedge;
use smetana::cgraph::graph::agopen;
use smetana::cgraph::node::agnode;
use smetana::cgraph::subg::agsubg;
use smetana::cgraph::{AGNODE, Agobj};
use smetana::core::Globals;
use smetana::core::ids::{EdgeId, GraphId, NodeId};
use smetana::h::cgraph::Agdirected;

/// A graph, node or edge as a trace names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Object {
    Graph(String),
    Node(String),
    /// The edge made by the N-th `agedge` call, counting from 1.
    Edge(usize),
}

/// A call of the input section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Call {
    Agopen(String),
    Agsubg {
        graph: String,
        name: String,
    },
    Agnode {
        graph: String,
        name: String,
    },
    Agedge {
        graph: String,
        tail: String,
        head: String,
        name: Option<String>,
    },
    Agsafeset {
        object: Object,
        name: String,
        value: String,
        def: String,
    },
    GvContext,
    GvLayoutJobs(String),
}

/// A parsed trace: the input calls, then each phase section's lines, unparsed.
#[derive(Debug)]
pub(crate) struct Trace {
    pub(crate) input: Vec<Call>,
    pub(crate) phases: Vec<(String, Vec<String>)>,
}

/// Every trace (and every other file with `extension`) under `dir`, sorted.
pub(crate) fn files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display())) {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == extension) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The `tests/` directory at the root of the repository.
pub(crate) fn repo_tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

/// Splits a line into tokens: bare words, `"quoted strings"` (unescaped) and `dim(W,H)` (expanded to PlantUML's
/// `_dim_W_H_` label).
pub(crate) fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next().expect("unterminated string") {
                    '"' => break,
                    '\\' => s.push(match chars.next().expect("escape") {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    }),
                    other => s.push(other),
                }
            }
            out.push(s);
        } else {
            let mut word = String::new();
            while let Some(&c) = chars.peek()
                && c != ' '
            {
                word.push(c);
                chars.next();
            }
            if let Some(dim) = word.strip_prefix("dim(").and_then(|d| d.strip_suffix(')')) {
                let (w, h) = dim.split_once(',').expect("dim(W,H)");
                word = format!("_dim_{w}_{h}_");
            }
            out.push(word);
        }
    }
    out
}

/// Quotes a string as the trace writer does.
pub(crate) fn quote(text: &str) -> String {
    let mut quoted = String::from('"');
    for c in text.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

fn object(kind: &str, name: &str) -> Object {
    match kind {
        "graph" => Object::Graph(name.to_owned()),
        "node" => Object::Node(name.to_owned()),
        "edge" => Object::Edge(
            name.strip_prefix('e')
                .and_then(|n| n.parse().ok())
                .expect("edge eN"),
        ),
        _ => panic!("unknown object kind {kind}"),
    }
}

fn call(t: &[String]) -> Call {
    let s = |i: usize| t[i].clone();
    match (t[0].as_str(), t.len()) {
        ("agopen", 2) => Call::Agopen(s(1)),
        ("agsubg", 4) => Call::Agsubg {
            graph: s(2),
            name: s(3),
        },
        ("agnode", 4) => Call::Agnode {
            graph: s(2),
            name: s(3),
        },
        ("agedge", 7 | 8) => Call::Agedge {
            graph: s(2),
            tail: s(4),
            head: s(6),
            name: t.get(7).cloned(),
        },
        ("agsafeset", 6) => Call::Agsafeset {
            object: object(&t[1], &t[2]),
            name: s(3),
            value: s(4),
            def: s(5),
        },
        ("gvContext", 1) => Call::GvContext,
        ("gvLayoutJobs", 3) => Call::GvLayoutJobs(s(2)),
        _ => panic!("unknown input line {t:?}"),
    }
}

pub(crate) fn parse(text: &str) -> Trace {
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("smetana-trace 1"),
        "not a version 1 trace"
    );
    let mut trace = Trace {
        input: Vec::new(),
        phases: Vec::new(),
    };
    for line in lines {
        if let Some(phase) = line.strip_prefix("phase ") {
            trace.phases.push((phase.to_owned(), Vec::new()));
        } else if let Some((_, section)) = trace.phases.last_mut() {
            section.push(line.to_owned());
        } else {
            trace.input.push(call(&tokens(line)));
        }
    }
    trace
}

/// The graph built from a trace's input section, with its objects by trace name.
pub(crate) struct Replay {
    pub(crate) zz: Globals,
    pub(crate) root: GraphId,
    pub(crate) graphs: HashMap<String, GraphId>,
    pub(crate) nodes: HashMap<String, NodeId>,
    /// Edges by creation number (index 0 is edge `e1`).
    pub(crate) edges: Vec<EdgeId>,
}

impl Replay {
    pub(crate) fn object(&self, o: &Object) -> Agobj {
        match o {
            Object::Graph(name) => self.graphs[name].into(),
            Object::Node(name) => self.nodes[name].into(),
            Object::Edge(n) => self.edges[n - 1].into(),
        }
    }
}

/// Makes the calls of the input section, up to `gvLayoutJobs`, as PlantUML made them.
pub(crate) fn replay(input: &[Call]) -> Replay {
    let mut zz = Globals::open();
    let Some(Call::Agopen(name)) = input.first() else {
        panic!("a trace starts with agopen")
    };
    let root = agopen(&mut zz, Some(name), Agdirected);
    let mut r = Replay {
        zz,
        root,
        graphs: HashMap::from([(name.clone(), root)]),
        nodes: HashMap::new(),
        edges: vec![],
    };
    for c in &input[1..] {
        match c {
            Call::Agopen(_) => panic!("a second agopen"),
            Call::Agsubg { graph, name } => {
                let subg = agsubg(&mut r.zz, r.graphs[graph], Some(name), true).expect("subgraph");
                r.graphs.insert(name.clone(), subg);
            }
            Call::Agnode { graph, name } => {
                let n = agnode(&mut r.zz, r.graphs[graph], Some(name), true).expect("node");
                r.nodes.insert(name.clone(), n);
            }
            Call::Agedge {
                graph,
                tail,
                head,
                name,
            } => {
                let (g, t, h) = (r.graphs[graph], r.nodes[tail], r.nodes[head]);
                let e = agedge(&mut r.zz, g, t, h, name.as_deref(), true).expect("edge");
                r.edges.push(e);
            }
            Call::Agsafeset {
                object,
                name,
                value,
                def,
            } => {
                let obj = r.object(object);
                agsafeset(&mut r.zz, obj, name, value, def);
            }
            // gvContext declares the default node label on the prototype graph.
            Call::GvContext => {
                agattr(&mut r.zz, None, AGNODE, "label", Some("\\N"));
            }
            Call::GvLayoutJobs(_) => break,
        }
    }
    r
}

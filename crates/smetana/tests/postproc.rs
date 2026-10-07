//! Post-processes every traced Smetana layout and compares the result with the trace's `phase final` section.
//!
//! The graph comes from the trace's input section, the edges' splines and labels from its `phase splines` section
//! (dot's state right before post-processing), and what the trace does not record (graph and node records, `State`)
//! from data/postproc.txt, which tools/oracle/smetana-unit/postproc.sh dumps from Java replays of the same traces.

mod trace;

use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};

use smetana::cgraph::node::{agfstnode, agnxtnode};
use smetana::common::postproc::dotneato_postprocess;
use smetana::core::Globals;
use smetana::core::ids::{EdgeId, GraphId, NodeId, TextlabelId};
use smetana::h::{bezier, boxf, pointf, splines, textlabel_t};
use trace::Replay;

const FIXTURE: &str = include_str!("data/postproc.txt");

fn number(s: &str) -> f64 {
    s.parse().unwrap_or_else(|_| panic!("number {s}"))
}

fn int(s: &str) -> i32 {
    s.parse().unwrap_or_else(|_| panic!("int {s}"))
}

fn point(t: &[String]) -> pointf {
    pointf {
        x: number(&t[0]),
        y: number(&t[1]),
    }
}

/// `pos X Y dimen W H set S`, as a new text label.
fn label(zz: &mut Globals, t: &[String]) -> TextlabelId {
    assert_eq!(
        (t[0].as_str(), t[3].as_str(), t[6].as_str()),
        ("pos", "dimen", "set")
    );
    zz.textlabels.push(textlabel_t {
        pos: point(&t[1..3]),
        dimen: point(&t[4..6]),
        set: int(&t[7]),
        ..textlabel_t::default()
    })
}

fn edge(r: &Replay, name: &str) -> EdgeId {
    let n: usize = name
        .strip_prefix('e')
        .and_then(|n| n.parse().ok())
        .expect("eN");
    r.edges[n - 1]
}

/// Fills the records from the fixture's lines for one trace.
fn load_records(r: &mut Replay, lines: &[&str]) {
    for line in lines {
        let t = trace::tokens(line);
        match (t[0].as_str(), t[2].as_str()) {
            ("state", _) => {
                r.zz.State = int(&t[2]);
                r.zz.EdgeLabelsDone = int(&t[4]);
            }
            ("graph", "label") => {
                let l = label(&mut r.zz, &t[3..]);
                r.zz.gd_mut(r.graphs[&t[1]]).label = Some(l);
            }
            ("graph", "rankdir") => {
                let g = r.graphs[&t[1]];
                let clusters: Vec<GraphId> = t[25..].iter().map(|c| r.graphs[c]).collect();
                let n_cluster = clusters.len() as i32;
                let clust = r.zz.graph_lists.ALLOC(n_cluster + 1);
                for (c, &sub) in clusters.iter().enumerate() {
                    r.zz.graph_lists.set(clust, c as i32 + 1, Some(sub));
                }
                let gd = r.zz.gd_mut(g);
                gd.rankdir = int(&t[3]);
                gd.flags = int(&t[5]);
                gd.has_labels = int(&t[7]);
                gd.label_pos = int(&t[9]);
                gd.bb = boxf {
                    LL: point(&t[11..13]),
                    UR: point(&t[13..15]),
                };
                for (i, border) in gd.border.iter_mut().enumerate() {
                    *border = point(&t[16 + 2 * i..18 + 2 * i]);
                }
                assert_eq!(t[24], "clusters");
                gd.n_cluster = n_cluster;
                gd.clust = Some(clust);
            }
            ("node", _) => {
                let nd = r.zz.nd_mut(r.nodes[&t[1]]);
                nd.coord = point(&t[3..5]);
                nd.width = number(&t[6]);
                nd.height = number(&t[8]);
                nd.lw = number(&t[10]);
                nd.rw = number(&t[12]);
                nd.ht = number(&t[14]);
            }
            ("edge", "edge_type") => {
                let e = edge(r, &t[1]);
                r.zz.ed_mut(e).edge_type = int(&t[3]);
            }
            _ => panic!("unknown fixture line {line}"),
        }
    }
}

/// Fills the edges' splines and labels from the trace's `phase splines` section.
fn load_splines(r: &mut Replay, lines: &[String]) {
    let mut beziers: HashMap<EdgeId, Vec<bezier>> = HashMap::new();
    for line in lines {
        let t = trace::tokens(line);
        let e = edge(r, &t[1]);
        match t[2].as_str() {
            "spl" => assert_eq!(t[3], "none"),
            "bezier" => {
                let size = int(&t[15]);
                let list = r.zz.pointfs.ALLOC(size);
                for i in 0..size as usize {
                    r.zz.pointfs
                        .set(list, i as i32, point(&t[16 + 2 * i..18 + 2 * i]));
                }
                let bz = bezier {
                    list: Some(list),
                    size,
                    sflag: int(&t[5]),
                    eflag: int(&t[7]),
                    sp: point(&t[9..11]),
                    ep: point(&t[12..14]),
                };
                let spline = beziers.entry(e).or_default();
                assert_eq!(spline.len().to_string(), t[3], "beziers in order");
                spline.push(bz);
            }
            kind => {
                let l = Some(label(&mut r.zz, &t[3..]));
                let ed = r.zz.ed_mut(e);
                match kind {
                    "label" => ed.label = l,
                    "head_label" => ed.head_label = l,
                    "tail_label" => ed.tail_label = l,
                    "xlabel" => ed.xlabel = l,
                    _ => panic!("unknown splines line {line}"),
                }
            }
        }
    }
    for e in r.edges.clone() {
        if let Some(bzs) = beziers.remove(&e) {
            let size = bzs.len() as i32;
            let list = r.zz.beziers.ALLOC(size);
            r.zz.beziers.copy_from(list, &bzs);
            let spl = r.zz.splines.push(splines {
                list: Some(list),
                size,
            });
            r.zz.ed_mut(e).spl = Some(spl);
        }
    }
}

fn pt(p: pointf) -> String {
    format!("{:?} {:?}", p.x, p.y)
}

fn label_line(
    out: &mut Vec<String>,
    zz: &Globals,
    prefix: &str,
    kind: &str,
    l: Option<TextlabelId>,
) {
    if let Some(l) = l {
        let l = &zz.textlabels[l];
        out.push(format!(
            "{prefix} {kind} pos {} dimen {} set {}",
            pt(l.pos),
            pt(l.dimen),
            l.set
        ));
    }
}

/// The state as the trace's `phase final` section writes it.
fn final_lines(r: &mut Replay) -> Vec<String> {
    let graph_names: HashMap<GraphId, &String> = r.graphs.iter().map(|(n, &g)| (g, n)).collect();
    let node_names: HashMap<NodeId, &String> = r.nodes.iter().map(|(n, &v)| (v, n)).collect();
    let zz = &mut r.zz;
    let mut out = Vec::new();
    let mut stack = vec![r.root];
    while let Some(g) = stack.pop() {
        let gd = *zz.gd(g);
        let prefix = format!("graph {}", trace::quote(graph_names[&g]));
        out.push(format!("{prefix} bb {} {}", pt(gd.bb.LL), pt(gd.bb.UR)));
        label_line(&mut out, zz, &prefix, "label", gd.label);
        for c in (1..=gd.n_cluster).rev() {
            stack.push(zz.graph_lists.get(gd.clust.unwrap(), c).unwrap());
        }
    }
    let mut n = agfstnode(zz, r.root);
    while let Some(v) = n {
        let nd = zz.nd(v);
        out.push(format!(
            "node {} coord {} width {:?} height {:?} lw {:?} rw {:?} ht {:?}",
            trace::quote(node_names[&v]),
            pt(nd.coord),
            nd.width,
            nd.height,
            nd.lw,
            nd.rw,
            nd.ht
        ));
        n = agnxtnode(zz, r.root, v);
    }
    for (i, &e) in r.edges.iter().enumerate() {
        let prefix = format!("edge e{}", i + 1);
        let ed = *zz.ed(e);
        match ed.spl {
            None => out.push(format!("{prefix} spl none")),
            Some(spl) => {
                let spl = zz.splines[spl];
                for j in 0..spl.size {
                    let bz = zz.beziers.get(spl.list.unwrap(), j);
                    let mut line = format!(
                        "{prefix} bezier {j} sflag {} eflag {} sp {} ep {} points {}",
                        bz.sflag,
                        bz.eflag,
                        pt(bz.sp),
                        pt(bz.ep),
                        bz.size
                    );
                    for k in 0..bz.size {
                        line.push(' ');
                        line.push_str(&pt(zz.pointfs.get(bz.list.unwrap(), k)));
                    }
                    out.push(line);
                }
            }
        }
        label_line(&mut out, zz, &prefix, "label", ed.label);
        label_line(&mut out, zz, &prefix, "head_label", ed.head_label);
        label_line(&mut out, zz, &prefix, "tail_label", ed.tail_label);
        label_line(&mut out, zz, &prefix, "xlabel", ed.xlabel);
    }
    out
}

/// Whether two lines say the same, numbers compared by value (Rust and Java print doubles differently).
fn same_line(actual: &str, expected: &str) -> bool {
    let (a, e) = (trace::tokens(actual), trace::tokens(expected));
    a.len() == e.len()
        && a.iter()
            .zip(&e)
            .all(|(a, e)| match (a.parse::<f64>(), e.parse::<f64>()) {
                (Ok(a), Ok(e)) => a.to_bits() == e.to_bits(),
                _ => a == e,
            })
}

/// Post-processes one trace; the first difference from its final section, if any.
fn check(path: &str, records: &[&str]) -> Option<String> {
    let file = trace::repo_tests_dir().join("smetana").join(path);
    let text = std::fs::read_to_string(&file).expect("trace");
    let parsed = trace::parse(&text);
    let section = |name: &str| {
        &parsed
            .phases
            .iter()
            .find(|(phase, _)| phase == name)
            .unwrap_or_else(|| panic!("{path}: no phase {name}"))
            .1
    };
    let mut r = trace::replay(&parsed.input);
    load_records(&mut r, records);
    load_splines(&mut r, section("splines"));
    dotneato_postprocess(&mut r.zz, r.root);
    let actual = final_lines(&mut r);
    let expected = section("final");
    for (i, e) in expected.iter().enumerate() {
        match actual.get(i) {
            Some(a) if same_line(a, e) => {}
            a => {
                return Some(format!(
                    "{path}: line {i}\n  expected {e}\n  actual   {a:?}"
                ));
            }
        }
    }
    (actual.len() != expected.len())
        .then(|| format!("{path}: {} extra lines", actual.len() - expected.len()))
}

#[test]
fn postprocessing_reproduces_every_traced_layout() {
    let mut cases: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in FIXTURE.lines().filter(|l| !l.starts_with('#')) {
        match line.strip_prefix("trace ") {
            Some(path) => cases.push((path, Vec::new())),
            None => cases.last_mut().expect("trace header").1.push(line),
        }
    }
    let dir = trace::repo_tests_dir().join("smetana");
    let mut traces: Vec<String> = trace::files(&dir, "trace")
        .iter()
        .map(|f| {
            f.strip_prefix(&dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    let mut covered: Vec<String> = cases.iter().map(|(p, _)| (*p).to_owned()).collect();
    traces.sort();
    covered.sort();
    assert_eq!(
        covered, traces,
        "data/postproc.txt covers every trace; rerun postproc.sh"
    );

    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(path, records)| {
            panic::catch_unwind(AssertUnwindSafe(|| check(path, records)))
                .unwrap_or_else(|_| Some(format!("{path}: panicked")))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} traces differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

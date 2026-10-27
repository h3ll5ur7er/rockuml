//! The Smetana graphs legacy activity diagrams make, against the traces PlantUML's layouts write
//! (`tests/smetana/legacy-activity/<case>/NN.trace`).

use std::path::Path;

use super::tests::parse_source;
use crate::klimt::debug::StringBounderDebug;
use crate::sdot::recorded_graphs;

fn repository_tests() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests"))
}

/// The calls of each trace's input section, up to the layout, in the order PlantUML laid the graphs out.
fn traced_inputs(case: &str) -> Vec<Vec<String>> {
    let directory = repository_tests().join(format!("smetana/legacy-activity/{case}"));
    let mut traces: Vec<_> = std::fs::read_dir(directory)
        .expect("Smetana traces")
        .map(|entry| entry.unwrap().path())
        .collect();
    traces.sort();
    traces
        .into_iter()
        .map(|trace| {
            std::fs::read_to_string(trace)
                .unwrap()
                .lines()
                .skip(1)
                .take_while(|line| !line.starts_with("gvLayoutJobs"))
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

#[test]
fn every_layout_makes_the_graph_plantuml_lays_out() {
    let corpus = repository_tests().join("corpus/legacy-activity");
    let mut cases: Vec<_> = std::fs::read_dir(&corpus)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "puml")
        })
        .collect();
    cases.sort();
    assert_eq!(cases.len(), 22);
    for path in cases {
        let case = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let diagram = parse_source(&lines).unwrap_or_else(|error| panic!("{case}: {error}"));
        recorded_graphs::start();
        let drawn = diagram.cuca.get_text_block(&StringBounderDebug);
        let graphs = recorded_graphs::take();
        assert!(drawn.is_ok(), "{case} is drawn");
        assert_eq!(graphs, traced_inputs(&case), "{case}");
    }
}

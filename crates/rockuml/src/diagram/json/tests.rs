//! The Smetana graphs JSON and YAML documents make, against the traces PlantUML's layouts write
//! (`tests/smetana/{json,yaml}/<case>/NN.trace`).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::host::IsolatedHost;
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::font::StringBounder;
use crate::preproc::{PreprocessorEnvironment, Source};
use crate::sdot::recorded_graphs;

fn repository_tests() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests"))
}

fn sorted_entries(directory: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(directory)
        .expect("a directory of the repository's tests")
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    entries
}

/// The calls of a trace's input section, up to the layout.
fn traced_input(trace: &Path) -> Vec<String> {
    std::fs::read_to_string(trace)
        .unwrap()
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with("gvLayoutJobs"))
        .map(str::to_owned)
        .collect()
}

/// The graph rockuml lays the corpus case out with.
fn graph_of(family: &str, case: &str) -> Vec<Vec<String>> {
    let path = repository_tests().join(format!("corpus/{family}/{case}.puml"));
    let text = std::fs::read_to_string(&path).unwrap();
    let source = Source {
        text: &text,
        description: case,
        directory: PathBuf::new(),
        environment: PreprocessorEnvironment::default(),
    };
    let block = crate::preproc::preprocess(&source, &IsolatedHost).remove(0);
    let diagram = crate::diagram::create(&block, &IsolatedHost).unwrap();
    let string_bounder: Rc<dyn StringBounder> = Rc::new(StringBounderDebug);
    recorded_graphs::start();
    assert!(
        diagram.text_block(0, &string_bounder).is_ok(),
        "{case} is drawn"
    );
    recorded_graphs::take()
}

/// PlantUML lays a document out twice, to measure it and to draw it; rockuml once.
#[test]
fn every_document_makes_the_graph_plantuml_lays_out() {
    let mut cases = 0;
    for family in ["json", "yaml"] {
        for directory in sorted_entries(&repository_tests().join(format!("smetana/{family}"))) {
            let case = directory.file_name().unwrap().to_str().unwrap().to_owned();
            let graphs = graph_of(family, &case);
            assert_eq!(graphs.len(), 1, "{family}/{case}: one layout");
            for trace in sorted_entries(&directory) {
                assert_eq!(
                    graphs[0],
                    traced_input(&trace),
                    "{family}/{case}: {trace:?}"
                );
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 16);
}

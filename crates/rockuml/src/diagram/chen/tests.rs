use std::path::Path;

use super::*;
use crate::command::factory::{self, Created};
use crate::klimt::debug::StringBounderDebug;
use crate::sdot::CucaDiagramFileMakerSmetana;
use crate::text::{LineLocation, StringLocated};

const CASES: [&str; 5] = [
    "attributes",
    "entities-relationship",
    "relationship-attributes",
    "university",
    "weak-entity",
];

fn repository_tests() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests"))
}

fn read_diagram(case: &str) -> ChenEerDiagram {
    let text = std::fs::read_to_string(repository_tests().join(format!("corpus/chen/{case}.puml")))
        .expect("corpus case");
    let location = LineLocation::new(case, None);
    let lines: Vec<StringLocated> = text
        .lines()
        .map(|line| StringLocated::new(line, location.clone()))
        .collect();
    let source = Rc::new(UmlSource::new(lines, Vec::new()));
    let commands = ChenEerDiagramFactory::init_commands_list();
    match factory::create_system(
        &source,
        || ChenEerDiagramFactory::create_empty_diagram(&source),
        &commands,
    ) {
        Created::Diagram(diagram) => diagram,
        Created::Failure(_) | Created::Nothing => panic!("{case} reads as a Chen diagram"),
    }
}

/// The calls of the trace's input section, up to the layout.
fn traced_input(case: &str) -> Vec<String> {
    let trace =
        std::fs::read_to_string(repository_tests().join(format!("smetana/chen/{case}/01.trace")))
            .expect("Smetana trace");
    trace
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with("gvLayoutJobs"))
        .map(str::to_owned)
        .collect()
}

/// The bridge makes the graph PlantUML makes, call for call, value for value.
#[test]
fn the_smetana_graph_is_the_one_plantuml_lays_out() {
    for case in CASES {
        let diagram = read_diagram(case);
        let mut cuca = diagram.cuca.clone();
        cuca.eventually_build_phantom_groups(None);
        let calls = CucaDiagramFileMakerSmetana::new(cuca)
            .smetana_calls(&StringBounderDebug)
            .expect("Chen diagrams are drawn");
        assert_eq!(calls, traced_input(case), "{case}");
    }
}

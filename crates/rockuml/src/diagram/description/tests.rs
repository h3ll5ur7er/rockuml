//! The description corpus read by the commands, against what PlantUML's commands make of it. The fixture
//! comes from `tools/oracle/cuca-unit/DescriptionDump.java`.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::rc::Rc;

use super::{DescriptionDiagram, DescriptionDiagramFactory};
use crate::abel::Entity;
use crate::color::{ColorType, Colors};
use crate::command::factory::{Created, create_system};
use crate::creole::Display;
use crate::decoration::symbol::{USymbol, USymbols};
use crate::diagram::builder::CommandFactory;
use crate::host::IsolatedHost;
use crate::preproc::{PreprocessorEnvironment, Source, preprocess};

const FIXTURE: &str = include_str!("../../../tests/data/description.txt");

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus");

/// `USymbolDump`'s order, which is the order of PlantUML's `USymbols` fields.
const SYMBOL_CODES: [&str; 36] = [
    "ACTION",
    "ACTOR_AWESOME",
    "ACTOR_HOLLOW",
    "ACTOR_STICKMAN",
    "ACTOR_STICKMAN_BUSINESS",
    "AGENT",
    "ARCHIMATE",
    "ARTIFACT",
    "BOUNDARY",
    "CARD",
    "CLOUD",
    "COLLECTIONS",
    "COMPONENT_RECTANGLE",
    "COMPONENT1",
    "COMPONENT2",
    "CONTROL",
    "DATABASE",
    "ENTITY_DOMAIN",
    "FILE",
    "FOLDER",
    "FRAME",
    "GROUP",
    "HEXAGON",
    "INTERFACE",
    "LABEL",
    "NODE",
    "PACKAGE",
    "PARTITION",
    "PERSON",
    "PROCESS",
    "QUEUE",
    "RECTANGLE",
    "STACK",
    "STORAGE",
    "USECASE",
    "USECASE_BUSINESS",
];

/// The fixture's sections by case, each split into the model and the images.
fn fixture_cases() -> BTreeMap<&'static str, (String, String)> {
    let mut cases = BTreeMap::new();
    for case in FIXTURE.split("\n=== ").skip(1) {
        let (header, body) = case.split_once('\n').unwrap();
        let (model, images) = match body.find("\nimage ") {
            Some(at) => (&body[..=at], &body[at + 1..]),
            None => (body, ""),
        };
        cases.insert(header, (model.to_owned(), images.to_owned()));
    }
    cases
}

/// The diagram the description commands make of a corpus case.
pub(super) fn read(case: &str) -> DescriptionDiagram {
    let path = PathBuf::from(CORPUS).join(case);
    let text = std::fs::read_to_string(&path).unwrap();
    let source = Source {
        text: &text,
        description: case,
        directory: path.parent().unwrap().to_owned(),
        environment: PreprocessorEnvironment::default(),
    };
    let block = preprocess(&source, &IsolatedHost).remove(0);
    let source = Rc::new(crate::diagram::prepare(&block).1);
    let commands = DescriptionDiagramFactory::init_commands_list();
    match create_system(
        &source,
        || DescriptionDiagramFactory::create_empty_diagram(&source),
        &commands,
    ) {
        Created::Diagram(diagram) => diagram,
        Created::Failure(failure) => panic!("{case}: {failure:?}"),
        Created::Nothing => panic!("{case}: no diagram"),
    }
}

/// Java's name of an enum constant Rust names `CamelCase`.
fn screaming(debug: impl std::fmt::Debug) -> String {
    let mut result = String::new();
    for (index, c) in format!("{debug:?}").chars().enumerate() {
        if c.is_ascii_uppercase() && index > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_uppercase());
    }
    result
}

fn quoted(text: Option<&str>) -> String {
    text.map_or_else(
        || "null".to_owned(),
        |text| {
            format!(
                "\"{}\"",
                text.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
            )
        },
    )
}

fn symbol(symbol: Option<USymbol>) -> &'static str {
    symbol.map_or("null", |symbol| {
        SYMBOL_CODES
            .into_iter()
            .find(|code| USymbols::by_code(code) == Some(symbol))
            .unwrap()
    })
}

fn lines(display: &Display) -> String {
    display.lines().join("\n")
}

fn colors(colors: &Colors) -> String {
    let mut result = String::new();
    for kind in [
        ColorType::Text,
        ColorType::Line,
        ColorType::Back,
        ColorType::Header,
        ColorType::Arrow,
    ] {
        if let Some(color) = colors.get(kind) {
            write!(result, " {}={}", screaming(kind), color.as_string()).unwrap();
        }
    }
    if let Some(stroke) = colors.get_specific_line_stroke() {
        write!(result, " stroke={stroke}").unwrap();
    }
    result
}

fn describe(entity: &Entity) -> String {
    let mut result = format!("display={}", quoted(Some(&lines(&entity.display))));
    if let Some(stereotype) = &entity.stereotype {
        write!(result, " stereo={}", quoted(Some(&stereotype.to_string()))).unwrap();
    }
    result += &colors(&entity.colors);
    if let Some(url) = &entity.url {
        write!(result, " url={}", quoted(Some(&url.href))).unwrap();
    }
    result
}

/// The model as `DescriptionDump` writes it.
fn dump_model(diagram: &DescriptionDiagram) -> String {
    let cuca = &diagram.cuca;
    let mut out = String::new();
    let qualified = |entity: &Entity| cuca.quark(entity.get_quark()).get_qualified_name();
    for group in cuca.groups() {
        let group = cuca.entity(group);
        writeln!(
            out,
            "group {} {} {} {} {}",
            group.get_uid(),
            quoted(Some(qualified(group))),
            screaming(group.get_group_type()),
            symbol(group.get_u_symbol()),
            describe(group)
        )
        .unwrap();
    }
    for leaf in cuca.leafs() {
        let leaf = cuca.entity(leaf);
        let Some(leaf_type) = leaf.get_leaf_type() else {
            unreachable!("a leaf");
        };
        writeln!(
            out,
            "leaf {} {} {} {} {}",
            leaf.get_uid(),
            quoted(Some(qualified(leaf))),
            leaf_type.name(),
            symbol(leaf.get_u_symbol()),
            describe(leaf)
        )
        .unwrap();
    }
    for link in cuca.get_links() {
        let link_type = link.get_type();
        writeln!(
            out,
            "link {} {} {} {} {} {} {}{} length={} label={} q1={} q2={} {}{}{} stereo={}",
            link.get_uid(),
            cuca.entity(link.get_entity1()).get_uid(),
            cuca.entity(link.get_entity2()).get_uid(),
            screaming(link_type.get_decor1()),
            screaming(link_type.get_decor2()),
            screaming(link_type.get_middle_decor()),
            link_type.get_stroke3(None),
            if link_type.is_invisible() {
                " invisible"
            } else {
                ""
            },
            link.get_length(),
            quoted(Some(&link.get_label().map(lines).unwrap_or_default())),
            quoted(link.get_quantifier1()),
            quoted(link.get_quantifier2()),
            screaming(link.get_link_arrow()),
            if link.is_inverted() { " inverted" } else { "" },
            colors(link.get_colors()),
            quoted(
                link.stereotype
                    .as_ref()
                    .map(|stereotype| stereotype.label_double_comparator())
                    .as_deref()
            ),
        )
        .unwrap();
    }
    out
}

#[test]
fn the_commands_build_plantumls_model() {
    let cases = fixture_cases();
    assert_eq!(cases.len(), 68);
    let mut failures = Vec::new();
    for (case, (expected, _)) in &cases {
        // The note commands are ported with the notes.
        if expected.contains(" NOTE ") {
            continue;
        }
        let actual = dump_model(&read(case));
        if &actual != expected {
            failures.push(format!(
                "=== {case}\n--- expected\n{expected}--- actual\n{actual}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

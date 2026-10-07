//! The description corpus read by the commands, against what PlantUML's commands make of it. The fixture
//! comes from `tools/oracle/cuca-unit/DescriptionDump.java`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::rc::Rc;

use super::{DescriptionDiagram, DescriptionDiagramFactory};
use crate::abel::{Entity, LeafType};
use crate::color::{ColorType, Colors, HColor};
use crate::command::factory::{Created, create_system};
use crate::creole::Display;
use crate::decoration::symbol::{USymbol, USymbols};
use crate::diagram::builder::CommandFactory;
use crate::host::IsolatedHost;
use crate::java::double_to_string;
use crate::klimt::TextBlock;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::ugraphic::UGraphic;
use crate::preproc::{PreprocessorEnvironment, Source, preprocess};
use crate::sdot::CucaDiagramFileMakerSmetana;
use crate::svek::IEntityImage;
use crate::svek::image::{EntityImageDescription, EntityImagePort};

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
    parse(&text, case, path.parent().unwrap().to_owned())
}

/// The diagram the description commands make of the lines between `@startuml` and `@enduml`.
fn parse_lines(body: &[&str]) -> DescriptionDiagram {
    let text = ["@startuml", &body.join("\n"), "@enduml"].join("\n");
    parse(&text, "test", PathBuf::new())
}

fn parse(text: &str, case: &str, directory: PathBuf) -> DescriptionDiagram {
    let source = Source {
        text,
        description: case,
        directory,
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
            symbol(group.get_usymbol()),
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
            symbol(leaf.get_usymbol()),
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

/// Each leaf's image as `DescriptionDump` writes it: its kind, size and drawing on the debug surface.
fn dump_images(diagram: &DescriptionDiagram) -> String {
    let cuca = &diagram.cuca;
    let mut out = String::new();
    for leaf in cuca.leafs() {
        let entity = cuca.entity(leaf);
        if entity.is_removed(cuca) || entity.get_leaf_type() == Some(LeafType::Note) {
            continue;
        }
        let (class, image): (_, Box<dyn IEntityImage>) = match entity.get_leaf_type() {
            Some(
                LeafType::Description
                | LeafType::Usecase
                | LeafType::UsecaseBusiness
                | LeafType::Circle,
            ) => (
                "EntityImageDescription",
                Box::new(EntityImageDescription::new(entity, cuca)),
            ),
            Some(LeafType::Portin | LeafType::Portout) => (
                "EntityImagePort",
                Box::new(EntityImagePort::new(entity, cuca)),
            ),
            other => panic!("no image for {other:?}"),
        };
        writeln!(out, "image {} {class}", entity.get_uid()).unwrap();
        let dimension = image.calculate_dimension(&StringBounderDebug);
        writeln!(
            out,
            "dimension: {} {}",
            double_to_string(dimension.width),
            double_to_string(dimension.height)
        )
        .unwrap();
        writeln!(out, "shape: {}", screaming(image.get_shape_type())).unwrap();
        let debug = Rc::new(RefCell::new(UGraphicDebug::new("DATE".to_owned())));
        let ug = UGraphic::new(debug.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
        image.draw_u(&ug);
        let document = debug.borrow().document(&DebugHeader {
            dimension,
            scale_factor: 1.0,
            seed: 0,
            svg_link_target: None,
            hover_path_color_rgb: None,
            preserve_aspect_ratio: "none".to_owned(),
        });
        for line in document
            .split('\n')
            .skip_while(|line| !line.is_empty())
            .skip(1)
        {
            out += line;
            out.push('\n');
        }
    }
    out
}

#[test]
fn the_images_draw_like_plantumls() {
    let cases = fixture_cases();
    let mut failures = Vec::new();
    for (case, (model, expected)) in &cases {
        // The note commands are ported with the notes, `remove` with `hide` and `show`.
        if model.contains(" NOTE ") || *case == "component/hide-unlinked.puml" {
            continue;
        }
        let actual = dump_images(&read(case));
        if actual.trim_end() != expected.trim_end() {
            failures.push(format!(
                "=== {case}\n--- expected\n{expected}--- actual\n{actual}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} cases differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn the_commands_build_plantumls_model() {
    let cases = fixture_cases();
    assert_eq!(cases.len(), 68);
    let mut failures = Vec::new();
    for (case, (expected, _)) in &cases {
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

/// The calls of a trace's input section, up to the layout.
fn traced_input(trace: &str) -> Vec<String> {
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
    let traces = PathBuf::from(CORPUS).join("../smetana");
    let mut failures = Vec::new();
    for case in fixture_cases().keys() {
        // `remove` is ported with `hide` and `show`.
        if *case == "component/hide-unlinked.puml" {
            continue;
        }
        let Ok(trace) =
            std::fs::read_to_string(traces.join(case.trim_end_matches(".puml")).join("01.trace"))
        else {
            continue;
        };
        let mut cuca = read(case).cuca;
        cuca.eventually_build_phantom_groups(None);
        let calls = CucaDiagramFileMakerSmetana::new(cuca).smetana_calls(&StringBounderDebug);
        match calls {
            Ok(calls) if calls == traced_input(&trace) => {}
            Ok(_) => failures.push((*case).to_owned()),
            Err(not_ported) => failures.push(format!("{case}: {not_ported}")),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn link_labels_show_a_visibility_as_an_icon_unless_icons_are_off() {
    let label = |icon_size: &str| {
        let diagram = parse_lines(&[
            &format!("skinparam classAttributeIconSize {icon_size}"),
            "actor User",
            "User --> (Run) : +start",
        ]);
        let cuca = &diagram.cuca;
        let link = cuca.link(cuca.get_link_ids()[0]);
        (
            link.get_visibility_modifier().is_some(),
            link.get_label().unwrap().lines().join("\n"),
        )
    };
    assert_eq!(label("10"), (true, "start".to_owned()));
    assert_eq!(label("0"), (false, "+start".to_owned()));
}

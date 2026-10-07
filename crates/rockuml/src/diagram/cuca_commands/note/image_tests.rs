//! The images of notes against PlantUML's, drawn on the debug surface. The fixture comes from
//! `tools/oracle/cuca-unit/NoteDump.java`, whose cases these inputs mirror.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::command::factory::{self, Created};
use crate::diagram::UmlSource;
use crate::diagram::builder::CommandFactory;
use crate::diagram::class::ClassDiagramFactory;
use crate::diagram::description::DescriptionDiagramFactory;
use crate::diagram::state::StateDiagramFactory;
use crate::java::double_to_string;
use crate::klimt::TextBlock;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::geom::{UTranslate, XPoint2D, XRectangle2D};
use crate::klimt::ugraphic::UGraphic;
use crate::svek::image::{EntityImageNote, EntityImageNoteLink, EntityImageTips, OpaleLink};
use crate::text::StringLocated;

const FIXTURE: &str = include_str!("../../../../tests/data/note.txt");

/// `NoteDump.SOURCES`: each case's lines between `@startuml` and `@enduml`.
const SOURCES: &[(&str, &[&str])] = &[
    ("plain", &["note \"Simple note\" as N1"]),
    (
        "multi",
        &[
            "note as N1",
            "  Reads **tokens** and",
            "  builds an //AST//.",
            "  ----",
            "  Second block",
            "  == Title ==",
            "  last",
            "end note",
        ],
    ),
    ("color", &["note \"Colored\" as N1 #pink"]),
    (
        "styled",
        &[
            "<style>",
            "note {",
            "  BackGroundColor lightblue",
            "  LineColor red",
            "  LineThickness 2",
            "}",
            ".warn {",
            "  BackGroundColor yellow",
            "}",
            "</style>",
            "note \"Styled\" as N1 <<warn>>",
            "note \"Plain\" as N2",
        ],
    ),
    (
        "round",
        &["skinparam roundCorner 12", "note \"Rounded\\nnote\" as N1"],
    ),
    ("empty", &["note as N1", "", "end note"]),
    (
        "wrap",
        &[
            "<style>",
            "note {",
            "  MaximumWidth 60",
            "  HorizontalAlignment center",
            "}",
            "</style>",
            "note \"A long note that should wrap over several lines\" as N1",
        ],
    ),
    ("state", &["state S", "note \"In a state diagram\" as N1"]),
    (
        "usecase",
        &["usecase U", "note \"In a description diagram\" as N1"],
    ),
];

/// `NoteDump.NOTE_CORNER`: where opale notes are laid out.
const NOTE_CORNER: XPoint2D = XPoint2D::new(100.0, 50.0);

fn located(texts: &[&str]) -> Vec<StringLocated> {
    let location = crate::text::LineLocation::new("test", None);
    texts
        .iter()
        .map(|text| StringLocated::new(*text, location.clone()))
        .collect()
}

/// The diagram `F` reads the case's lines as.
fn parse<F: CommandFactory>(case: &str) -> CucaDiagram
where
    F::Diagram: EntityDiagram,
{
    let (_, body) = SOURCES
        .iter()
        .find(|(name, _)| *name == case)
        .expect("a known case");
    let mut texts = vec!["@startuml"];
    texts.extend_from_slice(body);
    texts.push("@enduml");
    let source = Rc::new(UmlSource::new(located(&texts), Vec::new()));
    let commands = F::init_commands_list();
    match factory::create_system(&source, || F::create_empty_diagram(&source), &commands) {
        Created::Diagram(mut diagram) => std::mem::replace(diagram.cuca(), empty_cuca()),
        _ => panic!("{case} reads as a diagram"),
    }
}

fn empty_cuca() -> CucaDiagram {
    let source = UmlSource::new(Vec::new(), Vec::new());
    CucaDiagram::new(crate::diagram::titled::Titled::new(
        crate::style::SName::ClassDiagram,
        "CLASS",
        &source,
    ))
}

/// The case's diagram, read by the factory PlantUML picks for it.
fn diagram(case: &str) -> CucaDiagram {
    match case {
        "state" => parse::<StateDiagramFactory>(case),
        "usecase" => parse::<DescriptionDiagramFactory>(case),
        _ => parse::<ClassDiagramFactory>(case),
    }
}

fn note(diagram: &CucaDiagram, name: &str) -> EntityImageNote {
    let quark = diagram.first_with_name(name).expect("the note exists");
    let entity = diagram.quark(quark).get_data().expect("the note exists");
    EntityImageNote::new(diagram.entity(entity), diagram)
}

/// The image's size and drawing as `NoteDump.dump` writes them.
fn dump(block: &dyn TextBlock, draw: impl FnOnce(&UGraphic)) -> String {
    let dimension = block.calculate_dimension(&StringBounderDebug);
    let mut out = format!(
        "dimension: {} {}\n",
        double_to_string(dimension.width),
        double_to_string(dimension.height)
    );
    let debug = Rc::new(RefCell::new(UGraphicDebug::new("DATE".to_owned())));
    let ug = UGraphic::new(debug.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
    draw(&ug);
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
    out
}

fn dump_opale(words: &[&str]) -> String {
    let [case, values @ ..] = words else {
        panic!("an opale case names its note")
    };
    let values: Vec<f64> = values.iter().map(|value| value.parse().unwrap()).collect();
    let [sx, sy, ex, ey, fx, fy] = values[..] else {
        panic!("an opale case has six numbers")
    };
    let image = note(&diagram(case), "N1");
    let corner = |x: f64, y: f64| XPoint2D::new(NOTE_CORNER.x + x, NOTE_CORNER.y + y);
    let link = OpaleLink {
        start: corner(sx, sy),
        end: corner(ex, ey),
        node_min: NOTE_CORNER,
        other_force: UTranslate::new(fx, fy),
    };
    dump(&image, |ug| image.draw_with(ug, Some(link)))
}

/// `NoteDump.LINK_NOTES`: the colour or `-`, then the lines.
fn dump_link(spec: &str) -> String {
    let mut parts = spec.split('|');
    let color = parts.next().expect("a colour");
    let colors = if color == "-" {
        Colors::default()
    } else {
        Colors::parse(color, ColorType::Back).unwrap()
    };
    let display = Display::create(parts);
    let diagram = diagram("plain");
    let skin = diagram.skin();
    let image = EntityImageNoteLink::new(&display, &colors, skin, &skin.current_style_builder());
    dump(&image, |ug| image.draw_u(ug))
}

/// `NoteDump.TIPS_CORNER` and `NoteDump.CLASS_CORNER`: where the tips and their class are laid out.
const TIPS_CORNER: XPoint2D = XPoint2D::new(300.0, 40.0);
const CLASS_CORNER: XPoint2D = XPoint2D::new(100.0, 20.0);

/// `NoteDump.dumpTips`: the class is declared by hand and its members placed by hand, as classes are not
/// the subject here.
fn dump_tips(side: &str) -> String {
    struct Tips(CucaDiagram);
    impl EntityDiagram for Tips {
        fn cuca(&mut self) -> &mut CucaDiagram {
            &mut self.0
        }
    }
    let mut tips = Tips(diagram("plain"));
    let quark = tips.0.quark_in_context(false, "Thread");
    tips.0
        .really_create_leaf(None, quark, Display::create(["Thread"]), LeafType::Class);
    let command = tip_on_entity_multi_line(false);
    for lines in [
        &[
            "note right of Thread::start",
            "  spawns the",
            "  OS thread",
            "end note",
        ][..],
        &[
            "note right of Thread::priority #pink",
            "  1 to 10",
            "end note",
        ],
        &["note left of Thread::start", "  starts", "end note"],
    ] {
        command
            .execute(&mut tips, BlocLines::from_texts(lines))
            .unwrap();
    }
    let diagram = tips.0;
    let quark = diagram
        .first_with_name(&format!("Thread$$${side}"))
        .unwrap();
    let entity = diagram.quark(quark).get_data().unwrap();
    let image = EntityImageTips::new(diagram.entity(entity), &diagram);
    let rectangle = |x, y, width| XRectangle2D {
        x,
        y,
        width,
        height: 14.0,
    };
    let members = |member: &str| match member {
        "start" => Some(rectangle(10.0, 20.0, 80.0)),
        "priority" => Some(rectangle(10.0, 40.0, 90.0)),
        _ => None,
    };
    dump(&image, |ug| {
        image.draw_tips(ug, TIPS_CORNER, CLASS_CORNER, &members);
    })
}

fn rust_dump(header: &str) -> String {
    let words: Vec<&str> = header.split(' ').collect();
    match words[..] {
        ["note", case, name] => {
            let image = note(&diagram(case), name);
            dump(&image, |ug| image.draw_u(ug))
        }
        ["opale", ..] => dump_opale(&words[1..]),
        ["link", ..] => dump_link(&header["link ".len()..]),
        ["tips", side] => dump_tips(side),
        _ => panic!("unknown case {header}"),
    }
}

#[test]
fn notes_are_drawn_as_plantuml_draws_them() {
    let mut cases: BTreeMap<&str, String> = BTreeMap::new();
    let mut current = None;
    for line in FIXTURE.lines().skip(1) {
        if let Some(header) = line.strip_prefix("=== ") {
            current = Some(header);
            cases.insert(header, String::new());
        } else {
            let body = cases.get_mut(current.expect("a case first")).unwrap();
            body.push_str(line);
            body.push('\n');
        }
    }
    let mut failures = Vec::new();
    for (header, expected) in &cases {
        let actual = rust_dump(header);
        if actual.trim_end() != expected.trim_end() {
            failures.push(format!(
                "=== {header}\n--- expected\n{expected}--- actual\n{actual}"
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

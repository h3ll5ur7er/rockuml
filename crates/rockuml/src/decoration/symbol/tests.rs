//! Every symbol's small and big forms against PlantUML's, drawn on the debug surface. The fixture comes from
//! `tools/oracle/cuca-unit/USymbolDump.java`, whose cases these inputs mirror.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::rc::Rc;

use super::{Block, USymbols};
use crate::color::{HColor, XColor};
use crate::java::double_to_string;
use crate::klimt::blocks::TextBlockRaw;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::{XDimension2D, XPoint2D};
use crate::klimt::stencil::UHorizontalLine;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::TextBlockEmpty;

const FIXTURE: &str = include_str!("../../../tests/data/usymbol.txt");

const PROBES: [(f64, f64); 8] = [
    (-1.0, -1.0),
    (10.0, 5.0),
    (40.0, -2.0),
    (45.0, 0.0),
    (50.0, -1.0),
    (60.0, 10.0),
    (100.0, -5.0),
    (150.0, 20.0),
];

fn label_font() -> FontConfiguration {
    FontConfiguration::black_blue_true(UFont::serif(14))
}

fn stereo_font() -> FontConfiguration {
    FontConfiguration::black_blue_true(UFont::monospace(11))
}

fn raw(font: FontConfiguration, lines: &[&str]) -> TextBlockRaw {
    TextBlockRaw::new(lines.iter().copied(), font)
}

fn empty() -> Block {
    Rc::new(TextBlockEmpty::default())
}

/// Two blocks with a separator between them, as bodies draw them.
struct Separated {
    top: TextBlockRaw,
    style: char,
    title: Option<TextBlockRaw>,
    bottom: TextBlockRaw,
}

impl TextBlock for Separated {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.top
            .calculate_dimension(string_bounder)
            .merge_top_bottom(self.bottom.calculate_dimension(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.top.draw_u(ug);
        let y = self.top.calculate_dimension(ug.string_bounder()).height;
        ug.translated(0.0, y)
            .draw_horizontal_line(&UHorizontalLine {
                style: self.style,
                title: self.title.as_ref().map(|title| title as &dyn TextBlock),
                default_thickness: 1.0,
                skip: 0.0,
            });
        self.bottom.draw_u(&ug.translated(0.0, y));
    }
}

fn fashion(name: &str) -> Fashion {
    let color = |rgb| HColor::Simple(XColor::from_rgb(rgb));
    let plain =
        Fashion::new(color(0xFFEEDD), color(0x112233)).with_stroke(UStroke::with_thickness(1.5));
    match name {
        "round" => plain.with_corner(10.0, 0.0),
        "diagonal" => plain.with_corner(0.0, 8.0),
        _ => plain,
    }
}

fn alignment(name: &str) -> HorizontalAlignment {
    HorizontalAlignment::from_name(name).unwrap()
}

fn small_label(case: &str) -> Block {
    match case {
        "plain" => Rc::new(raw(label_font(), &["Label"])),
        "stereo" => Rc::new(raw(label_font(), &["Some label", "x"])),
        "dashes" => Rc::new(Separated {
            top: raw(label_font(), &["Top line"]),
            style: '.',
            title: None,
            bottom: raw(label_font(), &["Bottom"]),
        }),
        "titled" => Rc::new(Separated {
            top: raw(label_font(), &["Above the line"]),
            style: '=',
            title: Some(raw(stereo_font(), &["T"])),
            bottom: raw(label_font(), &["Below"]),
        }),
        "tall" => Rc::new(raw(
            label_font(),
            &[
                "First line of text",
                "Second",
                "Third line",
                "Fourth",
                "Fifth line here",
                "Sixth",
            ],
        )),
        _ => panic!("no small case {case}"),
    }
}

fn small_stereo(case: &str) -> Block {
    match case {
        "stereo" | "titled" => Rc::new(raw(stereo_font(), &["<<stereo>>"])),
        "dashes" => Rc::new(raw(stereo_font(), &["<<s>>"])),
        _ => empty(),
    }
}

fn big_title(case: &str) -> Block {
    match case {
        "titled" => Rc::new(raw(label_font(), &["Package title"])),
        "stereo" => Rc::new(raw(label_font(), &["T"])),
        "narrow" => Rc::new(raw(label_font(), &["A long title text"])),
        _ => empty(),
    }
}

fn big_stereo(case: &str) -> Block {
    if case == "stereo" {
        Rc::new(raw(stereo_font(), &["<<big>>"]))
    } else {
        empty()
    }
}

/// The block as `USymbolDump.dump` writes it.
fn dump(block: &dyn TextBlock) -> String {
    let dimension = block.calculate_dimension(&StringBounderDebug);
    let mut out = format!(
        "dimension: {} {}\n",
        double_to_string(dimension.width),
        double_to_string(dimension.height)
    );
    for (x, y) in PROBES {
        let force = block.magnetic_border_force_at(&StringBounderDebug, XPoint2D::new(x, y));
        if force.dx != 0.0 || force.dy != 0.0 {
            writeln!(
                out,
                "force {} {}: {} {}",
                double_to_string(x),
                double_to_string(y),
                double_to_string(force.dx),
                double_to_string(force.dy)
            )
            .unwrap();
        }
    }
    let debug = Rc::new(RefCell::new(UGraphicDebug::new("DATE".to_owned())));
    let ug = UGraphic::new(debug.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
    block.draw_u(&ug);
    let document = debug.borrow().document(&DebugHeader {
        dimension,
        scale_factor: 1.0,
        seed: 0,
        svg_link_target: None,
        hover_path_color_rgb: None,
        preserve_aspect_ratio: "none".to_owned(),
    });
    let shapes = document
        .split('\n')
        .skip_while(|line| !line.is_empty())
        .skip(1);
    for line in shapes {
        out += line;
        out.push('\n');
    }
    out
}

/// The block a fixture header like `small CLOUD plain center plain` names.
fn block(header: &str) -> Option<Box<dyn TextBlock>> {
    let words: Vec<&str> = header.split(' ').collect();
    let symbol = USymbols::by_code(words[1]).unwrap();
    let name: Block = Rc::new(raw(label_font(), &["Name"]));
    match words[..] {
        ["small", _, case, stereo_alignment, fashion_name] => Some(symbol.as_small(
            name,
            small_label(case),
            small_stereo(case),
            fashion(fashion_name),
            alignment(stereo_alignment),
        )),
        [
            "big",
            _,
            case,
            width,
            height,
            label_alignment,
            stereo_alignment,
            fashion_name,
        ] => symbol.as_big(
            big_title(case),
            alignment(label_alignment),
            big_stereo(case),
            width.parse().unwrap(),
            height.parse().unwrap(),
            fashion(fashion_name),
            alignment(stereo_alignment),
        ),
        _ => None,
    }
}

fn fixture_cases() -> BTreeMap<&'static str, String> {
    let mut cases = BTreeMap::new();
    for case in FIXTURE.split("=== ").skip(1) {
        let (header, body) = case.split_once('\n').unwrap();
        cases.insert(header, body.to_owned());
    }
    cases
}

#[test]
fn symbols_draw_like_plantuml() {
    let cases = fixture_cases();
    assert_eq!(cases.len(), 280);
    let mut failures = Vec::new();
    for (header, expected) in &cases {
        let block = block(header).unwrap();
        let actual = dump(block.as_ref());
        if &actual != expected {
            failures.push(format!(
                "=== {header}\n--- expected\n{expected}--- actual\n{actual}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ; first:\n{}",
        failures.len(),
        cases.len(),
        failures[0]
    );
}

#[test]
fn figures_have_no_big_form() {
    for symbol in [
        USymbols::ACTOR_STICKMAN,
        USymbols::ACTOR_STICKMAN_BUSINESS,
        USymbols::BOUNDARY,
        USymbols::CONTROL,
        USymbols::ENTITY_DOMAIN,
        USymbols::INTERFACE,
        USymbols::PERSON,
        USymbols::USECASE,
    ] {
        let big = symbol.as_big(
            big_title("plain"),
            HorizontalAlignment::Center,
            big_stereo("plain"),
            100.0,
            50.0,
            fashion("plain"),
            HorizontalAlignment::Center,
        );
        assert!(big.is_none(), "{symbol:?}");
    }
}

#[test]
fn symbols_name_their_styles_and_the_room_their_clusters_need() {
    use crate::style::SName;
    assert_eq!(USymbols::AGENT.get_s_names(), [SName::Agent]);
    assert_eq!(USymbols::ACTOR_HOLLOW.get_s_names(), [SName::Actor]);
    assert_eq!(
        USymbols::USECASE_BUSINESS.get_s_names(),
        [SName::Usecase, SName::Business]
    );
    assert_eq!(USymbols::PACKAGE.get_s_names(), [SName::Package]);
    let room = |symbol: super::USymbol| {
        (
            symbol.supp_width_because_of_shape(),
            symbol.supp_height_because_of_shape(),
        )
    };
    assert_eq!(room(USymbols::NODE), (60, 5));
    assert_eq!(room(USymbols::DATABASE), (0, 15));
    assert_eq!(room(USymbols::PACKAGE), (0, 0));
}

#[test]
fn package_like_symbols_come_from_their_names_and_styles() {
    use super::PackageStyle;
    use crate::skin::actor::ActorStyle;
    use crate::skin::component_style::ComponentStyle;
    let from = |name| {
        USymbols::from_string(
            name,
            ActorStyle::Awesome,
            ComponentStyle::Uml1,
            PackageStyle::Node,
        )
    };
    assert_eq!(from("package"), Some(USymbols::NODE));
    assert_eq!(from("Actor"), Some(USymbols::ACTOR_AWESOME));
    assert_eq!(from("component"), Some(USymbols::COMPONENT1));
    assert_eq!(from("entity"), Some(USymbols::ENTITY_DOMAIN));
    assert_eq!(from("circle"), Some(USymbols::INTERFACE));
    assert_eq!(
        from("<<usecase_business>>"),
        Some(USymbols::USECASE_BUSINESS)
    );
    assert_eq!(from("nothing"), None);
    assert_eq!(
        PackageStyle::from_string("Rect"),
        Some(PackageStyle::Rectangle)
    );
    assert_eq!(PackageStyle::Agent.to_u_symbol(), None);
}

#[test]
fn element_keywords_name_symbols() {
    let skin_param = crate::skin::SkinParam::default();
    let from = |name| USymbols::from_string_skin_param(name, &skin_param);
    assert_eq!(from("actor/"), Some(USymbols::ACTOR_STICKMAN_BUSINESS));
    assert_eq!(from("ACTOR"), Some(USymbols::ACTOR_STICKMAN));
    assert_eq!(from("component"), Some(USymbols::COMPONENT2));
    assert_eq!(from("()"), Some(USymbols::INTERFACE));
    assert_eq!(from("usecase"), None);
    assert_eq!(skin_param.package_style(), super::PackageStyle::Folder);
}

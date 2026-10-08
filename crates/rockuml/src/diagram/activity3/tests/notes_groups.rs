//! Notes and groups around a plain box against PlantUML's, their geometry and their drawing on the debug
//! surface. The fixture comes from `tools/oracle/activity-unit/NoteGroupDump.java`, whose cases these mirror.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::super::commands::group_style_signature;
use super::*;
use crate::diagram::activity3::{NotePosition, SwimlaneSet};
use crate::ftile::vcompact::{FtileGroup, FtileNoteAlone, FtileWithNoteOpale, FtileWithNotes};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable, TextBlockInterceptorUDrawable};
use crate::java::double_to_string;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::font::StringBounder;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{UDrawable, VerticalAlignment};
use crate::skin::SkinParam;
use crate::svek::UGraphicForSnake;

const FIXTURE: &str = include_str!("../../../../tests/data/activity-notes-groups.txt");

/// `NoteGroupDump.CASES`: a name, the skin lines joined by `|`, then what is built.
const CASES: &[&[&str]] = &[
    &[
        "right",
        "",
        "box 120 36 0",
        "note right note -|Checks types and ranges",
        "opale CENTER",
    ],
    &[
        "left-creole",
        "",
        "box 120 36 0",
        "note left note -|This note is on several|//lines// and can|contain <b>HTML</b>|====|* Calling the method \"\"foo()\"\" is prohibited",
        "opale CENTER",
    ],
    &[
        "floating",
        "",
        "box 120 36 0",
        "note right floating -|Aggregation runs|every five minutes",
        "opale CENTER",
    ],
    &[
        "colored",
        "",
        "box 80 20 0",
        "note right note #lightgreen|Automatic",
        "opale CENTER",
    ],
    &[
        "tall-top",
        "",
        "box 60 10 0",
        "note left note -|one|two|three",
        "opale TOP",
    ],
    &[
        "styled",
        "<style>|note {|  BackGroundColor lightblue|  LineColor red|  LineThickness 2|  MaximumWidth 60|}|</style>",
        "box 120 36 0",
        "note right note -|A long note that should wrap over several lines",
        "opale CENTER",
    ],
    &[
        "aligned",
        "skinparam noteTextAlignment center",
        "box 120 36 0",
        "note left note -|centred|short and a longer line",
        "opale CENTER",
    ],
    &[
        "several",
        "",
        "box 100 30 0",
        "note left note -|left one",
        "note right note -|right one",
        "note right note #pink|right two|second line",
        "opale CENTER",
    ],
    &[
        "notes-top",
        "",
        "box 100 80 0",
        "note left note -|on top",
        "notes TOP",
    ],
    &[
        "alone",
        "",
        "note right note -|Alone|with two lines",
        "alone",
    ],
    &[
        "alone-floating",
        "skinparam noteFontColor blue",
        "note left floating #red|Floating alone",
        "alone",
    ],
    &[
        "partition",
        "",
        "box 120 36 0",
        "group partition -|Initialization",
    ],
    &["group", "", "box 120 36 0", "group group -|Running"],
    &["package", "", "box 120 36 0", "group package -|Data layer"],
    &[
        "rectangle",
        "",
        "box 120 36 0",
        "group rectangle -|Business layer",
    ],
    &[
        "card",
        "",
        "box 120 36 0",
        "group card -|Presentation layer",
    ],
    &[
        "wide-title",
        "",
        "box 40 20 0",
        "group partition -|A partition with a title much wider than its box",
    ],
    &[
        "colored-group",
        "",
        "box 120 36 0",
        "group partition #lightblue|Order handling",
    ],
    &[
        "overflow",
        "",
        "box 100 30 45",
        "group partition -|Overflow",
    ],
    &[
        "nested",
        "",
        "box 120 36 0",
        "group partition #lightyellow|Payment",
        "group partition #lightblue|Order handling",
    ],
    &[
        "styled-group",
        "<style>|partition {|  RoundCorner 12|  LineThickness 2|  FontSize 18|}|</style>|skinparam packageTitleAlignment left",
        "box 120 36 0",
        "group partition -|Styled",
    ],
    &[
        "note-in-group",
        "",
        "box 120 36 0",
        "note right note -|Inside",
        "opale CENTER",
        "group group -|With a note",
    ],
];

/// A box that may draw wider than it says, as tiles with arrow labels do.
struct Box {
    base: AbstractFtile,
    width: f64,
    height: f64,
    overflow: f64,
}

impl Swimable for Box {
    fn get_swimlanes(&self) -> SwimlaneSet {
        SwimlaneSet::new()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        None
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        None
    }
}

impl Ftile for Box {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        FtileGeometry::with_out(self.width, self.height, self.width / 2.0, 0.0, self.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&UShape::Rectangle(URectangle::new(
            self.width + self.overflow,
            self.height,
        )));
    }
}

/// The skin of a diagram with `skin` before its only instruction.
fn skin_param(skin: &str) -> Rc<SkinParam> {
    let mut lines = vec!["@startuml"];
    lines.extend(skin.split('|').filter(|line| !line.is_empty()));
    lines.extend(["start", "@enduml"]);
    match parse(&lines.join("\n"), "test", PathBuf::new()) {
        Created::Diagram(diagram) => Rc::new(diagram.titled.skin),
        _ => panic!("{skin} makes no diagram"),
    }
}

fn colors(color: &str) -> Colors {
    if color == "-" {
        return Colors::default();
    }
    Colors::parse(color, ColorType::Back).unwrap()
}

fn symbol(type_: &str) -> USymbol {
    match type_ {
        "package" => USymbols::PACKAGE,
        "rectangle" => USymbols::RECTANGLE,
        "card" => USymbols::CARD,
        "group" => USymbols::GROUP,
        _ => USymbols::PARTITION,
    }
}

fn vertical_alignment(name: &str) -> VerticalAlignment {
    match name {
        "TOP" => VerticalAlignment::Top,
        "CENTER" => VerticalAlignment::Center,
        _ => panic!("no alignment {name}"),
    }
}

fn build(case: &[&str]) -> Rc<dyn Ftile> {
    let skin_param = skin_param(case[1]);
    let mut tile: Option<Rc<dyn Ftile>> = None;
    let mut notes: Vec<PositionedNote> = Vec::new();
    for item in &case[2..] {
        let (head, rest) = item.split_once('|').unwrap_or((item, ""));
        let words: Vec<&str> = head.split(' ').collect();
        let inner = || tile.clone().expect("a tile to wrap");
        tile = Some(match words[0] {
            "box" => Rc::new(Box {
                base: AbstractFtile::new(skin_param.clone()),
                width: words[1].parse().unwrap(),
                height: words[2].parse().unwrap(),
                overflow: words[3].parse().unwrap(),
            }),
            "note" => {
                notes.push(PositionedNote {
                    display: Display::create(rest.split('|')),
                    note_position: if words[1] == "left" {
                        NotePosition::Left
                    } else {
                        NotePosition::Right
                    },
                    type_: if words[2] == "floating" {
                        NoteType::FloatingNote
                    } else {
                        NoteType::Note
                    },
                    colors: colors(words[3]),
                    swimlane_note: None,
                    stereotype: None,
                });
                continue;
            }
            "opale" => FtileWithNoteOpale::create(
                inner(),
                &std::mem::take(&mut notes),
                true,
                vertical_alignment(words[1]),
            ),
            "notes" => Rc::new(FtileWithNotes::new(
                inner(),
                &std::mem::take(&mut notes),
                vertical_alignment(words[1]),
            )),
            "alone" => {
                let note = std::mem::take(&mut notes).remove(0);
                Rc::new(FtileNoteAlone::new(
                    &note.display,
                    skin_param.clone(),
                    &note.colors,
                    note.type_ == NoteType::Note,
                    None,
                ))
            }
            "group" => {
                let symbol = symbol(words[1]);
                let style = group_style_signature(symbol)
                    .get_merged_style(&skin_param.current_style_builder());
                Rc::new(FtileGroup::new(
                    inner(),
                    &Display::with_newlines(rest),
                    colors(words[2]).get(ColorType::Back).cloned(),
                    &skin_param,
                    symbol,
                    &style,
                ))
            }
            other => panic!("unknown item {other}"),
        });
    }
    tile.expect("a case builds a tile")
}

fn dump(case: &[&str]) -> String {
    let tile = build(case);
    let geometry = tile.calculate_dimension(&StringBounderDebug);
    let mut out = format!(
        "=== {}\ngeometry: {} {} {} {} {} {}\n",
        case[0],
        double_to_string(geometry.get_width()),
        double_to_string(geometry.get_height()),
        double_to_string(geometry.get_left()),
        double_to_string(geometry.get_in_y()),
        double_to_string(geometry.get_out_y()),
        geometry.has_point_out(),
    );
    let debug = Rc::new(RefCell::new(UGraphicDebug::new("DATE".to_owned())));
    let ug = UGraphic::new(debug.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
    TextBlockInterceptorUDrawable::new(tile, HColor::BLACK).draw_u(&UGraphicForSnake::create(ug));
    let document = debug.borrow().document(&DebugHeader {
        dimension: geometry.dimension(),
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

/// The fixture's case named `name`, from its `===` line to the next.
fn expected(name: &str) -> &'static str {
    let start = FIXTURE
        .find(&format!("=== {name}\n"))
        .unwrap_or_else(|| panic!("no case {name} in the fixture"));
    let end = FIXTURE[start + 4..]
        .find("\n=== ")
        .map_or(FIXTURE.len(), |end| start + 4 + end + 1);
    &FIXTURE[start..end]
}

#[test]
fn notes_and_groups_are_laid_out_and_drawn_as_in_plantuml() {
    let failures: Vec<&str> = CASES
        .iter()
        .filter(|case| {
            let actual = dump(case);
            let expected = expected(case[0]);
            if actual == expected {
                return false;
            }
            eprintln!("--- {}: expected\n{expected}--- actual\n{actual}", case[0]);
            true
        })
        .map(|case| case[0])
        .collect();
    assert!(failures.is_empty(), "differ: {failures:?}");
}

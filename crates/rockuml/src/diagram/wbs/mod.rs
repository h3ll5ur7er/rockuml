//! Work breakdown structures, `@startwbs` (PlantUML's `wbs` package).

mod commands;
mod element;
mod itf;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use element::{Direction, ElementId, ElementSpec, Elements};
use itf::{Context, Fork};

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::mindmap::IdeaShape;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::{ColorType, Colors, HColor};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult, ParserPass};
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XPoint2D};
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;
use crate::pattern::java_regex;
use crate::stereo::Stereotype;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::extremity::{ExtremityFactory, ExtremityFactoryArrow};

pub(super) struct WbsDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    elements: Elements,
    last: Option<ElementId>,
    /// The marker of the root's line, which the indentation of the others is measured against.
    first: Option<String>,
    codes: HashMap<String, ElementId>,
    links: Vec<WbsLink>,
}

/// Reads work breakdowns (PlantUML's `WBSDiagramFactory`).
pub(super) struct WbsDiagramFactory;

impl CommandFactory for WbsDiagramFactory {
    type Diagram = WbsDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Wbs;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> WbsDiagram {
        WbsDiagram {
            source: source.clone(),
            titled: Titled::new(SName::WbsDiagram, "WBS", source),
            elements: Elements::default(),
            last: None,
            first: None,
            codes: HashMap::new(),
            links: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<WbsDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend([
            commands::item_new(true),
            commands::item_new(false),
            commands::item_multiline_new(),
            commands::item_old(true),
            commands::item_old(false),
            commands::item_multiline_old(),
            commands::link(),
        ]);
        commands
    }
}

fn level_of(level: usize) -> i32 {
    i32::try_from(level).expect("work breakdowns are not that deep")
}

/// What an element's line says about it.
struct NewElement {
    code: Option<String>,
    back_color: Option<HColor>,
    level: usize,
    label: Display,
    stereotype: Option<Stereotype>,
    direction: Direction,
    shape: IdeaShape,
}

impl WbsDiagram {
    /// An element whose one-line label may end with a stereotype.
    fn add_idea_labelled(
        &mut self,
        code: Option<String>,
        back_color: Option<HColor>,
        level: usize,
        label: &str,
        direction: Direction,
        shape: IdeaShape,
    ) -> CommandResult {
        static PATTERN_STEREOTYPE: LazyLock<Regex> =
            LazyLock::new(|| java_regex(r"^\s*(.*?)\s*(\<\<\s*(.*)\s*\>\>)\s*$", false));
        let (label, stereotype) = match PATTERN_STEREOTYPE.captures(label) {
            Some(captures) => (captures[1].to_owned(), Some(Stereotype::new(&captures[2]))),
            None => (label.to_owned(), None),
        };
        self.add_idea(NewElement {
            code,
            back_color,
            level,
            label: Display::with_newlines(&label),
            stereotype,
            direction,
            shape,
        })
    }

    fn add_idea(&mut self, element: NewElement) -> CommandResult {
        let spec = ElementSpec {
            back_color: element.back_color,
            label: element.label,
            stereotype: element.stereotype,
            style_builder: self.titled.skin.current_style_builder(),
            shape: element.shape,
        };
        if element.level == 0 {
            if self.elements.has_root() {
                return Err(CommandError::new(
                    "WBS diagram only accepts one root first declared.",
                ));
            }
            self.elements.init_root(spec);
            self.last = Some(Elements::ROOT);
            return Ok(());
        }
        let last = self.last.expect("the root comes first");
        let last_level = self.elements.get(last).level;
        let parent = if element.level == last_level + 1 {
            last
        } else if element.level <= last_level {
            self.parent_of_last(last_level - element.level + 1)
        } else {
            return Err(CommandError::new("Bad tree structure"));
        };
        let id = self
            .elements
            .create_element(parent, element.level, spec, element.direction);
        self.last = Some(id);
        if let Some(code) = element.code {
            self.codes.insert(code, id);
        }
        Ok(())
    }

    fn parent_of_last(&self, generations: usize) -> ElementId {
        let mut result = self.last.expect("there is a last element");
        for _ in 0..generations {
            result = self
                .elements
                .parent(result)
                .expect("the levels are consistent");
        }
        result
    }

    /// The depth of an element's marker, indented with spaces or tabs maybe (`getSmartLevel`); `None` for an
    /// indentation PlantUML cannot make sense of.
    fn get_smart_level(&mut self, marker: &str) -> Option<usize> {
        if !self.elements.has_root() {
            self.first = Some(marker.to_owned());
            return Some(0);
        }
        let first = self.first.clone().unwrap_or_default();
        let marker = marker.replace('\t', " ");
        let length = |text: &str| text.encode_utf16().count();
        if !marker.contains(' ') {
            return length(&marker).checked_sub(1);
        }
        if marker.ends_with(&first) {
            return length(&marker).checked_sub(length(&first));
        }
        if crate::java::trim(&marker).encode_utf16().count() == 1 {
            return length(&marker).checked_sub(1);
        }
        if marker.starts_with(&first) {
            return length(&marker).checked_sub(length(&first));
        }
        None
    }

    fn link(
        &mut self,
        code1: &str,
        code2: &str,
        colors: &Colors,
        stereotype: Option<&Stereotype>,
    ) -> CommandResult {
        let element1 = *self
            .codes
            .get(code1)
            .ok_or_else(|| CommandError::new(format!("No such node {code1}")))?;
        let element2 = *self
            .codes
            .get(code2)
            .ok_or_else(|| CommandError::new(format!("No such node {code2}")))?;
        let color = match colors.get(ColorType::Line) {
            Some(color) => color.clone(),
            None => {
                StyleSignature::of(&[SName::Root, SName::Element, SName::WbsDiagram, SName::Arrow])
                    .get_merged_style_with(&self.titled.skin.current_style_builder(), stereotype)
                    .value(PName::LineColor)
                    .as_color()
            }
        };
        self.links.push(WbsLink {
            element1,
            element2,
            color,
        });
        Ok(())
    }
}

/// An arrow from one element to another, drawn over the breakdown (PlantUML's `WBSLink`).
struct WbsLink {
    element1: ElementId,
    element2: ElementId,
    color: HColor,
}

impl WbsLink {
    /// Between the borders of the two elements, along the line joining their centres.
    fn draw_u(&self, ug: &UGraphic, elements: &Elements) {
        let (Some((position1, dim1)), Some((position2, dim2))) = (
            elements.get(self.element1).geometry(),
            elements.get(self.element2).geometry(),
        ) else {
            return;
        };
        let rect1 = (position1.dx, position1.dy, dim1.width, dim1.height);
        let rect2 = (position2.dx, position2.dy, dim2.width, dim2.height);
        let line = (center(rect1), center(rect2));
        let (Some(c1), Some(c2)) = (intersect(rect1, line), intersect(rect2, line)) else {
            return;
        };
        let ug = ug.apply(self.color.clone());
        ug.translated(c1.x, c1.y).draw(&UShape::Line {
            dx: c2.x - c1.x,
            dy: c2.y - c1.y,
        });
        let angle = libm::atan2(c2.y - c1.y, c2.x - c1.x);
        ExtremityFactoryArrow
            .create_udrawable(c2, angle)
            .draw_u(&ug);
    }
}

type Rectangle = (f64, f64, f64, f64);
type Line = (XPoint2D, XPoint2D);

fn center((x, y, width, height): Rectangle) -> XPoint2D {
    XPoint2D::new(x + width / 2.0, y + height / 2.0)
}

/// Where `line` crosses the rectangle's top, right, bottom or left side, tried in that order
/// (`XRectangle2D.intersect`).
fn intersect((x, y, width, height): Rectangle, line: Line) -> Option<XPoint2D> {
    let a = XPoint2D::new(x, y);
    let b = XPoint2D::new(x + width, y);
    let c = XPoint2D::new(x + width, y + height);
    let d = XPoint2D::new(x, y + height);
    [(a, b), (b, c), (c, d), (d, a)]
        .into_iter()
        .find_map(|side| intersect_lines(line, side))
}

/// Where two segments cross (`XLine2D.intersect`).
fn intersect_lines((p1, p2): Line, (q1, q2): Line) -> Option<XPoint2D> {
    let s1x = p2.x - p1.x;
    let s1y = p2.y - p1.y;
    let s2x = q2.x - q1.x;
    let s2y = q2.y - q1.y;
    let s = (-s1y * (p1.x - q1.x) + s1x * (p1.y - q1.y)) / (-s2x * s1y + s1x * s2y);
    let t = (s2x * (p1.y - q1.y) - s2y * (p1.x - q1.x)) / (-s2x * s1y + s1x * s2y);
    ((0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t))
        .then(|| XPoint2D::new(p1.x + t * s1x, p1.y + t * s1y))
}

/// The breakdown with a 10 pixel margin, its links drawn over it; nothing without a root, where PlantUML
/// crashes.
struct Drawing<'a> {
    fork: Option<Fork<'a>>,
    diagram: &'a WbsDiagram,
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.fork
            .as_ref()
            .map_or_else(XDimension2D::default, |fork| {
                fork.calculate_dimension(string_bounder)
            })
            .delta(20.0, 20.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let Some(fork) = &self.fork else {
            return;
        };
        let ug = ug.translated(10.0, 10.0);
        fork.draw_u(&ug);
        for link in &self.diagram.links {
            link.draw_u(&ug, &self.diagram.elements);
        }
    }
}

impl AbstractDiagram for WbsDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for WbsDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for WbsDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let context = Context {
            elements: &self.elements,
            skin_param: &self.titled.skin,
        };
        let drawing = Drawing {
            fork: self.elements.has_root().then(|| Fork::new(context)),
            diagram: self,
        };
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

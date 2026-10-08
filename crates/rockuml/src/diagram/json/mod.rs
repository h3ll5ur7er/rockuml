//! JSON and YAML documents drawn as linked tables, `@startjson` and `@startyaml` (PlantUML's `jsondiagram`
//! package and `YamlDiagramFactory`).

mod highlighted;
mod smetana_for_json;
mod style_extractor;
#[cfg(test)]
mod tests;
mod text_block_json;

use std::rc::Rc;

use highlighted::Highlighted;
use smetana_for_json::SmetanaForJson;
use style_extractor::StyleExtractor;

use super::common_commands::add_common_scale_commands;
use super::titled::{Titled, TitledDiagram};
use super::yaml;
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{BlocLines, CommandControl, ParserPass};
use crate::creole::Display;
use crate::json::JsonValue;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::SName;
use crate::text::StringLocated;

/// Which language the document is in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    Json,
    Yaml,
}

impl Language {
    fn name(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Yaml => "YAML",
        }
    }

    fn style_name(self) -> SName {
        match self {
            Self::Json => SName::JsonDiagram,
            Self::Yaml => SName::YamlDiagram,
        }
    }
}

pub(super) struct JsonDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    language: Language,
    /// `None` for data that does not parse.
    root: Option<JsonValue>,
    highlighted: Vec<Highlighted>,
}

/// A `@startjson` diagram (`JsonDiagramFactory`).
pub(super) fn create_json(source: UmlSource) -> Box<dyn Diagram> {
    let extractor = StyleExtractor::new(source.lines());
    let mut highlighted = Vec::new();
    let mut text = String::new();
    for line in extractor.data() {
        if line.starts_with('#') {
            if Highlighted::matches_definition(line) {
                highlighted.extend(Highlighted::build(line));
            }
        } else {
            text.push_str(line);
            text.push('\n');
        }
    }
    let json = crate::json::parse(&text).ok();
    create(source, Language::Json, json, highlighted, &extractor)
}

/// A `@startyaml` diagram (`YamlDiagramFactory`).
pub(super) fn create_yaml(source: UmlSource) -> Box<dyn Diagram> {
    let extractor = StyleExtractor::new(source.lines());
    let mut highlighted = Vec::new();
    let mut lines = Vec::new();
    for line in extractor.data() {
        if Highlighted::matches_definition(line) {
            highlighted.extend(Highlighted::build(line));
        } else {
            lines.push(line.as_str());
        }
    }
    let yaml = yaml::parse(&lines);
    create(source, Language::Yaml, yaml, highlighted, &extractor)
}

fn create(
    source: UmlSource,
    language: Language,
    json: Option<JsonValue>,
    highlighted: Vec<Highlighted>,
    extractor: &StyleExtractor,
) -> Box<dyn Diagram> {
    let source = Rc::new(source);
    let mut diagram = JsonDiagram {
        titled: Titled::new(language.style_name(), language.name(), &source),
        source,
        language,
        root: json.map(wrap_root),
        highlighted,
    };
    if extractor.handwritten {
        diagram
            .titled
            .not_ported(NotYetPorted("handwritten drawing"));
    }
    if let Some(scale) = &extractor.scale {
        diagram.apply_scale(scale);
    }
    if extractor.new_skin.is_some() {
        diagram
            .titled
            .not_ported(NotYetPorted("the skin directive"));
    }
    if !extractor.style.is_empty() {
        let body: Vec<&str> = extractor
            .style
            .iter()
            .map(String::as_str)
            .filter(|line| !matches!(crate::java::trim(line), "<style>" | "</style>"))
            .collect();
        // PlantUML reports a style it cannot parse and draws without it.
        let _ = diagram.titled.skin.apply_style_sheet(&body);
    }
    if let Some(title) = &extractor.title {
        diagram
            .titled
            .set_title_without_source(Display::with_newlines(title));
    }
    Box::new(diagram)
}

/// A plain value is drawn as a one-row array, an empty container as an array holding an empty string.
fn wrap_root(json: JsonValue) -> JsonValue {
    match json {
        JsonValue::String(_) | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::Null => {
            JsonValue::Array(vec![json])
        }
        JsonValue::Array(ref values) if values.is_empty() => {
            JsonValue::Array(vec![JsonValue::String(String::new())])
        }
        JsonValue::Object(ref object) if object.is_empty() => {
            JsonValue::Array(vec![JsonValue::String(String::new())])
        }
        _ => json,
    }
}

impl JsonDiagram {
    /// `scale ...` read like the line of any diagram.
    fn apply_scale(&mut self, scale: &StringLocated) {
        let lines = BlocLines::single(scale.clone());
        for command in add_common_scale_commands::<Self>() {
            if command.is_valid(&lines) == CommandControl::Ok {
                // An invalid scale is ignored, as PlantUML ignores what its commands return here.
                let _ = command.execute(self, lines.clone());
            }
        }
    }
}

/// The tables and their links, or a message for data that does not parse.
struct Drawing<'a> {
    layout: Option<SmetanaForJson<'a>>,
    message: Option<TextBlockMarged<Box<dyn TextBlock>>>,
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        LimitFinder::min_max_from_origin_of(self, string_bounder.shared()).dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        if let Some(message) = &self.message {
            message.draw_u(ug);
        }
        if let Some(layout) = &self.layout {
            layout.draw_me(ug);
        }
    }
}

impl AbstractDiagram for JsonDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for JsonDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for JsonDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        if let Some(not_ported) = self.titled.not_ported_part() {
            return Err(not_ported);
        }
        let drawing = if let Some(root) = &self.root {
            Drawing {
                layout: Some(SmetanaForJson::new(
                    &self.titled.skin,
                    self.language.style_name(),
                    root,
                    &self.highlighted,
                    string_bounder.as_ref(),
                )),
                message: None,
            }
        } else {
            let display = Display::with_newlines(&format!(
                "Your data does not sound like {} data",
                self.language.name()
            ));
            let text = display.create0(
                &FontConfiguration::black_blue_true(UFont::monospace(14)),
                HorizontalAlignment::Left,
                &self.titled.skin,
                0.0,
                crate::creole::CreoleMode::Full,
            );
            Drawing {
                layout: None,
                message: Some(TextBlockMarged::new(
                    Box::new(text),
                    ClockwiseTopRightBottomLeft::margin1_margin2(2.0, 5.0),
                )),
            }
        };
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

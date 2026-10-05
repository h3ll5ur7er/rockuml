//! `@startsalt`: mock-ups of user interfaces drawn from a grid of widgets.

mod data_source;
mod elements;

use std::sync::LazyLock;

use regex::Regex;

use super::common_commands::common_commands;
use super::diagram_type::DiagramType;
use super::error::ErrorDiagram;
use super::source::UmlSource;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted};
use crate::command::{
    Command, CommandError, CommandResult, SingleLine, SingleLineCommand, factory,
};
use crate::java;
use crate::jaws::BLOCK_E1_NEWLINE;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::pattern::{RegexResult, RegexTree, java_regex};
use crate::text::LineLocation;
use data_source::{DataSource, Terminated, Terminator};
use elements::{
    Button, Droplist, Element, Positionner, Pyramid, RadioCheckbox, TableStrategy, Text, TextField,
};

pub struct SaltDiagram {
    source: UmlSource,
    titled: Titled,
    lines: Vec<String>,
}

impl TitledDiagram for SaltDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl SaltDiagram {
    /// The diagram, or the error image for its first faulty line.
    pub fn create(source: UmlSource) -> Box<dyn Diagram> {
        let mut diagram = Self {
            source,
            titled: Titled::new(crate::style::SName::SaltDiagram, "SALT"),
            lines: Vec::new(),
        };
        let mut commands: Vec<Box<dyn Command<SaltDiagram>>> = common_commands();
        commands.push(Box::new(SingleLine(Anything::new())));
        let lines = diagram.source.lines().to_vec();
        match factory::execute_lines(&lines, &mut diagram, &commands) {
            Ok(()) => Box::new(diagram),
            Err(failure) => Box::new(ErrorDiagram::new(
                diagram.source,
                failure.trace,
                &failure.error.message,
                Some(DiagramType::Salt),
            )),
        }
    }

    /// The lines that describe widgets; skinparams, `scale` and sprites are not ported yet.
    fn widget_lines(&self) -> Result<Vec<String>, NotYetPorted> {
        let mut lines = Vec::new();
        for line in &self.lines {
            if line == "hide stereotype" || line.starts_with("skinparam ") {
                continue;
            }
            if line.starts_with("scale ") || line.starts_with("sprite $") {
                return Err(NotYetPorted("scale and sprites in salt"));
            }
            lines.push(line.clone());
        }
        Ok(lines)
    }
}

/// Every line no other command takes describes widgets.
struct Anything(RegexTree);

impl Anything {
    fn new() -> Self {
        Self(RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "ALL", "(.*)"),
            RegexTree::end(),
        ]))
    }
}

impl SingleLineCommand<SaltDiagram> for Anything {
    fn pattern(&self) -> &RegexTree {
        &self.0
    }

    fn trims_line(&self) -> bool {
        false
    }

    fn execute_arg(
        &self,
        diagram: &mut SaltDiagram,
        _location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        let line = arg.get("ALL", 0).unwrap_or_default();
        if diagram.lines.is_empty() && java::trim(line) == "salt" {
            return Err(CommandError::new("This is not needed anymore."));
        }
        let pieces = java::split(line, &BLOCK_E1_NEWLINE.to_string());
        diagram.lines.extend(if line.is_empty() {
            vec![String::new()]
        } else {
            pieces
        });
        Ok(())
    }
}

impl Diagram for SaltDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(&self) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let lines = self.widget_lines()?;
        let mut source = DataSource::new(&lines);
        let root = top_level_element(&mut source)?;
        Ok(self.titled.add_chrome(Box::new(Drawing(root))))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled.export_settings(self.source.seed(), 5.0)
    }
}

/// The whole mock-up, drawn in its two passes.
struct Drawing(Box<dyn Element>);

impl TextBlock for Drawing {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.0.preferred_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dimension = self.0.preferred_dimension(ug.string_bounder());
        let ug = ug.with_color(crate::color::HColor::BLACK);
        self.0.draw_u(&ug, 0, dimension);
        self.0.draw_u(&ug, 1, dimension);
    }
}

fn peek_text(source: &DataSource, ahead: usize) -> Result<&str, NotYetPorted> {
    source
        .peek(ahead)
        .map(|item| item.item.as_str())
        .ok_or(NotYetPorted("error diagram for unterminated salt"))
}

/// The diagram is one group: a grid, a scroll pane, a border layout or a tree.
fn top_level_element(source: &mut DataSource) -> Result<Box<dyn Element>, NotYetPorted> {
    if is_pyramid(source)? {
        return Ok(pyramid(source)?.item);
    }
    Err(NotYetPorted("salt scroll panes, border layouts and trees"))
}

fn is_pyramid(source: &DataSource) -> Result<bool, NotYetPorted> {
    static BORDER: LazyLock<Regex> = LazyLock::new(|| java_regex("^[NSEW]=$", false));
    let opening = peek_text(source, 0)?;
    if !["{", "{+", "{^", "{#", "{!", "{-"].contains(&opening) {
        return Ok(false);
    }
    let next = peek_text(source, 1)?;
    Ok(!BORDER.is_match(next) && !is_tree_marker(next))
}

/// `T`, or `T` followed by a table strategy character.
fn is_tree_marker(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some('T'), None, _) => true,
        (Some('T'), Some(strategy), None) => TableStrategy::from_char(strategy).is_some(),
        _ => false,
    }
}

fn pyramid(source: &mut DataSource) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    let header = source.next().expect("checked by is_pyramid");
    let strategy = header
        .item
        .chars()
        .nth(1)
        .map_or(Some(TableStrategy::None), TableStrategy::from_char)
        .ok_or(NotYetPorted("error diagram for a bad salt grid"))?;
    let title = if strategy == TableStrategy::OutsideWithTitle
        && header.terminator == Terminator::NewColumn
    {
        let title = source
            .next()
            .ok_or(NotYetPorted("error diagram for unterminated salt"))?;
        Some(remove_quotes(&title.item).to_owned())
    } else {
        None
    };
    let mut positionner = Positionner::default();
    while peek_text(source, 0)? != "}" {
        let next = next_element(source)?;
        if next.item.plain_text() == Some("*") {
            positionner.merge_left(next.terminator);
        } else {
            positionner.add(next.item, next.terminator);
        }
    }
    let closing = source.next().expect("just peeked");
    Ok(Terminated {
        item: Box::new(Pyramid::new(positionner, strategy, title.as_deref())),
        terminator: closing.terminator,
    })
}

/// `"quoted"` titles lose their quotes.
fn remove_quotes(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(text)
}

/// The next element inside a group, tried in PlantUML's factory order.
fn next_element(source: &mut DataSource) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    let text = peek_text(source, 0)?.to_owned();
    let is_line = ['-', '=', '~', '.'].iter().any(|c| {
        let marker: String = [*c, *c].iter().collect();
        text.starts_with(&marker) && text.ends_with(&marker)
    });
    if text == "{*"
        || text == "{/"
        || (text == "{" && is_tree_marker(peek_text(source, 1)?))
        || is_line
    {
        return Err(NotYetPorted("salt menus, tabs, trees and separators"));
    }
    let font = elements::widget_font();
    let widget: Option<Box<dyn Element>> =
        if text.starts_with('"') && text.ends_with('"') && text.len() > 1 {
            Some(Box::new(TextField::new(&text[1..text.len() - 1], font)))
        } else if text.starts_with("[X]") {
            Some(Box::new(RadioCheckbox::new(
                after(&text, ']'),
                font,
                false,
                true,
            )))
        } else if text.starts_with("[]") || text.starts_with("[ ]") {
            Some(Box::new(RadioCheckbox::new(
                after(&text, ']'),
                font,
                false,
                false,
            )))
        } else if !text.starts_with("[[") && text.starts_with('[') && text.ends_with(']') {
            Some(Box::new(Button::new(&text[1..text.len() - 1], font)))
        } else if text.starts_with('^') && text.ends_with('^') && text.len() > 1 {
            Some(Box::new(Droplist::new(&text[1..text.len() - 1], font)))
        } else if text.starts_with("(X)") {
            Some(Box::new(RadioCheckbox::new(
                after(&text, ')'),
                font,
                true,
                true,
            )))
        } else if text.starts_with("()") || text.starts_with("( )") {
            Some(Box::new(RadioCheckbox::new(
                after(&text, ')'),
                font,
                true,
                false,
            )))
        } else if is_image_or_dictionary_entry(&text) {
            return Err(NotYetPorted("salt images and dictionary entries"));
        } else if !text.starts_with('{') && !text.starts_with('}') && !java::trim(&text).is_empty()
        {
            Some(Box::new(Text::new(&text, font)))
        } else {
            None
        };
    if let Some(widget) = widget {
        let terminator = source.next().expect("just peeked").terminator;
        return Ok(Terminated {
            item: widget,
            terminator,
        });
    }
    if is_pyramid(source)? {
        return pyramid(source);
    }
    Err(NotYetPorted("salt scroll panes and border layouts"))
}

/// `<<` or `<<name` start an image; `<<name>>` reuses a named element.
fn is_image_or_dictionary_entry(text: &str) -> bool {
    static IMAGE: LazyLock<Regex> = LazyLock::new(|| java_regex(r"^\<\<\w+$", false));
    static ENTRY: LazyLock<Regex> = LazyLock::new(|| java_regex(r"^\<\<\w+\>\>$", false));
    text == "<<" || IMAGE.is_match(text) || ENTRY.is_match(text)
}

/// The text after the first `closing`, trimmed: the label of a radio button or checkbox.
fn after(text: &str, closing: char) -> &str {
    let start = text.find(closing).map_or(0, |index| index + 1);
    java::trim(&text[start..])
}

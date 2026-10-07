//! `@startsalt`: mock-ups of user interfaces drawn from a grid of widgets.

mod bars;
mod data_source;
mod elements;
mod tree;

use std::rc::Rc;

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
use bars::{MenuBar, TabBar};
use data_source::{DataSource, Terminated, Terminator};
use elements::{
    Button, Droplist, Element, Line, Positionner, Pyramid, PyramidScrolled, RadioCheckbox,
    ScrollStrategy, TableStrategy, Text, TextField,
};
use tree::Tree;

pub(super) struct SaltDiagram {
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
    pub(super) fn create(source: UmlSource) -> Box<dyn Diagram> {
        if source.is_empty() {
            let trace = source.lines().iter().take(2).cloned().collect();
            return Box::new(ErrorDiagram::new(
                source,
                trace,
                "Empty description",
                Some(DiagramType::Salt),
            ));
        }
        let mut diagram = Self {
            titled: Titled::new(
                crate::style::SName::SaltDiagram,
                "SALT",
                source.md5_map().clone(),
            ),
            source,
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

    /// The lines that describe widgets. PlantUML drops `hide stereotype` and skinparams, which have no
    /// say over widgets; sprites are not ported yet.
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
        diagram
            .lines
            .extend(java::split(line, &BLOCK_E1_NEWLINE.to_string()));
        Ok(())
    }
}

impl Diagram for SaltDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let lines = self.widget_lines()?;
        let mut source = DataSource::new(&lines);
        let root = top_level_element(&mut source)?;
        Ok(self
            .titled
            .add_chrome(Box::new(Drawing(root)), string_bounder))
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

/// Groups nest at most this deep, so that parsing and drawing them cannot overflow the stack. Debug
/// builds on Windows' 1 MiB main thread stack overflow at about 250 levels.
const MAX_NESTING: usize = 128;

/// The depth of a group opened inside one at `depth`.
fn inner_depth(depth: usize) -> Result<usize, NotYetPorted> {
    if depth < MAX_NESTING {
        Ok(depth + 1)
    } else {
        Err(NotYetPorted("deeply nested salt groups"))
    }
}

/// The diagram is one group: a grid, a scroll pane, a border layout or a tree.
fn top_level_element(source: &mut DataSource) -> Result<Box<dyn Element>, NotYetPorted> {
    group(source, 1)?
        .map(|group| group.item)
        .ok_or(NotYetPorted("crash report for salt without a group"))
}

/// A group at `depth`, tried in PlantUML's factory order.
fn group(
    source: &mut DataSource,
    depth: usize,
) -> Result<Option<Terminated<Box<dyn Element>>>, NotYetPorted> {
    if let Some(strategy) = pyramid_strategy(source)? {
        return pyramid(source, strategy, depth).map(Some);
    }
    if ["{S", "{S-", "{SI"].contains(&peek_text(source, 0)?) {
        return scroll(source, depth).map(Some);
    }
    if is_border(source)? {
        return Err(NotYetPorted("salt border layouts"));
    }
    if let Some(strategy) = tree_strategy(source)? {
        return tree(source, strategy, depth).map(Some);
    }
    Ok(None)
}

/// The lines of the grid that opens here, if one does.
fn pyramid_strategy(source: &DataSource) -> Result<Option<TableStrategy>, NotYetPorted> {
    let header = peek_text(source, 0)?;
    if !["{", "{+", "{^", "{#", "{!", "{-"].contains(&header) {
        return Ok(None);
    }
    let next = peek_text(source, 1)?;
    if is_border_marker(next) || is_tree_marker(next) {
        return Ok(None);
    }
    Ok(strategy_suffix(&header[1..]))
}

fn is_border(source: &DataSource) -> Result<bool, NotYetPorted> {
    Ok(
        ["{", "{+", "{#", "{!", "{-"].contains(&peek_text(source, 0)?)
            && is_border_marker(peek_text(source, 1)?),
    )
}

/// `N=`, `S=`, `E=` or `W=`: the side of a border layout the next element goes to.
fn is_border_marker(text: &str) -> bool {
    static BORDER: LazyLock<Regex> = LazyLock::new(|| java_regex("^[NSEW]=$", false));
    BORDER.is_match(text)
}

/// The lines of the tree that opens here, if one does.
fn tree_strategy(source: &DataSource) -> Result<Option<TableStrategy>, NotYetPorted> {
    if peek_text(source, 0)? != "{" {
        return Ok(None);
    }
    Ok(tree_marker_strategy(peek_text(source, 1)?))
}

fn is_tree_marker(text: &str) -> bool {
    tree_marker_strategy(text).is_some()
}

/// `T`, or `T` followed by a table strategy character.
fn tree_marker_strategy(text: &str) -> Option<TableStrategy> {
    text.strip_prefix('T').and_then(strategy_suffix)
}

/// Nothing, or a table strategy character.
fn strategy_suffix(text: &str) -> Option<TableStrategy> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (None, _) => Some(TableStrategy::None),
        (Some(strategy), None) => TableStrategy::from_char(strategy),
        _ => None,
    }
}

fn next_item(source: &mut DataSource) -> Result<Terminated<String>, NotYetPorted> {
    source
        .next()
        .ok_or(NotYetPorted("error diagram for unterminated salt"))
}

/// Ends a group at its closing `}`, which says how the group itself is terminated.
fn close_group(
    source: &mut DataSource,
    group: Box<dyn Element>,
) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    Ok(Terminated {
        item: group,
        terminator: next_item(source)?.terminator,
    })
}

fn pyramid(
    source: &mut DataSource,
    strategy: TableStrategy,
    depth: usize,
) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    let header = next_item(source)?;
    let title = if strategy == TableStrategy::OutsideWithTitle
        && header.terminator == Terminator::NewColumn
    {
        Some(remove_quotes(&next_item(source)?.item).to_owned())
    } else {
        None
    };
    let mut positionner = Positionner::default();
    while peek_text(source, 0)? != "}" {
        let next = next_element(source, depth)?;
        if next.item.plain_text() == Some("*") {
            positionner.merge_left(next.terminator);
        } else {
            positionner.add(next.item, next.terminator);
        }
    }
    close_group(
        source,
        Box::new(Pyramid::new(positionner, strategy, title.as_deref())),
    )
}

/// `"quoted"` titles lose their quotes.
fn remove_quotes(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(text)
}

fn scroll(
    source: &mut DataSource,
    depth: usize,
) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    let header = next_item(source)?;
    let mut positionner = Positionner::default();
    while peek_text(source, 0)? != "}" {
        let next = next_element(source, depth)?;
        positionner.add(next.item, next.terminator);
    }
    let strategy = ScrollStrategy::from_desc(&header.item);
    close_group(
        source,
        Box::new(PyramidScrolled::new(positionner, strategy)),
    )
}

/// After the `{` and the `T` marker, each row starts with its label; the cells after it are elements
/// like in a grid.
fn tree(
    source: &mut DataSource,
    strategy: TableStrategy,
    depth: usize,
) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    next_item(source)?;
    next_item(source)?;
    let mut tree = Tree::new(strategy);
    let mut takes_label = true;
    while peek_text(source, 0)? != "}" {
        let terminator = if takes_label {
            let label = next_item(source)?;
            tree.add_entry(&label.item);
            label.terminator
        } else {
            let cell = next_element(source, depth)?;
            tree.add_cell_to_entry(cell.item);
            cell.terminator
        };
        takes_label = terminator == Terminator::NewLine;
    }
    close_group(source, Box::new(tree))
}

/// The tabs; tabs on separate lines would make a vertical bar.
fn tab_bar(source: &mut DataSource) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    next_item(source)?;
    let mut tab_bar = TabBar::default();
    while peek_text(source, 0)? != "}" {
        let tab = next_item(source)?;
        if tab.terminator == Terminator::NewLine {
            return Err(NotYetPorted("vertical salt tab bars"));
        }
        tab_bar.add_tab(&tab.item);
    }
    close_group(source, Box::new(tab_bar))
}

/// The entries of the first line, then one line per popup: the entry it belongs to, then its entries.
fn menu_bar(source: &mut DataSource) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    enum Reading {
        Entries,
        PopupOwner,
        Popup(String),
    }
    next_item(source)?;
    let mut menu_bar = MenuBar::default();
    let mut reading = Reading::Entries;
    while peek_text(source, 0)? != "}" {
        let item = next_item(source)?;
        match &reading {
            Reading::Entries => menu_bar.add_entry(&item.item),
            Reading::PopupOwner => reading = Reading::Popup(item.item.clone()),
            Reading::Popup(owner) => menu_bar.add_sub_entry(owner, &item.item)?,
        }
        if item.terminator == Terminator::NewLine {
            reading = Reading::PopupOwner;
        }
    }
    close_group(source, Box::new(menu_bar))
}

/// The next element inside a group at `depth`, tried in PlantUML's factory order.
fn next_element(
    source: &mut DataSource,
    depth: usize,
) -> Result<Terminated<Box<dyn Element>>, NotYetPorted> {
    let text = peek_text(source, 0)?.to_owned();
    if text == "{*" {
        return menu_bar(source);
    }
    if let Some(strategy) = tree_strategy(source)? {
        return tree(source, strategy, inner_depth(depth)?);
    }
    if text == "{/" {
        return tab_bar(source);
    }
    let line_separator = ['-', '=', '~', '.'].into_iter().find(|&c| {
        let marker: String = [c, c].iter().collect();
        text.starts_with(&marker) && text.ends_with(&marker)
    });
    let font = elements::widget_font();
    let widget: Option<Box<dyn Element>> = if let Some(separator) = line_separator {
        Some(Box::new(Line::new(separator)))
    } else if text.starts_with('"') && text.ends_with('"') && text.len() > 1 {
        Some(Box::new(TextField::new(&text[1..text.len() - 1], font)))
    } else if let Some(label) = label_after(&text, &["[X]"]) {
        Some(Box::new(RadioCheckbox::checkbox_on(label, font)))
    } else if let Some(label) = label_after(&text, &["[]", "[ ]"]) {
        Some(Box::new(RadioCheckbox::checkbox_off(label, font)))
    } else if !text.starts_with("[[") && text.starts_with('[') && text.ends_with(']') {
        Some(Box::new(Button::new(&text[1..text.len() - 1], font)))
    } else if text.starts_with('^') && text.ends_with('^') && text.len() > 1 {
        Some(Box::new(Droplist::new(&text[1..text.len() - 1], font)))
    } else if let Some(label) = label_after(&text, &["(X)"]) {
        Some(Box::new(RadioCheckbox::radio_on(label, font)))
    } else if let Some(label) = label_after(&text, &["()", "( )"]) {
        Some(Box::new(RadioCheckbox::radio_off(label, font)))
    } else if is_image_or_dictionary_entry(&text) {
        return Err(NotYetPorted("salt images and dictionary entries"));
    } else if !text.starts_with('{') && !text.starts_with('}') && !java::trim(&text).is_empty() {
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
    group(source, inner_depth(depth)?)?
        .ok_or(NotYetPorted("crash report for an unknown salt element"))
}

/// `<<` or `<<name` start an image; `<<name>>` reuses a named element.
fn is_image_or_dictionary_entry(text: &str) -> bool {
    static IMAGE: LazyLock<Regex> = LazyLock::new(|| java_regex(r"^\<\<\w+$", false));
    static ENTRY: LazyLock<Regex> = LazyLock::new(|| java_regex(r"^\<\<\w+\>\>$", false));
    text == "<<" || IMAGE.is_match(text) || ENTRY.is_match(text)
}

/// The label of a radio button or checkbox: the text after its mark, trimmed.
fn label_after<'a>(text: &'a str, marks: &[&str]) -> Option<&'a str> {
    marks
        .iter()
        .find_map(|mark| text.strip_prefix(mark))
        .map(java::trim)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::diagram::{ImageFormat, export};
    use crate::fonts::FontRegistry;
    use crate::host::IsolatedHost;
    use crate::text::StringLocated;

    fn export_debug(lines: &[String]) -> Result<Vec<u8>, NotYetPorted> {
        let location = LineLocation::new("test", None);
        let source = UmlSource::new(
            lines
                .iter()
                .map(|line| StringLocated::new(line, location.clone()))
                .collect(),
            lines.to_vec(),
        );
        let diagram = SaltDiagram::create(source);
        export(
            diagram.as_ref(),
            0,
            ImageFormat::Debug,
            &Arc::new(FontRegistry::default()),
            &IsolatedHost,
        )
    }

    fn salt(body: &[&str]) -> Vec<String> {
        std::iter::once("@startsalt")
            .chain(body.iter().copied())
            .chain(std::iter::once("@endsalt"))
            .map(str::to_owned)
            .collect()
    }

    fn nested(depth: usize) -> Vec<String> {
        let body: Vec<&str> = std::iter::repeat_n("{", depth)
            .chain(std::iter::once("leaf"))
            .chain(std::iter::repeat_n("}", depth))
            .collect();
        salt(&body)
    }

    #[test]
    fn a_merge_at_the_start_of_a_row_widens_the_cell_above() {
        assert!(export_debug(&salt(&["{", "a", "*", "}"])).is_ok());
    }

    #[test]
    fn nested_grids_are_measured_once() {
        assert!(export_debug(&nested(30)).is_ok());
    }

    #[test]
    fn nested_trees_are_measured_once() {
        let body: Vec<&str> = std::iter::once("{T")
            .chain(std::iter::repeat_n("+ a | {T", 30))
            .chain(std::iter::once("+ leaf"))
            .chain(std::iter::repeat_n("}", 31))
            .collect();
        assert!(export_debug(&salt(&body)).is_ok());
    }

    #[test]
    fn nesting_is_limited() {
        assert!(export_debug(&nested(MAX_NESTING)).is_ok());
        assert!(export_debug(&nested(MAX_NESTING + 1)).is_err());
        assert!(export_debug(&nested(20_000)).is_err());
    }
}

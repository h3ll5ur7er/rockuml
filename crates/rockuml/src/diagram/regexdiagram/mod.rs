//! Regular expressions drawn as railroad diagrams, `@startregex` (PlantUML's `regexdiagram` package), with
//! the tiles of EBNF diagrams.

mod expression;

use std::rc::Rc;

use expression::{Failure, ReToken, ReTokenType};

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::ebnf::tile::{
    BoxKind, ETile, ETileAlternation, ETileBox, ETileConcatenation, ETileLookAheadOrBehind,
    ETileNamedGroup, ETileRegexGroup, ETileRegexGroupAllBut, TileStyle,
};
use super::ebnf::tile_style;
use super::json::StyleExtractor;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{BlocLines, Command, CommandError, CommandResult, PatternCommand, SingleLine};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::pattern::{RegexResult, RegexTree};
use crate::style::SName;
use crate::text::{LineLocation, StringLocated};

/// Drawing a diagram without expression, where PlantUML throws and draws a crash report instead.
const CRASH_REPORT: NotYetPorted =
    NotYetPorted("the crash report for a regular expression diagram without expression");

pub(super) struct PSystemRegex {
    source: Rc<UmlSource>,
    titled: Titled,
    /// Its top is last; only the top is drawn.
    stack: Vec<ETile>,
    style: TileStyle,
}

/// Reads regular expressions (PlantUML's `PSystemRegexFactory`).
pub(super) struct PSystemRegexFactory;

impl CommandFactory for PSystemRegexFactory {
    type Diagram = PSystemRegex;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Regex;

    /// The `<style>` blocks apply before any line is read, and the tiles keep the style they make.
    fn create_empty_diagram(source: &Rc<UmlSource>) -> PSystemRegex {
        let mut titled = Titled::new(SName::Regex, "REGEX", source);
        let extractor = StyleExtractor::new(source.lines());
        if extractor.new_skin.is_some() {
            titled.not_ported(NotYetPorted("the skin directive"));
        }
        if let Some(rules) = extractor.style_rules() {
            // PlantUML reports a style it cannot parse and draws without it.
            let _ = titled.skin.apply_style_sheet(&rules);
        }
        let style = tile_style(&titled.skin);
        PSystemRegex {
            source: source.clone(),
            titled,
            stack: Vec::new(),
            style,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<PSystemRegex>>> {
        let mut commands = add_common_commands1();
        commands.push(command_regexf_single_line());
        commands
    }
}

/// PlantUML's `CommandRegexfSingleLine`: every other line is a regular expression.
fn command_regexf_single_line() -> Box<dyn Command<PSystemRegex>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LINE", "(.*)"),
            RegexTree::end(),
        ]),
        |diagram: &mut PSystemRegex, location: &LineLocation, arg: &RegexResult| {
            let line = arg.get("LINE", 0).unwrap_or_default();
            diagram.add_bloc_lines(&BlocLines::single(StringLocated::new(
                line,
                location.clone(),
            )))
        },
    )))
}

impl PSystemRegex {
    fn add_bloc_lines(&mut self, from: &BlocLines) -> CommandResult {
        match self.read(from) {
            Ok(()) => Ok(()),
            Err(Failure::Parsing(message)) => {
                Err(CommandError::new(format!("Error parsing: {message}")))
            }
            Err(Failure::Thrown(exception)) => Err(CommandError::thrown(exception)),
        }
    }

    fn read(&mut self, from: &BlocLines) -> Result<(), Failure> {
        let parsed1 = expression::parse(&mut from.inspector())?;
        let parsed2 = expression::add_implicit_concatenation(parsed1);
        for token in expression::shunting_yard(parsed2)? {
            self.apply(&token)?;
        }
        Ok(())
    }

    fn apply(&mut self, token: &ReToken) -> Result<(), Failure> {
        use ReTokenType as T;
        let data = token.data.as_str();
        match token.token_type {
            T::SimpleChar | T::EscapedChar => self.push_etile_box(data, BoxKind::Terminal),
            T::Class | T::Anchor => self.push_etile_box(data, BoxKind::Literal),
            T::Group => self.push_regex_group(data),
            T::LookAhead | T::LookBehind => {
                let arg1 = self.pop()?;
                self.stack
                    .push(ETile::LookAheadOrBehind(ETileLookAheadOrBehind::new(
                        arg1,
                        self.style.clone(),
                        data.to_owned(),
                    )));
            }
            T::NamedGroup => {
                let arg1 = self.pop()?;
                self.stack.push(ETile::NamedGroup(ETileNamedGroup::new(
                    arg1,
                    self.style.font.clone(),
                    data.to_owned(),
                )));
            }
            T::ConcatenationImplicit => self.concatenation()?,
            T::Alternative => self.alternation()?,
            T::Quantifier if data.starts_with('*') => {
                let arg1 = self.pop()?;
                self.stack.push(ETile::zero_or_more(arg1));
            }
            T::Quantifier if data.starts_with('+') => {
                let arg1 = self.pop()?;
                self.stack.push(ETile::one_or_more(arg1, None));
            }
            T::Quantifier if data.starts_with('?') => {
                let arg1 = self.pop()?;
                self.stack.push(ETile::optional(arg1));
            }
            T::Quantifier => self.repetition_braced(data)?,
            T::ParenthesisOpen | T::ParenthesisClose => {
                return Err(Failure::Parsing(token.to_string()));
            }
        }
        Ok(())
    }

    fn pop(&mut self) -> Result<ETile, Failure> {
        self.stack.pop().ok_or(Failure::NO_SUCH_ELEMENT)
    }

    fn push_etile_box(&mut self, value: &str, kind: BoxKind) {
        self.stack.push(ETile::Box(ETileBox::new(
            value.to_owned(),
            kind,
            self.style.clone(),
        )));
    }

    /// `[^…]` lists what it excludes apart.
    fn push_regex_group(&mut self, group: &str) {
        let elements = expression::split_group(group);
        let style = self.style.clone();
        self.stack
            .push(if elements.first().is_some_and(|first| first == "^") {
                ETile::RegexGroupAllBut(ETileRegexGroupAllBut::new(elements, style))
            } else {
                ETile::RegexGroup(ETileRegexGroup::new(elements, style))
            });
    }

    /// `{n}`, `{n,}` or `{n,m}`: an optional part for no minimum, labelled with the count.
    fn repetition_braced(&mut self, repetition: &str) -> Result<(), Failure> {
        let min = parse_braced_min(repetition);
        let max = parse_braced_max(repetition);
        let arg1 = self.pop()?;
        let label = || Some((repetition.to_owned(), self.style.font.bigger(-2.0)));
        let tile = if min == 0 && (is_braced_max_unbounded(repetition) || max == 0) {
            ETile::zero_or_more(arg1)
        } else if min == 0 && max == 1 {
            ETile::optional(arg1)
        } else if min == 0 {
            ETile::optional(ETile::one_or_more(arg1, label()))
        } else {
            ETile::one_or_more(arg1, label())
        };
        self.stack.push(tile);
        Ok(())
    }

    fn alternation(&mut self) -> Result<(), Failure> {
        let mut arg1 = self.pop()?;
        let mut arg2 = self.pop()?;
        if matches!(arg1, ETile::Alternation(_)) {
            arg1.push(arg2);
            self.stack.push(arg1);
        } else if matches!(arg2, ETile::Alternation(_)) {
            arg2.push(arg1);
            self.stack.push(arg2);
        } else {
            self.stack
                .push(ETile::Alternation(ETileAlternation::of(arg2, arg1)));
        }
        Ok(())
    }

    /// Letters in a row merge into one box.
    fn concatenation(&mut self) -> Result<(), Failure> {
        let mut arg1 = self.pop()?;
        let arg2 = self.pop()?;
        if let (Some(box1), Some(box2)) = (mergeable_box(&arg1), mergeable_box(&arg2)) {
            self.stack.push(ETile::Box(box2.merge_with(box1)));
            return Ok(());
        }
        if let (ETile::Concatenation(concat1), Some(box2)) = (&mut arg1, mergeable_box(&arg2))
            && let Some(box1) = mergeable_box(concat1.first())
        {
            let merged = box2.merge_with(box1);
            *concat1.first_mut() = ETile::Box(merged);
            self.stack.push(arg1);
            return Ok(());
        }
        if matches!(arg1, ETile::Concatenation(_)) {
            arg1.push(arg2);
            self.stack.push(arg1);
        } else {
            self.stack
                .push(ETile::Concatenation(ETileConcatenation::of(arg2, arg1)));
        }
        Ok(())
    }
}

/// A box of letters, which the letters next to it join.
fn mergeable_box(tile: &ETile) -> Option<&ETileBox> {
    match tile {
        ETile::Box(tile_box) if tile_box.kind() == BoxKind::Terminal => Some(tile_box),
        _ => None,
    }
}

/// `{…}` without its braces; the reader refuses a lazy mark after them.
fn strip_braces(repetition: &str) -> &str {
    repetition
        .strip_prefix('{')
        .and_then(|inside| inside.strip_suffix('}'))
        .unwrap_or(repetition)
}

/// Java's `Integer.parseInt` of a count, -1 where it throws.
fn parse_count(text: &str) -> i32 {
    text.parse().unwrap_or(-1)
}

fn parse_braced_min(repetition: &str) -> i32 {
    let inside = strip_braces(repetition);
    let min = crate::java::trim(inside.split_once(',').map_or(inside, |(min, _)| min));
    if min.is_empty() { 0 } else { parse_count(min) }
}

fn parse_braced_max(repetition: &str) -> i32 {
    let inside = strip_braces(repetition);
    match inside.split_once(',') {
        None => parse_braced_min(repetition),
        Some((_, max)) => {
            let max = crate::java::trim(max);
            if max.is_empty() { -1 } else { parse_count(max) }
        }
    }
}

fn is_braced_max_unbounded(repetition: &str) -> bool {
    strip_braces(repetition)
        .split_once(',')
        .is_some_and(|(_, max)| crate::java::trim(max).is_empty())
}

impl AbstractDiagram for PSystemRegex {}

impl TitledDiagram for PSystemRegex {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

/// The top tile in the line colour, with room around it.
struct Drawing<'a> {
    tile: &'a ETile,
    style: &'a TileStyle,
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.tile
            .calculate_dimension(string_bounder)
            .delta(10.0, 10.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.tile.draw_u(
            &ug.translated(5.0, 5.0)
                .with_color(self.style.line_color.clone()),
        );
    }
}

impl Diagram for PSystemRegex {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let tile = self.stack.last().ok_or(CRASH_REPORT)?;
        let drawing = Drawing {
            tile,
            style: &self.style,
        };
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braced_counts_read_like_java() {
        let read = |repetition| {
            (
                parse_braced_min(repetition),
                parse_braced_max(repetition),
                is_braced_max_unbounded(repetition),
            )
        };
        assert_eq!(read("{3}"), (3, 3, false));
        assert_eq!(read("{2,5}"), (2, 5, false));
        assert_eq!(read("{0,}"), (0, -1, true));
        assert_eq!(read("{,4}"), (0, 4, false));
        assert_eq!(read("{1,2,3}"), (1, -1, false));
    }
}

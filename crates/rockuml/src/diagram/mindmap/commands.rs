//! The commands of mind maps (PlantUML's `CommandMindMap*`).

use std::sync::LazyLock;

use regex::Regex;

use super::MindMapDiagram;
use super::idea::IdeaShape;
use crate::color::HColor;
use crate::command::{BlocLines, Command, CommandError, Multiline, PatternCommand, SingleLine};
use crate::creole::Display;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stereo::Stereotype;
use crate::text::LineLocation;

/// `[#color]` after the markers, captured as `BACKCOLOR`.
fn back_color_pattern() -> RegexTree {
    RegexTree::optional(RegexTree::named(1, "BACKCOLOR", r"\[(#\w+)\]"))
}

/// The colour named in `[#color]`, if any.
fn back_color(arg: &RegexResult) -> Result<Option<HColor>, CommandError> {
    arg.get("BACKCOLOR", 0)
        .map(|name| {
            HColor::parse(name)
                .ok()
                .flatten()
                .ok_or_else(CommandError::bad_color)
        })
        .transpose()
}

fn length(text: &str) -> i32 {
    i32::try_from(text.encode_utf16().count()).unwrap_or(i32::MAX)
}

/// PlantUML's `CommandMindMapOrgmodeMultiline`: `**:first line` up to `last line;`, maybe followed by a
/// stereotype.
pub(super) fn orgmode_multiline() -> Box<dyn Command<MindMapDiagram>> {
    static END: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r"^(.*);\s*(\<\<(.+)\>\>)?$"));
    fn start() -> RegexTree {
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"([*#]+)"),
            back_color_pattern(),
            RegexTree::named(1, "SHAPE", r"(_)?"),
            RegexTree::leaf(":"),
            RegexTree::named(1, "DATA", r"(.*)"),
            RegexTree::end(),
        ])
    }
    let first_line = start();
    Box::new(
        Multiline::starting_with_owned(
            start(),
            &END,
            move |diagram: &mut MindMapDiagram, lines: &BlocLines| {
                let first = lines.first().expect("a block has its first line").trimmed();
                let first_match = first_line
                    .matcher(first.text())
                    .expect("the block's first line matched");
                let last = lines.last().expect("a block has its last line");
                let last_match = END
                    .captures(last.text())
                    .expect("the block's last line matched");
                let mut lines = lines
                    .remove_starting_and_ending(first_match.get("DATA", 0).unwrap_or_default(), 1);
                let stereotype = last_match.get(2).map(|stereotype| stereotype.as_str());
                if stereotype.is_some() {
                    lines = lines.override_last_line(&last_match[1]);
                }
                let level = length(first_match.get("TYPE", 0).unwrap_or_default()) - 1;
                let back_color = back_color(&first_match)?;
                let shape = IdeaShape::from_desc(first_match.get("SHAPE", 0));
                match stereotype {
                    None => diagram.add_idea(back_color, level, lines.to_display(), shape),
                    Some(stereotype) => diagram.add_idea_stereotyped(
                        Stereotype::new(stereotype),
                        back_color,
                        level,
                        lines.to_display(),
                        shape,
                    ),
                }
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandMindMapOrgmode`: `** label`, its markers maybe indented.
pub(super) fn orgmode() -> Box<dyn Command<MindMapDiagram>> {
    Box::new(SingleLine(
        PatternCommand::new(
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(1, "TYPE", r"([ \t]*[*#]+)"),
                back_color_pattern(),
                RegexTree::named(1, "SHAPE", r"(_)?"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL", r"(.*)"),
                RegexTree::end(),
            ]),
            |diagram: &mut MindMapDiagram, _: &LineLocation, arg: &RegexResult| {
                let back_color = back_color(arg)?;
                let level = diagram
                    .get_smart_level(arg.get("TYPE", 0).unwrap_or_default())
                    .ok_or_else(|| CommandError::new("Bad indentation"))?;
                diagram.add_idea(
                    back_color,
                    level,
                    Display::with_newlines(arg.get("LABEL", 0).unwrap_or_default()),
                    IdeaShape::from_desc(arg.get("SHAPE", 0)),
                )
            },
        )
        .untrimmed(),
    ))
}

/// PlantUML's `CommandMindMapRoot`: `0 label`.
pub(super) fn root() -> Box<dyn Command<MindMapDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(0)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LABEL", r"(.*)"),
            RegexTree::end(),
        ]),
        |diagram: &mut MindMapDiagram, _: &LineLocation, arg: &RegexResult| {
            diagram.add_idea_in(
                None,
                0,
                Display::with_newlines(arg.get("LABEL", 0).unwrap_or_default()),
                IdeaShape::Box,
                true,
            )
        },
    )))
}

/// PlantUML's `CommandMindMapPlus`: `++ label` grows to the right, `-- label` to the left.
pub(super) fn plus() -> Box<dyn Command<MindMapDiagram>> {
    Box::new(SingleLine(
        PatternCommand::new(
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(1, "TYPE", r"([+-]+)"),
                back_color_pattern(),
                RegexTree::named(1, "SHAPE", r"(_)?"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL", r"(.*)"),
                RegexTree::end(),
            ]),
            |diagram: &mut MindMapDiagram, _: &LineLocation, arg: &RegexResult| {
                let marker = arg.get("TYPE", 0).unwrap_or_default();
                let back_color = back_color(arg)?;
                diagram.add_idea_in(
                    back_color,
                    length(marker) - 1,
                    Display::with_newlines(arg.get("LABEL", 0).unwrap_or_default()),
                    IdeaShape::from_desc(arg.get("SHAPE", 0)),
                    !marker.contains('-'),
                )
            },
        )
        .untrimmed(),
    ))
}

/// PlantUML's `CommandMindMapDirection`: `left side` or `right to left direction` make ideas grow that
/// way.
pub(super) fn direction() -> Box<dyn Command<MindMapDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"[^*#]*"),
            RegexTree::leaf(r"\b"),
            RegexTree::named(1, "DIRECTION", r"(left|right|top|bottom)"),
            RegexTree::leaf(r"\b"),
            RegexTree::leaf(r"[^*#]*"),
            RegexTree::named(1, "KIND", r"(side|direction)"),
            RegexTree::leaf(r"[^*#]*"),
            RegexTree::end(),
        ]),
        |diagram: &mut MindMapDiagram, _: &LineLocation, arg: &RegexResult| {
            let direction = arg.get("DIRECTION", 0).unwrap_or_default();
            diagram.set_default_direction(
                direction.eq_ignore_ascii_case("right") || direction.eq_ignore_ascii_case("bottom"),
            );
            Ok(())
        },
    )))
}

//! The commands of work breakdowns (PlantUML's `CommandWBS*`). The `Old` forms put the colour and code before
//! the shape and direction, which PlantUML still reads but warns about.

use std::sync::LazyLock;

use regex::Regex;

use super::element::Direction;
use super::{NewElement, WbsDiagram};
use crate::color::{ColorType, HColor};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, PatternCommand, SingleLine,
};
use crate::diagram::chrome::Warning;
use crate::diagram::cuca_commands::colors;
use crate::diagram::mindmap::IdeaShape;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stereo::Stereotype;
use crate::text::LineLocation;

const OLD_ORDER_WARNING: &str = "Please define Direction/Shape before Color/Id.";

fn type_pattern() -> RegexTree {
    RegexTree::named(1, "TYPE", r"([ \t]*[*+-]+)")
}

/// `_` for no box and `<` or `>` for a side.
fn shape_and_direction() -> RegexTree {
    RegexTree::or(vec![
        RegexTree::concat(vec![
            RegexTree::named(1, "SHAPE_1", r"(_)?"),
            RegexTree::named(1, "DIRECTION_1", r"([<>])?"),
        ]),
        RegexTree::concat(vec![RegexTree::named(1, "DIRECTION_2", r"([<>])?")]),
        RegexTree::named(1, "SHAPE_2", r"(_)?"),
    ])
}

/// `[#color]` and `(code)`, in either order.
fn color_and_code() -> RegexTree {
    RegexTree::or(vec![
        RegexTree::concat(vec![
            RegexTree::optional(RegexTree::named(1, "BACKCOLOR_1", r"\[(#\w+)\]")),
            RegexTree::optional(RegexTree::named(1, "CODE_1", r"\(([%pLN_]+)\)")),
        ]),
        RegexTree::concat(vec![
            RegexTree::optional(RegexTree::named(1, "CODE_2", r"\(([%pLN_]+)\)")),
            RegexTree::optional(RegexTree::named(1, "BACKCOLOR_2", r"\[(#\w+)\]")),
        ]),
    ])
}

/// `"label" as code`.
fn quoted_label_as_code() -> Vec<RegexTree> {
    vec![
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "LABEL", r"[%g](.*)[%g]"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf("as"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "CODE", r"([%pLN_]+)"),
        RegexTree::end(),
    ]
}

fn optional_label() -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "LABEL", r"([^%s].*)"),
    ]))
}

fn back_color(arg: &RegexResult) -> Result<Option<HColor>, CommandError> {
    arg.get_lazzy("BACKCOLOR", 0)
        .map(|name| {
            HColor::parse(name)
                .ok()
                .flatten()
                .ok_or_else(CommandError::bad_color)
        })
        .transpose()
}

/// `-` hangs an element to the left, `<` and `>` say so explicitly (`Direction.getWBSDirection`).
fn wbs_direction(arg: &RegexResult) -> Direction {
    match arg.get_lazzy("DIRECTION", 0) {
        Some("<") => Direction::Left,
        Some(">") => Direction::Right,
        _ if arg.get("TYPE", 0).unwrap_or_default().contains('-') => Direction::Left,
        _ => Direction::Right,
    }
}

/// An element written on one line; `old_order` warns that its colour or code comes first.
fn add_single_line_item(
    diagram: &mut WbsDiagram,
    arg: &RegexResult,
    old_order: bool,
) -> CommandResult {
    let label = arg.get("LABEL", 0);
    let code = arg.get_lazzy("CODE", 0).map(str::to_owned);
    let back_color = back_color(arg)?;
    let direction = wbs_direction(arg);
    let mut shape = IdeaShape::from_desc(arg.get_lazzy("SHAPE", 0));
    if label.is_none() && shape == IdeaShape::None {
        shape = IdeaShape::Pseudo;
    } else if label.is_none() {
        return Err(CommandError::new("Missing label for WBS node."));
    }
    if old_order {
        diagram
            .titled
            .add_warning(Warning(OLD_ORDER_WARNING.to_owned()));
    }
    let level = smart_level(diagram, arg.get("TYPE", 0).unwrap_or_default())?;
    diagram.add_idea_labelled(
        code,
        back_color,
        level,
        label.unwrap_or_default(),
        direction,
        shape,
    )
}

fn smart_level(diagram: &mut WbsDiagram, marker: &str) -> Result<usize, CommandError> {
    diagram
        .get_smart_level(marker)
        .ok_or_else(|| CommandError::new("Bad tree structure"))
}

/// PlantUML's `CommandWBSItemNew`: `** label`, or `** "label" as code` with `quoted`.
pub(super) fn item_new(quoted: bool) -> Box<dyn Command<WbsDiagram>> {
    let mut parts = vec![RegexTree::start(), type_pattern(), shape_and_direction()];
    if quoted {
        parts.push(RegexTree::optional(RegexTree::named(
            1,
            "BACKCOLOR",
            r"\[(#\w+)\]",
        )));
        parts.extend(quoted_label_as_code());
    } else {
        parts.extend([color_and_code(), optional_label(), RegexTree::end()]);
    }
    item(parts, false)
}

/// PlantUML's `CommandWBSItemOld`: the colour and code before the shape and direction.
pub(super) fn item_old(quoted: bool) -> Box<dyn Command<WbsDiagram>> {
    let mut parts = vec![RegexTree::start(), type_pattern()];
    if quoted {
        parts.extend([
            RegexTree::optional(RegexTree::named(1, "BACKCOLOR", r"\[(#\w+)\]")),
            shape_and_direction(),
        ]);
        parts.extend(quoted_label_as_code());
    } else {
        parts.extend([
            color_and_code(),
            shape_and_direction(),
            optional_label(),
            RegexTree::end(),
        ]);
    }
    item(parts, true)
}

fn item(parts: Vec<RegexTree>, old_order: bool) -> Box<dyn Command<WbsDiagram>> {
    Box::new(SingleLine(
        PatternCommand::new(
            RegexTree::concat(parts),
            move |diagram: &mut WbsDiagram, _: &LineLocation, arg: &RegexResult| {
                add_single_line_item(diagram, arg, old_order)
            },
        )
        .untrimmed(),
    ))
}

/// The last line of a multiline element: its end and maybe a stereotype.
static MULTILINE_END: LazyLock<Regex> =
    LazyLock::new(|| plantuml_regex(r"^(.*);\s*(\<\<(.+)\>\>)?$"));

/// PlantUML's `CommandWBSItemMultilineNew`: `**:first line` up to `last line;`.
pub(super) fn item_multiline_new() -> Box<dyn Command<WbsDiagram>> {
    multiline(
        || {
            vec![
                RegexTree::start(),
                type_pattern(),
                shape_and_direction(),
                color_and_code(),
            ]
        },
        false,
    )
}

/// PlantUML's `CommandWBSItemMultilineOld`.
pub(super) fn item_multiline_old() -> Box<dyn Command<WbsDiagram>> {
    multiline(
        || {
            vec![
                RegexTree::start(),
                type_pattern(),
                color_and_code(),
                shape_and_direction(),
            ]
        },
        true,
    )
}

fn multiline(start: fn() -> Vec<RegexTree>, old_order: bool) -> Box<dyn Command<WbsDiagram>> {
    let pattern = || {
        let mut parts = start();
        parts.extend([
            RegexTree::leaf(":"),
            RegexTree::named(1, "DATA", r"(.*)"),
            RegexTree::end(),
        ]);
        RegexTree::concat(parts)
    };
    let first_line = pattern();
    Box::new(
        Multiline::starting_with_owned(
            pattern(),
            &MULTILINE_END,
            move |diagram: &mut WbsDiagram, lines: &BlocLines| {
                let first = lines.first().expect("a block has its first line").trimmed();
                let first_match = first_line
                    .matcher(first.text())
                    .expect("the block's first line matched");
                let last = lines.last().expect("a block has its last line");
                let last_match = MULTILINE_END
                    .captures(last.text())
                    .expect("the block's last line matched");
                let mut lines = lines
                    .remove_starting_and_ending(first_match.get("DATA", 0).unwrap_or_default(), 1);
                let stereotype = last_match.get(2).map(|stereotype| stereotype.as_str());
                if stereotype.is_some() {
                    lines = lines.override_last_line(&last_match[1]);
                }
                let back_color = back_color(&first_match)?;
                let code = first_match.get_lazzy("CODE", 0).map(str::to_owned);
                let direction = wbs_direction(&first_match);
                if old_order {
                    diagram
                        .titled
                        .add_warning(Warning(OLD_ORDER_WARNING.to_owned()));
                }
                let level = smart_level(diagram, first_match.get("TYPE", 0).unwrap_or_default())?;
                diagram.add_idea(NewElement {
                    code,
                    back_color,
                    level,
                    label: lines.to_display(),
                    stereotype: stereotype.map(Stereotype::new),
                    direction,
                    shape: IdeaShape::from_desc(first_match.get_lazzy("SHAPE", 0)),
                })
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandWBSLink`: `code1 -> code2`, maybe coloured.
pub(super) fn link() -> Box<dyn Command<WbsDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "CODE1", r"([%pLN_]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"[.-]+\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "CODE2", r"([%pLN_]+)"),
            RegexTree::spaces_zero_or_more(),
            crate::color::optional_pattern("COLOR"),
            crate::stereo::optional_pattern("STEREOTYPE"),
            RegexTree::named(1, "LABEL_LINK", r"(?::[%s]*(.+))?"),
            RegexTree::end(),
        ]),
        |diagram: &mut WbsDiagram, _: &LineLocation, arg: &RegexResult| {
            let colors = colors(arg, ColorType::Line)?;
            let stereotype = arg.get("STEREOTYPE", 0).map(Stereotype::new);
            diagram.link(
                arg.get("CODE1", 0).unwrap_or_default(),
                arg.get("CODE2", 0).unwrap_or_default(),
                &colors,
                stereotype.as_ref(),
            )
        },
    )))
}

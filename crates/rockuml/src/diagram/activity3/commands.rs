//! The commands of activity diagrams, in the order `ActivityDiagramFactory3` tries them.

use std::sync::LazyLock;

use regex::Regex;

use super::ActivityDiagram3;
use super::instruction::NoteType;
use super::link_rendering::LinkRendering;
use super::parallel::ForkStyle;
use crate::color::{ColorType, Colors, HColor};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, DecoratorMultine, Multiline, PatternCommand,
    SingleLine,
};
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::decoration::symbol::{USymbol, USymbols};
use crate::diagram::chrome::Warning;
use crate::diagram::description::style_colors_multiples;
use crate::diagram::sequence::model::NotePosition;
use crate::ftile::BoxStyle;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::text::{LineLocation, without_quotes_or_brackets};
use crate::{color, stereo};

type Apply = fn(&mut ActivityDiagram3, &LineLocation, &RegexResult) -> CommandResult;

fn command(pattern: RegexTree, apply: Apply) -> Box<dyn Command<ActivityDiagram3>> {
    Box::new(SingleLine(PatternCommand::new(pattern, apply)))
}

/// A command also accepted over up to 50 lines (`CommandDecoratorMultine`).
fn over_lines(pattern: RegexTree, apply: Apply) -> Box<dyn Command<ActivityDiagram3>> {
    Box::new(DecoratorMultine::new(
        PatternCommand::new(pattern, apply),
        50,
    ))
}

/// `Display.getWithNewlines`: no text, no display.
fn display(text: Option<&str>) -> Option<Display> {
    text.map(Display::with_newlines)
}

/// An empty condition is none.
fn non_empty(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.is_empty())
}

/// `HColorSet.getColor`: an unknown colour fails the command.
fn color_named(text: &str) -> Result<HColor, CommandError> {
    HColor::parse(text)
        .ok()
        .flatten()
        .ok_or_else(CommandError::bad_color)
}

fn optional_color(arg: &RegexResult, name: &str) -> Result<Option<HColor>, CommandError> {
    arg.get(name, 0).map(color_named).transpose()
}

/// The colours a specification captured under `name` gives, the main one painting the background
/// (`ColorParser.simpleColor(ColorType.BACK, name)`).
fn back_colors(arg: &RegexResult, name: &str) -> Result<Colors, CommandError> {
    Ok(arg
        .get(name, 0)
        .map(|data| Colors::parse(data, ColorType::Back))
        .transpose()?
        .unwrap_or_default())
}

fn url(arg: &RegexResult) -> Option<Url> {
    arg.get("URL", 0).and_then(Url::parse)
}

fn stereogroup(arg: &RegexResult) -> Stereogroup {
    Stereogroup::build(arg.get("STEREOGROUP", 0))
}

/// The colours of an arrow written like `#red;#blue,dashed`.
fn rainbow(diagram: &ActivityDiagram3, color_string: &str) -> Result<Rainbow, CommandError> {
    let skin = &diagram.titled.skin;
    Ok(Rainbow::build_from_definition(
        skin,
        color_string,
        skin.color_arrow_separation_space(),
    )?)
}

/// An arrow written `(-[#red]-> label)`: its colours under `<name>_COLOR` and its label under `name`
/// (`CommandBackward3.getBackRendering`).
fn back_rendering(
    diagram: &ActivityDiagram3,
    arg: &RegexResult,
    name: &str,
) -> Result<LinkRendering, CommandError> {
    let rendering = match arg.get(&format!("{name}_COLOR"), 0) {
        Some(color_string) => LinkRendering::create(rainbow(diagram, color_string)?),
        None => LinkRendering::none(),
    };
    Ok(rendering.with_display(display(arg.get(name, 0))))
}

const DEPRECATED_COLOR: (&str, &str) = (
    "This syntax is deprecated, you must add <<",
    ">> at the end of the line, after the ';'",
);

fn warn(diagram: &mut ActivityDiagram3, message: String) {
    diagram.titled.add_warning(Warning(message));
}

fn warn_deprecated_color(diagram: &mut ActivityDiagram3, color: Option<&str>) {
    if let Some(color) = color {
        let (before, after) = DEPRECATED_COLOR;
        warn(diagram, format!("{before}{color}{after}"));
    }
}

/// `#color:` before a keyword, the old way of colouring.
fn leading_color() -> RegexTree {
    RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+):)?")
}

/// PlantUML's `CommandSwimlane`.
pub(super) fn swimlane() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\|"),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+)\|)?"),
            RegexTree::named(1, "SWIMLANE", r"([^|]+)"),
            RegexTree::leaf(r"\|"),
            RegexTree::named(1, "LABEL", r"([^|]+)?"),
            RegexTree::end(),
        ]),
        execute_swimlane,
    )
}

/// PlantUML's `CommandSwimlane2`.
pub(super) fn swimlane2() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"swimlane"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "COLOR", r"(?:(#\w+[-\\|/]?\w+))?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "SWIMLANE", r"([^|]+?)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "LABEL", r"([^|]+)"),
            ])),
            RegexTree::end(),
        ]),
        execute_swimlane,
    )
}

fn execute_swimlane(
    diagram: &mut ActivityDiagram3,
    _: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let color = optional_color(arg, "COLOR")?;
    let name = arg.get("SWIMLANE", 0).unwrap_or_default();
    diagram.swimlane(name, color, display(arg.get("LABEL", 0)))
}

/// PlantUML's `CommandPartition3`.
pub(super) fn partition3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(partition|package|rectangle|card|group)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                color::optional_pattern("BACK1"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "NAME", r"([%g][^%g]+[%g]|.*?)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                color::optional_pattern("BACK2"),
            ])),
            stereo::optional_pattern("STEREO"),
            RegexTree::named(1, "BRACKET", r"(\{?)"),
            RegexTree::end(),
        ]),
        execute_partition3,
    )
}

fn execute_partition3(
    diagram: &mut ActivityDiagram3,
    _: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let name = without_quotes_or_brackets(arg.get("NAME", 0).unwrap_or_default());
    let colors = back_colors(
        arg,
        if arg.get("BACK1", 0).is_some() {
            "BACK1"
        } else {
            "BACK2"
        },
    )?;
    let type_ = arg.get("TYPE", 0).unwrap_or_default();
    let symbol = match type_.to_lowercase().as_str() {
        "package" => USymbols::PACKAGE,
        "rectangle" => USymbols::RECTANGLE,
        "card" => USymbols::CARD,
        "group" => USymbols::GROUP,
        _ => USymbols::PARTITION,
    };
    let stereotype = arg.get("STEREO", 0).map(Stereotype::new);
    if arg.get("BRACKET", 0).unwrap_or_default().is_empty() {
        warn(
            diagram,
            format!("You should use a bracket ({{) when defining your container '{type_}' {name}"),
        );
    }
    let style_partition = group_style_signature(symbol).get_merged_style_with(
        &diagram.titled.skin.current_style_builder(),
        stereotype.as_ref(),
    );
    let back_color = colors
        .get(ColorType::Back)
        .cloned()
        .unwrap_or_else(|| style_partition.value(PName::BackGroundColor).as_color());
    diagram.start_group(
        Display::with_newlines(name),
        Some(back_color),
        symbol,
        style_partition,
    );
    Ok(())
}

/// The style of a group drawn as `symbol` (`FtileGroup.getStyleSignature`).
fn group_style_signature(symbol: USymbol) -> StyleSignature {
    let mut names = vec![SName::Root, SName::Element, SName::ActivityDiagram];
    names.extend(symbol.get_s_names());
    names.push(SName::Composite);
    StyleSignature::of(&names)
}

/// PlantUML's `CommandCloseGroup3`.
pub(super) fn close_group3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"\}"),
            RegexTree::end(),
        ]),
        |diagram, _, _| diagram.close_group(),
    )
}

/// PlantUML's `CommandCloseGroupLegacy3`.
pub(super) fn close_group_legacy3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "CMD", r"(end ?group|group ?end)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let written = arg.get("CMD", 0).unwrap_or_default();
            warn(
                diagram,
                format!("You should use a bracket (}}) instead of '{written}'"),
            );
            diagram.close_group()
        },
    )
}

/// PlantUML's `CommandArrow3`.
pub(super) fn arrow3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::leaf(r"->"),
                RegexTree::named(1, "COLOR", style_colors_multiples()),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "LABEL", r"(.*);"),
                RegexTree::leaf(r""),
            ]),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            if let Some(color_string) = arg.get("COLOR", 0) {
                let rainbow = rainbow(diagram, color_string)?;
                diagram.set_color_next_arrow(rainbow);
            }
            if let Some(label) = non_empty(arg.get("LABEL", 0)) {
                diagram.set_label_next_arrow(display(Some(label)));
            }
            Ok(())
        },
    )
}

fn arrow_long3_start() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::or(vec![
            RegexTree::leaf(r"->"),
            RegexTree::named(1, "COLOR", style_colors_multiples()),
        ]),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "LABEL", r"(.*)"),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandArrowLong3`.
pub(super) fn arrow_long3() -> Box<dyn Command<ActivityDiagram3>> {
    let first_line = arrow_long3_start();
    Box::new(
        Multiline::starting_with_owned(
            arrow_long3_start(),
            &plantuml_regex(r"^(.*);$"),
            move |diagram: &mut ActivityDiagram3, lines: &BlocLines| {
                let lines = lines.without_empty_columns();
                let first = matched_first_line(&first_line, &lines);
                if let Some(color_string) = first.get("COLOR", 0) {
                    let rainbow = rainbow(diagram, color_string)?;
                    diagram.set_color_next_arrow(rainbow);
                }
                let lines =
                    lines.remove_starting_and_ending(first.get("LABEL", 0).unwrap_or_default(), 1);
                diagram.set_label_next_arrow(Some(lines.to_display()));
                Ok(())
            },
        )
        .skipping_quote_lines(),
    )
}

/// What the block's first line, which the command accepted, captures.
fn matched_first_line(pattern: &RegexTree, lines: &BlocLines) -> RegexResult {
    let first = lines.first().expect("a block has its first line").trimmed();
    pattern
        .matcher(first.text())
        .expect("the block's first line matched")
}

/// What the block's last line, which ended the block, captures.
fn matched_last_line(pattern: &RegexTree, lines: &BlocLines) -> RegexResult {
    let last = lines.last().expect("a block has its last line");
    pattern
        .matcher(last.text())
        .expect("the block's last line matched")
}

/// PlantUML's `CommandRepeat3`.
pub(super) fn repeat3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            leading_color(),
            RegexTree::leaf(r"repeat"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::named(1, "LABEL", r"(.*?)"),
                RegexTree::leaf(r";"),
                RegexTree::spaces_zero_or_more(),
            ])),
            Stereogroup::optional_pattern(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let label = display(arg.get("LABEL", 0));
            let stereogroup = stereogroup(arg);
            let box_style = stereogroup.get_box_style();
            warn_deprecated_color(diagram, arg.get("COLOR", 0));
            stereogroup.get_inner_colors()?;
            diagram.start_repeat(label, box_style, stereogroup);
            Ok(())
        },
    )
}

/// PlantUML's `CommandActivity3`.
pub(super) fn activity3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            stereo::optional_pattern("IGNORED"),
            RegexTree::leaf(r":"),
            RegexTree::named(1, "LABEL", r"(.*?)"),
            RegexTree::leaf(r";"),
            RegexTree::spaces_zero_or_more(),
            Stereogroup::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let stereogroup = stereogroup(arg);
            if arg.get("IGNORED", 0).is_some() {
                warn(
                    diagram,
                    "You must use stereotype at the end of the line after the ';'".to_owned(),
                );
            }
            let label = Display::with_newlines(arg.get("LABEL", 0).unwrap_or_default());
            diagram.add_activity(label, stereogroup.get_box_style(), url(arg), &stereogroup)
        },
    )
}

/// PlantUML's `CommandIf4`.
pub(super) fn if4() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            leading_color(),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::counted(1, r"(is|equals?)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "WHEN", r"(.+?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"then"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            Stereogroup::optional_pattern(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let color = optional_color(arg, "COLOR")?;
            let test = display(non_empty(arg.get("TEST", 0)));
            diagram.start_if(test, display(arg.get("WHEN", 0)), color, None, None);
            Ok(())
        },
    )
}

fn if2_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        Url::optional_pattern(),
        RegexTree::spaces_zero_or_more(),
        leading_color(),
        RegexTree::leaf(r"if"),
        stereo::optional_pattern("IGNORED"),
        RegexTree::leaf(r"\("),
        RegexTree::named(1, "TEST", r"(.*?)"),
        RegexTree::leaf(r"\)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::leaf(r"then"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "WHEN", r"\((.+?)\)")),
        ])),
        RegexTree::leaf(r";?"),
        RegexTree::spaces_zero_or_more(),
        Stereogroup::optional_pattern(),
        RegexTree::end(),
    ])
}

fn execute_if2(
    diagram: &mut ActivityDiagram3,
    _: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let test = display(non_empty(arg.get("TEST", 0)));
    let stereogroup = stereogroup(arg);
    let colors = stereogroup.get_inner_colors()?;
    diagram.start_if(
        test,
        display(arg.get("WHEN", 0)),
        colors.get(ColorType::Back).cloned(),
        url(arg),
        stereogroup.build_stereotype(),
    );
    Ok(())
}

/// PlantUML's `CommandIf2`.
pub(super) fn if2() -> Box<dyn Command<ActivityDiagram3>> {
    command(if2_pattern(), execute_if2)
}

/// PlantUML's `CommandIf2`, over several lines.
pub(super) fn if2_multine() -> Box<dyn Command<ActivityDiagram3>> {
    over_lines(if2_pattern(), execute_if2)
}

/// PlantUML's `CommandIfLegacy1`.
pub(super) fn if_legacy1() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.+?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"then"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"when"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHEN", r"(.*)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            diagram.start_if(
                display(arg.get("TEST", 0)),
                display(arg.get("WHEN", 0)),
                None,
                None,
                None,
            );
            Ok(())
        },
    )
}

/// `(-[#red]-> label)` before `else`: the arrow into the next diamond.
fn incoming() -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::leaf(r"\("),
        RegexTree::optional(RegexTree::or(vec![
            RegexTree::leaf(r"->"),
            RegexTree::named(1, "INCOMING_COLOR", style_colors_multiples()),
        ])),
        RegexTree::named(1, "INCOMING", r"(.*?)"),
        RegexTree::leaf(r"\)"),
    ]))
}

/// PlantUML's `CommandElseIf3`.
pub(super) fn else_if3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            leading_color(),
            RegexTree::spaces_zero_or_more(),
            incoming(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"if"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::counted(1, r"(is|equals?)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "WHEN", r"(.+?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"then"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::optional(RegexTree::concat(vec![
                    RegexTree::leaf(r"\("),
                    RegexTree::optional(RegexTree::or(vec![
                        RegexTree::leaf(r"->"),
                        RegexTree::named(1, "WHEN_COLOR", style_colors_multiples()),
                    ])),
                    RegexTree::leaf(r"\)"),
                ])),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let color = optional_color(arg, "COLOR")?;
            let test = display(non_empty(arg.get("TEST", 0)));
            let incoming = back_rendering(diagram, arg, "INCOMING")?;
            let when = back_rendering(diagram, arg, "WHEN")?;
            diagram.else_if(incoming, test, when, color)
        },
    )
}

fn else_if2_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        leading_color(),
        RegexTree::spaces_zero_or_more(),
        incoming(),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"else"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"if"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\("),
        RegexTree::named(1, "TEST", r"(.*?)"),
        RegexTree::leaf(r"\)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::leaf(r"then"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "WHEN_COLOR", style_colors_multiples()),
                ])),
                RegexTree::named(1, "WHEN", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
        ])),
        RegexTree::leaf(r";?"),
        RegexTree::spaces_zero_or_more(),
        Stereogroup::optional_pattern(),
        RegexTree::end(),
    ])
}

fn execute_else_if2(
    diagram: &mut ActivityDiagram3,
    _: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let colors = stereogroup(arg).get_inner_colors()?;
    let test = display(non_empty(arg.get("TEST", 0)));
    let incoming = back_rendering(diagram, arg, "INCOMING")?;
    let when = back_rendering(diagram, arg, "WHEN")?;
    diagram.else_if(incoming, test, when, colors.get(ColorType::Back).cloned())
}

/// PlantUML's `CommandElseIf2`.
pub(super) fn else_if2() -> Box<dyn Command<ActivityDiagram3>> {
    command(else_if2_pattern(), execute_else_if2)
}

/// PlantUML's `CommandElseIf2`, over several lines.
pub(super) fn else_if2_multine() -> Box<dyn Command<ActivityDiagram3>> {
    over_lines(else_if2_pattern(), execute_else_if2)
}

fn else3_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf(r"else"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::leaf(r"\("),
            RegexTree::optional(RegexTree::or(vec![
                RegexTree::leaf(r"->"),
                RegexTree::named(1, "WHEN_COLOR", style_colors_multiples()),
            ])),
            RegexTree::named(1, "WHEN", r"(.*?)"),
            RegexTree::leaf(r"\)"),
        ])),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r";?"),
        RegexTree::end(),
    ])
}

fn execute_else3(
    diagram: &mut ActivityDiagram3,
    _: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let when = back_rendering(diagram, arg, "WHEN")?;
    diagram.else2(when)
}

/// PlantUML's `CommandElse3`.
pub(super) fn else3() -> Box<dyn Command<ActivityDiagram3>> {
    command(else3_pattern(), execute_else3)
}

/// PlantUML's `CommandElse3`, over several lines.
pub(super) fn else3_multine() -> Box<dyn Command<ActivityDiagram3>> {
    over_lines(else3_pattern(), execute_else3)
}

/// PlantUML's `CommandElseLegacy1`.
pub(super) fn else_legacy1() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"else"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"when"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "WHEN", r"(.*)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            diagram.else2(LinkRendering::none().with_display(display(arg.get("WHEN", 0))))
        },
    )
}

/// `endif`, `endswitch` and the like: the keyword, then stereotypes colouring the end.
fn ending(keyword: RegexTree) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        keyword,
        RegexTree::spaces_zero_or_more(),
        Stereogroup::optional_pattern(),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandEndif3`.
pub(super) fn endif3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        ending(RegexTree::concat(vec![
            RegexTree::leaf(r"end"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"if"),
            RegexTree::leaf(r";?"),
        ])),
        |diagram, _, arg| {
            let colors = stereogroup(arg).get_inner_colors()?;
            diagram.endif(colors)
        },
    )
}

/// PlantUML's `CommandSwitch`.
pub(super) fn switch() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            leading_color(),
            RegexTree::leaf(r"switch"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            Stereogroup::optional_pattern(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let colors = stereogroup(arg).get_inner_colors()?;
            diagram.start_switch(display(non_empty(arg.get("TEST", 0))), colors);
            Ok(())
        },
    )
}

/// PlantUML's `CommandCase`.
pub(super) fn case() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"case"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| diagram.switch_case(display(non_empty(arg.get("TEST", 0)))),
    )
}

/// PlantUML's `CommandEndSwitch`.
pub(super) fn end_switch() -> Box<dyn Command<ActivityDiagram3>> {
    command(ending(RegexTree::leaf(r"endswitch")), |diagram, _, arg| {
        let colors = stereogroup(arg).get_inner_colors()?;
        diagram.end_switch(colors)
    })
}

/// PlantUML's `CommandRepeatWhile3`.
pub(super) fn repeat_while3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"repeat"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"while"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(1, "TEST3", r"\((.*?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(is|equals?)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "WHEN3", r"\((.+?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(not)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "OUT3", r"\((.+?)\)"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "TEST4", r"\((.*?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(not)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "OUT4", r"\((.+?)\)"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "TEST2", r"\((.*?)\)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::counted(1, r"(is|equals?)"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::named(1, "WHEN2", r"\((.+?)\)"),
                ]),
                RegexTree::optional(RegexTree::named(1, "TEST1", r"\((.*)\)")),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "XCOLOR", style_colors_multiples()),
                ]),
                RegexTree::spaces_zero_or_more(),
                RegexTree::or(vec![
                    RegexTree::named(1, "LABEL", r"(.*)"),
                    RegexTree::leaf(r""),
                ]),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            Stereogroup::optional_pattern(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let test = display(arg.get_lazzy("TEST", 0));
            let yes = display(arg.get_lazzy("WHEN", 0));
            let out = display(arg.get_lazzy("OUT", 0));
            let rainbow = match arg.get("XCOLOR", 0) {
                Some(color_string) => rainbow(diagram, color_string)?,
                None => Rainbow::none(),
            };
            let link_label = display(arg.get("LABEL", 0));
            diagram.repeat_while(test, yes, out, link_label, rainbow, stereogroup(arg))
        },
    )
}

fn repeat_while3_multilines_start() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf(r"repeat"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"while"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\("),
        RegexTree::named(1, "TEST1", r"(.*)"),
        RegexTree::end(),
    ])
}

fn repeat_while3_multilines_end() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::named(1, "TEST1", r"(.*)"),
        RegexTree::leaf(r"\)"),
        RegexTree::leaf(r";?"),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandRepeatWhile3Multilines`.
pub(super) fn repeat_while3_multilines() -> Box<dyn Command<ActivityDiagram3>> {
    static IS_OR_EQUALS: LazyLock<Regex> =
        LazyLock::new(|| plantuml_regex(r"\)[%s]*(is|equals?)[%s]*\("));
    let first_line = repeat_while3_multilines_start();
    let last_line = repeat_while3_multilines_end();
    Box::new(
        Multiline::between_trees(
            repeat_while3_multilines_start(),
            repeat_while3_multilines_end(),
            true,
            move |diagram: &mut ActivityDiagram3, lines: &BlocLines| {
                let lines = lines.trimmed();
                let first = matched_first_line(&first_line, &lines);
                let last = matched_last_line(&last_line, &lines);
                let mut test = Display::with_newlines(first.get("TEST1", 0).unwrap_or_default());
                for line in lines.sub_extract(1, 1).iter() {
                    test = test.add(line.text());
                }
                if let Some(trail) = last
                    .get("TEST1", 0)
                    .filter(|trail| !trail.chars().all(|c| matches!(c, ' ' | '\t' | '\0')))
                {
                    test = test.add(trail);
                }
                let mut yes = None;
                if let [split_test, split_yes] = test.split_multiline(&IS_OR_EQUALS).as_slice() {
                    yes = Some(split_yes.clone());
                    test = split_test.clone();
                }
                diagram.repeat_while(
                    Some(test),
                    yes,
                    None,
                    None,
                    Rainbow::none(),
                    Stereogroup::default(),
                )
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandBackward3`.
pub(super) fn backward3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            incoming(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"backward"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::named(1, "LABEL", r"(.*?)"),
            RegexTree::leaf(r";"),
            RegexTree::spaces_zero_or_more(),
            Stereogroup::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r"\("),
                RegexTree::optional(RegexTree::or(vec![
                    RegexTree::leaf(r"->"),
                    RegexTree::named(1, "OUTCOMING_COLOR", style_colors_multiples()),
                ])),
                RegexTree::named(1, "OUTCOMING", r"(.*?)"),
                RegexTree::leaf(r"\)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let stereogroup = stereogroup(arg);
            let label = Display::with_newlines(arg.get("LABEL", 0).unwrap_or_default());
            let incoming = back_rendering(diagram, arg, "INCOMING")?;
            let outcoming = back_rendering(diagram, arg, "OUTCOMING")?;
            diagram.backward(
                label,
                stereogroup.get_box_style(),
                incoming,
                outcoming,
                stereogroup.build_stereotype(),
            )
        },
    )
}

fn backward_long3_start() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"backward"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r":"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "DATA", r"(.*)"),
        RegexTree::end(),
    ])
}

/// The last line of a long activity: the end of its text, then stereotypes.
fn long_activity_end(with_url: bool) -> RegexTree {
    let mut parts = vec![
        RegexTree::named(1, "TEXT", r"(.*)"),
        RegexTree::leaf(r";"),
        RegexTree::spaces_zero_or_more(),
        Stereogroup::optional_pattern(),
    ];
    if with_url {
        parts.extend([RegexTree::spaces_zero_or_more(), Url::optional_pattern()]);
    }
    parts.push(RegexTree::end());
    RegexTree::concat(parts)
}

/// The text of a long activity: from what follows `start` on its first line to the end of its text
/// on the last.
fn long_activity_text(lines: &BlocLines, data: Option<&str>, last: &RegexResult) -> Display {
    lines
        .remove_starting_and_ending(data.unwrap_or_default(), 0)
        .override_last_line(last.get("TEXT", 0).unwrap_or_default())
        .to_display()
}

/// PlantUML's `CommandBackwardLong3`.
pub(super) fn backward_long3() -> Box<dyn Command<ActivityDiagram3>> {
    let first_line = backward_long3_start();
    let last_line = long_activity_end(false);
    Box::new(
        Multiline::between_trees(
            backward_long3_start(),
            long_activity_end(false),
            true,
            move |diagram: &mut ActivityDiagram3, lines: &BlocLines| {
                let lines = lines.without_empty_columns();
                let first = matched_first_line(&first_line, &lines);
                let last = matched_last_line(&last_line, &lines);
                let stereogroup = stereogroup(&last);
                let label = long_activity_text(&lines, first.get("DATA", 0), &last);
                diagram.backward(
                    label,
                    stereogroup.get_box_style(),
                    LinkRendering::none(),
                    LinkRendering::none(),
                    stereogroup.build_stereotype(),
                )
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandWhile3`.
pub(super) fn while3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            leading_color(),
            RegexTree::leaf(r"while"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::named(1, "TEST", r"(.*?)"),
            RegexTree::leaf(r"\)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::counted(1, r"(is|equals?)"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "YES", r"\((.+?)\)"),
            ])),
            RegexTree::leaf(r";?"),
            RegexTree::spaces_zero_or_more(),
            Stereogroup::optional_pattern(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let colors = stereogroup(arg).get_inner_colors()?;
            diagram.do_while(
                Display::with_newlines(arg.get("TEST", 0).unwrap_or_default()),
                display(arg.get("YES", 0)),
                colors.get(ColorType::Back).cloned(),
            );
            Ok(())
        },
    )
}

/// PlantUML's `CommandWhileEnd3`.
pub(super) fn while_end3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::leaf(r"end"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"while"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::leaf(r"while"),
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(r"end"),
                ]),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "OUT", r"\((.+?)\)")),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| diagram.endwhile(display(arg.get("OUT", 0))),
    )
}

/// PlantUML's `CommandFork3`.
pub(super) fn fork3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        ending(RegexTree::concat(vec![
            RegexTree::leaf(r"fork"),
            RegexTree::leaf(r";?"),
        ])),
        |diagram, _, arg| {
            let colors = stereogroup(arg).get_inner_colors()?;
            diagram.fork(colors);
            Ok(())
        },
    )
}

/// `keyword1 keyword2`, optionally ended by `;`.
fn two_words(first: &'static str, second: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::leaf(first),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(second),
    ])
}

/// A whole line of `words`, optionally ended by `;`.
fn keywords(words: RegexTree) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        words,
        RegexTree::leaf(r";?"),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandForkAgain3`.
pub(super) fn fork_again3() -> Box<dyn Command<ActivityDiagram3>> {
    command(keywords(two_words("fork", "again")), |diagram, _, _| {
        diagram.fork_again()
    })
}

/// PlantUML's `CommandForkEnd3`.
pub(super) fn fork_end3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named_or(
                "STYLE",
                vec![
                    two_words("end", "fork"),
                    two_words("fork", "end"),
                    two_words("end", "merge"),
                ],
            ),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LABEL", r"(\{.+\})?"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let fork_style = if arg.get("STYLE", 0).unwrap_or_default().contains("merge") {
                ForkStyle::Merge
            } else {
                ForkStyle::Fork
            };
            diagram.end_fork(fork_style, arg.get("LABEL", 0).map(str::to_owned))
        },
    )
}

/// PlantUML's `CommandSplit3`.
pub(super) fn split3() -> Box<dyn Command<ActivityDiagram3>> {
    command(keywords(RegexTree::leaf(r"split")), |diagram, _, _| {
        diagram.split();
        Ok(())
    })
}

/// PlantUML's `CommandSplitAgain3`.
pub(super) fn split_again3() -> Box<dyn Command<ActivityDiagram3>> {
    command(keywords(two_words("split", "again")), |diagram, _, _| {
        diagram.split_again()
    })
}

/// PlantUML's `CommandSplitEnd3`.
pub(super) fn split_end3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        keywords(RegexTree::or(vec![
            two_words("end", "split"),
            two_words("split", "end"),
        ])),
        |diagram, _, _| diagram.end_split(),
    )
}

/// `start`, `stop` or `end`, then stereotypes colouring the circle.
fn circle(keyword: &'static str) -> RegexTree {
    ending(RegexTree::concat(vec![
        RegexTree::leaf(keyword),
        RegexTree::leaf(r";?"),
    ]))
}

/// PlantUML's `CommandStart3`.
pub(super) fn start3() -> Box<dyn Command<ActivityDiagram3>> {
    command(circle(r"start"), |diagram, _, arg| {
        diagram.start(stereogroup(arg).get_inner_colors()?);
        Ok(())
    })
}

/// PlantUML's `CommandStop3`.
pub(super) fn stop3() -> Box<dyn Command<ActivityDiagram3>> {
    command(circle(r"stop"), |diagram, _, arg| {
        diagram.stop(stereogroup(arg).get_inner_colors()?);
        Ok(())
    })
}

/// PlantUML's `CommandCircleSpot3`.
pub(super) fn circle_spot3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            leading_color(),
            RegexTree::named(1, "SPOT", r"\((\S)\)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let color = optional_color(arg, "COLOR")?;
            diagram.add_spot(arg.get("SPOT", 0).unwrap_or_default(), color);
            Ok(())
        },
    )
}

/// PlantUML's `CommandBreak`.
pub(super) fn break_command() -> Box<dyn Command<ActivityDiagram3>> {
    command(keywords(RegexTree::leaf(r"break")), |diagram, _, _| {
        diagram.break_instruction();
        Ok(())
    })
}

/// PlantUML's `CommandEnd3`.
pub(super) fn end3() -> Box<dyn Command<ActivityDiagram3>> {
    command(circle(r"end"), |diagram, _, arg| {
        diagram.end(stereogroup(arg).get_inner_colors()?);
        Ok(())
    })
}

/// PlantUML's `CommandKill3`.
pub(super) fn kill3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        keywords(RegexTree::leaf(r"kill|detach")),
        |diagram, _, _| diagram.kill(),
    )
}

/// PlantUML's `CommandLink3`.
pub(super) fn link3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"link"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "COLOR", r"(#\w+)"),
            RegexTree::leaf(r";?"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            if let Some(color) = optional_color(arg, "COLOR")? {
                diagram.set_color_next_arrow(Rainbow::from_color(Some(color), None));
            }
            Ok(())
        },
    )
}

/// `note left` or `floating note right`, coloured and stereotyped, as a note's first line has it.
fn note_position_and_type(arg: &RegexResult) -> (NotePosition, NoteType) {
    let position = arg
        .get("POSITION", 0)
        .and_then(NotePosition::named)
        .unwrap_or(NotePosition::Left);
    let type_ = arg
        .get("TYPE", 0)
        .map_or(NoteType::Note, NoteType::default_type);
    (position, type_)
}

/// PlantUML's `CommandNote3`.
pub(super) fn note3() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(note|floating note)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "POSITION", r"(left|right)?"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS"),
            stereo::optional_pattern("STEREO"),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NOTE", r"(.*)"),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let colors = back_colors(arg, "COLOR")?;
            let note = Display::with_newlines(arg.get("NOTE", 0).unwrap_or_default());
            let (position, type_) = note_position_and_type(arg);
            let stereotype = arg.get("STEREO", 0).map(Stereotype::new);
            diagram.add_note(note, position, type_, colors, stereotype);
            Ok(())
        },
    )
}

fn note_long3_start() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::named(1, "TYPE", r"(note|floating note)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "POSITION", r"(left|right)?"),
        RegexTree::spaces_zero_or_more(),
        stereo::tags_pattern("TAGS"),
        stereo::optional_pattern("STEREO"),
        color::optional_pattern("COLOR"),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandNoteLong3`.
pub(super) fn note_long3() -> Box<dyn Command<ActivityDiagram3>> {
    let first_line = note_long3_start();
    Box::new(
        Multiline::starting_with_owned(
            note_long3_start(),
            &plantuml_regex(r"^end[%s]?note$"),
            move |diagram: &mut ActivityDiagram3, lines: &BlocLines| {
                let first = matched_first_line(&first_line, lines);
                let note = lines.sub_extract(1, 1).without_empty_columns().to_display();
                let (position, type_) = note_position_and_type(&first);
                let colors = back_colors(&first, "COLOR")?;
                let stereotype = first.get("STEREO", 0).map(Stereotype::new);
                diagram.add_note(note, position, type_, colors, stereotype);
                Ok(())
            },
        )
        .skipping_quote_lines(),
    )
}

fn activity_long3_start() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        color::optional_pattern("COLOR"),
        RegexTree::leaf(r":"),
        RegexTree::named(1, "DATA", r"(.*)"),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandActivityLong3`.
pub(super) fn activity_long3() -> Box<dyn Command<ActivityDiagram3>> {
    let first_line = activity_long3_start();
    let last_line = long_activity_end(true);
    Box::new(
        Multiline::between_trees(
            activity_long3_start(),
            long_activity_end(true),
            false,
            move |diagram: &mut ActivityDiagram3, lines: &BlocLines| {
                let lines = lines.without_empty_columns();
                let first = matched_first_line(&first_line, &lines);
                let last = matched_last_line(&last_line, &lines);
                warn_deprecated_color(diagram, first.get("COLOR", 0));
                let stereogroup = stereogroup(&last);
                let label = long_activity_text(&lines, first.get("DATA", 0), &last);
                diagram.add_activity(label, stereogroup.get_box_style(), url(&last), &stereogroup)
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandActivityList`.
pub(super) fn activity_list() -> Box<dyn Command<ActivityDiagram3>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"[-*]"),
            RegexTree::optional(RegexTree::leaf(r"[%s]")),
            RegexTree::named(1, "LABEL", r"(.*?)"),
            RegexTree::spaces_zero_or_more(),
            Stereogroup::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::end(),
        ]),
        |diagram, _, arg| {
            let label = Display::with_newlines(arg.get("LABEL", 0).unwrap_or_default());
            diagram.add_activity(label, BoxStyle::Plain, url(arg), &stereogroup(arg))
        },
    )
}

/// `keyword name`: a label or a jump to one.
fn named_target(keyword: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf(keyword),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "NAME", r"([%pLN_.]+)"),
        RegexTree::leaf(r";?"),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandLabel`.
pub(super) fn label() -> Box<dyn Command<ActivityDiagram3>> {
    command(named_target(r"label"), |diagram, _, arg| {
        diagram.add_label(arg.get("NAME", 0).unwrap_or_default());
        Ok(())
    })
}

/// PlantUML's `CommandGoto`.
pub(super) fn goto() -> Box<dyn Command<ActivityDiagram3>> {
    command(named_target(r"goto"), |diagram, _, arg| {
        diagram.add_goto(arg.get("NAME", 0).unwrap_or_default());
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_activity_ends_at_its_last_semicolon() {
        let end = long_activity_end(true);
        let last = end.matcher("  is it; done; <<input>>").unwrap();
        assert_eq!(last.get("TEXT", 0), Some("  is it; done"));
        assert_eq!(last.get("STEREOGROUP", 0), Some("<<input>>"));
    }
}

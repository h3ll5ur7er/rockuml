//! The lines of a wire diagram. None is trimmed: indentation tells the level of the block a line is for.

use super::link::{WArrowDirection, WLinkType, WOrientation};
use super::{LinkSpec, WireDiagram};
use crate::color::HColor;
use crate::command::{Command, CommandError, CommandResult, PatternCommand, SingleLine};
use crate::creole::Display;
use crate::pattern::{RegexResult, RegexTree};
use crate::text::LineLocation;

use super::super::common_commands::add_common_commands1;

pub(super) fn init_commands_list() -> Vec<Box<dyn Command<WireDiagram>>> {
    let mut commands = add_common_commands1();
    commands.extend([
        command_component(),
        command_spot(),
        command_goto(),
        command_move(),
        command_wlink(),
        command_new_column(),
        command_print(),
    ]);
    commands
}

fn command(
    parts: Vec<RegexTree>,
    execute: fn(&mut WireDiagram, &RegexResult) -> CommandResult,
) -> Box<dyn Command<WireDiagram>> {
    let mut pattern = vec![RegexTree::start()];
    pattern.extend(parts);
    pattern.extend([RegexTree::spaces_zero_or_more(), RegexTree::end()]);
    Box::new(SingleLine(
        PatternCommand::new(
            RegexTree::concat(pattern),
            move |diagram: &mut WireDiagram, _: &LineLocation, arg: &RegexResult| {
                execute(diagram, arg)
            },
        )
        .untrimmed(),
    ))
}

fn indent() -> RegexTree {
    RegexTree::named(1, "INDENT", r"([\s\t]*)")
}

/// `(x, y)` in pixels, percents or both, named `X…` and `Y…`.
fn point(x: &'static str, y: &'static str) -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::leaf(r"\("),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(2, x, r"(-?\d+(%|%[-+]\d+)?)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(","),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(2, y, r"(-?\d+(%|%[-+]\d+)?)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\)"),
    ]))
}

/// `(x, y)` in pixels, negative ones only when `signed`.
fn coordinates(signed: bool) -> Vec<RegexTree> {
    let number = if signed { r"(-?\d+)" } else { r"(\d+)" };
    vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\("),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "X", number),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(","),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "Y", number),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\)"),
    ]
}

fn color() -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "COLOR", r"(#\w+)?"),
    ]))
}

fn get_color(arg: &RegexResult) -> Result<Option<HColor>, CommandError> {
    arg.get("COLOR", 0)
        .map(|color| {
            HColor::parse(color)
                .ok()
                .flatten()
                .ok_or_else(CommandError::bad_color)
        })
        .transpose()
}

fn get_point<'a>(arg: &'a RegexResult, x: &str, y: &str) -> Option<(&'a str, &'a str)> {
    arg.get(x, 0).zip(arg.get(y, 0))
}

fn get_number(arg: &RegexResult, name: &str) -> f64 {
    let number = arg
        .get(name, 0)
        .expect("the pattern reads both coordinates");
    number.parse().expect("the pattern reads numbers")
}

/// `Integer.parseInt`, which throws past `i32`.
fn parse_int(text: &str) -> Result<i32, CommandError> {
    text.parse().map_err(|_| {
        CommandError::thrown(&format!(
            "java.lang.NumberFormatException: For input string: \"{text}\""
        ))
    })
}

/// PlantUML's `CommandComponent`: `* name [width x height] #color`, indented under its parent.
fn command_component() -> Box<dyn Command<WireDiagram>> {
    command(
        vec![
            indent(),
            RegexTree::leaf(r"\*"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME", r"([\w]+)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"\["),
                RegexTree::named(1, "WIDTH", r"([\d]+)"),
                RegexTree::leaf("[x*]"),
                RegexTree::named(1, "HEIGHT", r"([\d]+)"),
                RegexTree::leaf(r"\]"),
            ])),
            color(),
        ],
        |diagram, arg| {
            let (width, height) = match arg.get("WIDTH", 0).zip(arg.get("HEIGHT", 0)) {
                Some((width, height)) => (parse_int(width)?, parse_int(height)?),
                None => (0, 0),
            };
            diagram.add_component(
                arg.get("INDENT", 0).unwrap_or_default(),
                arg.get("NAME", 0).unwrap_or_default(),
                f64::from(width),
                f64::from(height),
                get_color(arg)?,
            )
        },
    )
}

/// PlantUML's `CommandSpot`: `spot block.child(x, y) #color`.
fn command_spot() -> Box<dyn Command<WireDiagram>> {
    command(
        vec![
            RegexTree::leaf("spot"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NAME", r"([\w][.\w]*)"),
            point("X", "Y"),
            color(),
        ],
        |diagram, arg| {
            let color = get_color(arg)?;
            diagram.spot(
                arg.get("NAME", 0).unwrap_or_default(),
                color,
                get_point(arg, "X", "Y"),
            )
        },
    )
}

/// PlantUML's `CommandGoto`: `goto(x, y)` sends the cursor of its level there.
fn command_goto() -> Box<dyn Command<WireDiagram>> {
    let mut parts = vec![indent(), RegexTree::leaf("goto")];
    parts.extend(coordinates(false));
    command(parts, |diagram, arg| {
        diagram.wgoto(
            arg.get("INDENT", 0).unwrap_or_default(),
            get_number(arg, "X"),
            get_number(arg, "Y"),
        )
    })
}

/// PlantUML's `CommandMove`: `move(dx, dy)` moves the cursor of its level by that much.
fn command_move() -> Box<dyn Command<WireDiagram>> {
    let mut parts = vec![indent(), RegexTree::leaf("move")];
    parts.extend(coordinates(true));
    command(parts, |diagram, arg| {
        diagram.wmove(
            arg.get("INDENT", 0).unwrap_or_default(),
            get_number(arg, "X"),
            get_number(arg, "Y"),
        )
    })
}

/// PlantUML's `CommandWLink`: `from(x, y) -> to #color : label`, across with one `-` or `=`, down with two.
fn command_wlink() -> Box<dyn Command<WireDiagram>> {
    command(
        vec![
            RegexTree::named(1, "NAME1", r"([\w][.\w]*)"),
            point("X1", "Y1"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "STYLE", r"(\<?[-=]{1,2}\>|\<?[-=]{1,2})"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME2", r"([\w][.\w]*)"),
            color(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "MESSAGE", r"(?::[%s]*(.*))?"),
        ],
        |diagram, arg| {
            let style = arg.get("STYLE", 0).unwrap_or_default();
            let spec = LinkSpec {
                from: arg.get("NAME1", 0).unwrap_or_default(),
                at: get_point(arg, "X1", "Y1"),
                to: arg.get("NAME2", 0).unwrap_or_default(),
                link_type: WLinkType::from(style),
                direction: WArrowDirection::from(style),
                color: get_color(arg)?.unwrap_or(HColor::BLACK),
            };
            match WOrientation::from(style) {
                WOrientation::Vertical => diagram.vlink(spec),
                WOrientation::Horizontal => {
                    let label = arg
                        .get("MESSAGE", 0)
                        .map_or_else(Display::default, Display::with_newlines);
                    diagram.hlink(spec, label)
                }
            }
        },
    )
}

/// PlantUML's `CommandNewColumn`: a line of `-` starts a column right of what its level holds.
fn command_new_column() -> Box<dyn Command<WireDiagram>> {
    command(vec![indent(), RegexTree::leaf("-+")], |diagram, arg| {
        diagram.new_column(arg.get("INDENT", 0).unwrap_or_default())
    })
}

/// PlantUML's `CommandPrint`: `print("text")` at the cursor of its level.
fn command_print() -> Box<dyn Command<WireDiagram>> {
    command(
        vec![
            indent(),
            RegexTree::leaf("print"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("[%g]"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "TEXT", "(.*)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("[%g]"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\)"),
        ],
        |diagram, arg| {
            diagram.print(
                arg.get("INDENT", 0).unwrap_or_default(),
                arg.get("TEXT", 0).unwrap_or_default(),
            )
        },
    )
}

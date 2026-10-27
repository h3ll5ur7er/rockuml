//! The commands of packet diagrams (PlantUML's `packetdiag.command` package).

use super::{PacketDiagram, ScaleDirection};
use crate::command::{Command, CommandError, CommandResult, PatternCommand, SingleLine};
use crate::diagram::nwdiag::Properties;
use crate::java;
use crate::pattern::{RegexResult, RegexTree};
use crate::text::LineLocation;

/// The commands in PlantUML's order, after the common ones.
pub(super) fn all() -> Vec<Box<dyn Command<PacketDiagram>>> {
    vec![
        packet_diag_start(),
        col_width(),
        node_height(),
        scale_direction(),
        scale_interval(),
        same_height(),
        num_range(),
        packet_diag_end(),
    ]
}

/// A one-line command made of `parts` between the line's start and end.
fn command(
    parts: Vec<RegexTree>,
    apply: fn(&mut PacketDiagram, &RegexResult) -> CommandResult,
) -> Box<dyn Command<PacketDiagram>> {
    let mut pattern = vec![RegexTree::start()];
    pattern.extend(parts);
    pattern.push(RegexTree::end());
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(pattern),
        move |diagram: &mut PacketDiagram, _: &LineLocation, arg: &RegexResult| apply(diagram, arg),
    )))
}

/// A `keyword = value` setting, its value matched by `value` in group `name`.
fn setting(
    keyword: &'static str,
    name: &'static str,
    value: &'static str,
    apply: fn(&mut PacketDiagram, &RegexResult) -> CommandResult,
) -> Box<dyn Command<PacketDiagram>> {
    command(
        vec![
            RegexTree::leaf(keyword),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("="),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, name, value),
            RegexTree::spaces_zero_or_more(),
        ],
        apply,
    )
}

/// A number of at most seven digits, which always fits an `int`.
fn digits(text: Option<&str>) -> i32 {
    text.unwrap_or_default()
        .parse()
        .expect("the pattern allows only a few digits")
}

/// PlantUML's `CommandPacketDiagStart`: `packetdiag {`, which opens nothing.
fn packet_diag_start() -> Box<dyn Command<PacketDiagram>> {
    command(
        vec![
            RegexTree::named(1, "TYPE", "(packetdiag)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{?"),
        ],
        |_, _| Ok(()),
    )
}

/// PlantUML's `CommandColWidth`: `colwidth = 32`, the bits in a row.
fn col_width() -> Box<dyn Command<PacketDiagram>> {
    setting("colwidth", "WIDTH", r"(\d{1,3});?", |diagram, arg| {
        diagram.set_col_width(digits(arg.get("WIDTH", 0)));
        Ok(())
    })
}

/// PlantUML's `CommandNodeHeight`: `node_height = 72`, the height of a field one unit high.
fn node_height() -> Box<dyn Command<PacketDiagram>> {
    setting("node_height", "HEIGHT", r"(\d{1,3});?", |diagram, arg| {
        diagram.update_node_height(digits(arg.get("HEIGHT", 0)));
        Ok(())
    })
}

/// PlantUML's `CommandScaleDirection`: `scale_direction = rtl` numbers the bits from the right.
fn scale_direction() -> Box<dyn Command<PacketDiagram>> {
    setting("scale_direction", "DIR", "(ltr|rtl);?", |diagram, arg| {
        diagram.scale_direction = if arg
            .get("DIR", 0)
            .is_some_and(|dir| dir.eq_ignore_ascii_case("rtl"))
        {
            ScaleDirection::Rtl
        } else {
            ScaleDirection::Ltr
        };
        Ok(())
    })
}

/// PlantUML's `CommandScaleInterval`: `scale_interval = 4`, how often the scale is numbered.
fn scale_interval() -> Box<dyn Command<PacketDiagram>> {
    setting("scale_interval", "INTERVAL", r"(\d+);?", |diagram, arg| {
        let interval = arg
            .get("INTERVAL", 0)
            .unwrap_or_default()
            .parse()
            .map_err(|_| CommandError::new("Scale interval invalid"))?;
        diagram.update_scale_interval(interval);
        Ok(())
    })
}

/// PlantUML's `CommandSameHeight`: `same_height = true` gives the fields of a row its tallest's height.
fn same_height() -> Box<dyn Command<PacketDiagram>> {
    setting("same_height", "VALUE", "(true|false);?", |diagram, arg| {
        diagram.same_height = arg
            .get("VALUE", 0)
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        Ok(())
    })
}

/// PlantUML's `CommandNumRange`: a field, `0-15: Source Port`, `16: Flag [len = 4]` or `* Next [height = 2]`.
/// PlantUML reads a `rotate` attribute too, but draws no label rotated.
fn num_range() -> Box<dyn Command<PacketDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![
                RegexTree::named(1, "RANGE_UNIT", r"(\d{1,7})(?::)?"),
                RegexTree::named(2, "RANGE", r"(\d{1,7})(?:-(\d{1,7}))(?::)?"),
                RegexTree::leaf(r"\*"),
            ]),
            RegexTree::named(1, "DESC", r"\s+(.*?)"),
            RegexTree::named(1, "ATTRIBUTE", r"(?:\s*\[(.*?)\])?"),
            RegexTree::spaces_zero_or_more(),
        ],
        |diagram, arg| {
            let attr = Properties::parse(arg.get("ATTRIBUTE", 0));
            let int_attribute = |name, default: i32| {
                attr.get(name)
                    .map_or(default, |value| java::parse_int(value).unwrap_or(default))
            };
            let after_length =
                |start: i32| start.wrapping_add(int_attribute("len", 1)).wrapping_sub(1);
            let (start, end) = match (arg.get("RANGE_UNIT", 0), arg.get("RANGE", 0)) {
                (Some(unit), _) => {
                    let start = digits(Some(unit));
                    (start, after_length(start))
                }
                (None, Some(start)) => (digits(Some(start)), digits(arg.get("RANGE", 1))),
                (None, None) => {
                    let start = diagram
                        .last_packet_end()
                        .map_or(0, |end| end.wrapping_add(1));
                    (start, after_length(start))
                }
            };
            diagram.add_packet_item_range(
                start,
                end,
                int_attribute("height", 1),
                arg.get("DESC", 0).unwrap_or_default(),
            );
            Ok(())
        },
    )
}

/// PlantUML's `CommandPacketDiagEnd`: `}`, which closes nothing.
fn packet_diag_end() -> Box<dyn Command<PacketDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\}?"),
            RegexTree::spaces_zero_or_more(),
        ],
        |_, _| Ok(()),
    )
}

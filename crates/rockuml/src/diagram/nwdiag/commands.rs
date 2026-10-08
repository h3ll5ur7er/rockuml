//! The commands of network diagrams (PlantUML's `nwdiag.Command*`).

use super::NwDiagram;
use crate::command::{Command, CommandResult, PatternCommand, SingleLine};
use crate::pattern::{RegexResult, RegexTree};
use crate::text::LineLocation;

/// The commands in PlantUML's order, after the common ones.
pub(super) fn all() -> Vec<Box<dyn Command<NwDiagram>>> {
    vec![
        nw_diag_init(),
        comment(),
        element(),
        group(),
        network(),
        link(),
        property(),
        end_something(),
        crate::diagram::cuca_commands::footbox_ignored(),
    ]
}

/// A one-line command made of `parts` between the line's start and end.
fn command(
    parts: Vec<RegexTree>,
    apply: fn(&mut NwDiagram, &RegexResult) -> CommandResult,
) -> Box<dyn Command<NwDiagram>> {
    let mut pattern = vec![RegexTree::start()];
    pattern.extend(parts);
    pattern.push(RegexTree::end());
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(pattern),
        move |diagram: &mut NwDiagram, _: &LineLocation, arg: &RegexResult| apply(diagram, arg),
    )))
}

/// PlantUML's `CommandNwDiagInit`: `nwdiag {`, which opens nothing.
fn nw_diag_init() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::named(1, "TYPE", "(nwdiag)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{?"),
        ],
        |_, _| Ok(()),
    )
}

/// PlantUML's `CommandComment`: `// ...`.
fn comment() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("//"),
            RegexTree::leaf(".*"),
        ],
        |_, _| Ok(()),
    )
}

/// PlantUML's `CommandElement`: `web01 [address = "x", shape = database];`.
fn element() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME", r"([-.%pLN_]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(2, "DEFINITION", r"(\[(.*)\])?"),
            RegexTree::leaf(";?"),
        ],
        |diagram, arg| {
            diagram.add_element(
                arg.get("NAME", 0).unwrap_or_default(),
                arg.get("DEFINITION", 1),
            )
        },
    )
}

/// PlantUML's `CommandGroup`: `group name {`.
fn group() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("group"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME", r"([%pLN_]+)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
        ],
        |diagram, _| diagram.open_group(),
    )
}

/// PlantUML's `CommandNetwork`: `network name {`.
fn network() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("network"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME", r"([-.%pLN_]+)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
        ],
        |diagram, arg| diagram.open_network(arg.get("NAME", 0)),
    )
}

/// PlantUML's `CommandLink`: `inet -- router;`.
fn link() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME1", r"([%pLN_]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("--"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME2", r"([%pLN_]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(";?"),
        ],
        |diagram, arg| {
            diagram.link(
                arg.get("NAME1", 0).unwrap_or_default(),
                arg.get("NAME2", 0).unwrap_or_default(),
            )
        },
    )
}

/// PlantUML's `CommandProperty`: `address = "..."` and the like in a network or group.
fn property() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NAME", "(address|color|width|description)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("="),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("\"?"),
            RegexTree::named(1, "VALUE", "([^\"]*)"),
            RegexTree::leaf("\"?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(";?"),
        ],
        |diagram, arg| {
            diagram.set_property(
                arg.get("NAME", 0).unwrap_or_default(),
                arg.get("VALUE", 0).unwrap_or_default(),
            );
            Ok(())
        },
    )
}

/// PlantUML's `CommandEndSomething`: `}`.
fn end_something() -> Box<dyn Command<NwDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\}"),
            RegexTree::spaces_zero_or_more(),
        ],
        |diagram, _| {
            diagram.close_something();
            Ok(())
        },
    )
}

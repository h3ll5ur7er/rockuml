//! The commands of sequence diagrams, in PlantUML's order (`SequenceDiagramFactory.initCommandsList`).

mod arrow;
mod exo_arrow;
mod misc;
mod note;
mod participant;

use super::SequenceDiagram;
use super::model::{LifeEventType, LiveColors, ParticipantId};
use crate::color::{ColorType, Colors, HColor};
use crate::command::{Command, CommandError, CommandResult};
use crate::diagram::common_commands::common_commands;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree};
use crate::text::unquoted;

pub(super) fn commands() -> Vec<Box<dyn Command<SequenceDiagram>>> {
    let mut commands = common_commands();
    commands.extend([
        misc::hide_unlinked(),
        misc::activate(),
        misc::deactivate_short(),
    ]);
    commands.extend(participant::commands());
    commands.extend([arrow::command(), exo_arrow::left(), exo_arrow::right()]);
    commands.extend([
        note::single_line(),
        note::over_several_single_line(),
        note::across_single_line(),
        misc::box_start(),
        misc::box_end(),
        misc::grouping(),
        misc::activate_shortcut(),
        misc::return_command(),
        note::on_arrow_single_line(),
        note::multi_line(),
        note::over_several_multi_line(),
        note::on_arrow_multi_line(),
        note::across_multi_line(),
        misc::newpage(),
        misc::ignore_newpage(),
        misc::auto_newpage(),
        misc::divider(),
        misc::hspace(),
        misc::reference_over_several(),
        misc::reference_multiline_over_several(),
        misc::autonumber(),
        misc::autonumber_stop(),
        misc::autonumber_resume(),
        misc::autonumber_increment(),
        misc::autoactivate(),
        misc::footbox(),
        misc::delay(),
        misc::footbox_old(),
        misc::url(),
        misc::link_anchor(),
    ]);
    commands
}

/// The pattern of a participant name: a code, or anything quoted.
const PARTICIPANT_CODE_OR_QUOTED: &str = r"([%pLN_.@]+|[%g][^%g]+[%g])";

/// `UrlBuilder.OPTIONAL`.
fn optional_url() -> RegexTree {
    RegexTree::optional(RegexTree::named(12, "URL", Url::command_pattern()))
}

/// `ColorParser.simpleColor`: an optional colour specification named `COLOR`.
fn optional_colors() -> RegexTree {
    RegexTree::named(1, "COLOR", format!("({})?", crate::color::COLORS_REGEXP))
}

/// `HColorSet.getColor`: an unknown colour fails the command.
fn color_named(text: &str) -> Result<HColor, CommandError> {
    HColor::parse(text).ok().flatten().ok_or_else(no_such_color)
}

fn no_such_color() -> CommandError {
    CommandError::new("No such color")
}

/// The colours a `COLOR` specification gives, the main one painting the background.
fn colors(arg: &RegexResult) -> Result<Colors, CommandError> {
    arg.get("COLOR", 0)
        .map(|data| Colors::parse(data, ColorType::Back).map_err(|_| no_such_color()))
        .transpose()
        .map(Option::unwrap_or_default)
}

/// The diagram's `activate`, turning its refusal into a command error.
fn activate(
    diagram: &mut SequenceDiagram,
    participant: ParticipantId,
    kind: LifeEventType,
    back: Option<HColor>,
) -> CommandResult {
    diagram
        .activate(participant, kind, LiveColors { back, line: None })
        .map_err(CommandError::new)
}

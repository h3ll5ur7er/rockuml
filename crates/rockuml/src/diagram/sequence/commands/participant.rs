//! Participant declarations, like `actor Bob #red` or `participant "Long name" as L` (PlantUML's
//! `CommandParticipant` and its variants).

use std::sync::LazyLock;

use super::{activate, color_named, optional_colors, optional_url};
use crate::command::{BlocLines, Command, CommandResult, Multiline, SingleLine, SingleLineCommand};
use crate::creole::Display;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{LifeEventType, ParticipantType};
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stereo::{self, Stereotype};
use crate::text::LineLocation;

/// PlantUML's `CommandParticipantA`, `CommandParticipantA2`, `CommandParticipantA3`, `CommandParticipantA4` and
/// `CommandParticipantMultilines`, in that order.
pub(super) fn commands() -> Vec<Box<dyn Command<SequenceDiagram>>> {
    let full_as_code = vec![
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("as"),
            RegexTree::spaces_one_or_more(),
        ])),
        RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
    ];
    let code_as_full = vec![
        RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf("as"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
    ];
    let unquoted_full_as_code = vec![
        RegexTree::named(1, "FULL", r"([%pLN_.@]+)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf("as"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
    ];
    let quoted_code = vec![RegexTree::named(1, "CODE", r"[%g]([^%g]+)[%g]")];
    let mut commands: Vec<Box<dyn Command<SequenceDiagram>>> = [
        full_as_code,
        code_as_full,
        unquoted_full_as_code,
        quoted_code,
    ]
    .into_iter()
    .map(|names| -> Box<dyn Command<SequenceDiagram>> {
        Box::new(SingleLine(CommandParticipant(declaration(names))))
    })
    .collect();
    commands.push(Box::new(
        Multiline::starting_with(&FIRST_LINE, &plantuml_regex(r"^([^\[\]]*)\]$"), multiline)
            .skipping_quote_lines(),
    ));
    commands
}

fn participant_type() -> RegexTree {
    RegexTree::or(vec![
        RegexTree::named(
            1,
            "TYPE",
            "(participant|actor|create|boundary|control|entity|queue|database|collections)",
        ),
        RegexTree::named(
            1,
            "CREATE",
            "create[%s](participant|actor|boundary|control|entity|queue|database|collections)",
        ),
    ])
}

fn order() -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::leaf("order"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "ORDER", r"(-?\d{1,7})"),
    ]))
}

/// What every declaration ends with: stereotype, order, link and colour.
fn declaration_tail() -> Vec<RegexTree> {
    vec![
        stereo::optional_pattern("STEREO"),
        order(),
        RegexTree::spaces_zero_or_more(),
        optional_url(),
        RegexTree::spaces_zero_or_more(),
        optional_colors(),
    ]
}

fn declaration(names: Vec<RegexTree>) -> RegexTree {
    let mut parts = vec![
        RegexTree::start(),
        participant_type(),
        RegexTree::spaces_one_or_more(),
    ];
    parts.extend(names);
    parts.extend(declaration_tail());
    parts.push(RegexTree::end());
    RegexTree::concat(parts)
}

struct CommandParticipant(RegexTree);

impl SingleLineCommand<SequenceDiagram> for CommandParticipant {
    fn pattern(&self) -> &RegexTree {
        &self.0
    }

    fn execute_arg(
        &self,
        diagram: &mut SequenceDiagram,
        _location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        let code = arg.get("CODE", 0).unwrap_or_default().to_owned();
        if diagram.contains_participant(&code) {
            diagram.put_participant_in_last(&code);
            return Ok(());
        }
        let display = arg.get("FULL", 0).map(Display::with_newlines);
        let (kind, create) = match (arg.get("CREATE", 0), arg.get("TYPE", 0)) {
            (Some(created), _) => (ParticipantType::named(created), true),
            (None, Some(kind)) if kind.eq_ignore_ascii_case("create") => {
                (Some(ParticipantType::Participant), true)
            }
            (None, kind) => (kind.and_then(ParticipantType::named), false),
        };
        let kind = kind.expect("the pattern only matches participant types");
        let order = participant_order(arg);
        let participant = diagram.create_new_participant(kind, &code, display, order);
        decorate(diagram, participant, arg)?;
        let color = arg.get("COLOR", 0).map(color_named).transpose()?;
        if let Some(color) = color {
            let colors = &mut diagram.participant_mut(participant).colors;
            *colors = colors.with(crate::color::ColorType::Back, Some(color));
        }
        if create {
            activate(diagram, participant, LifeEventType::Create, None)?;
        }
        Ok(())
    }
}

/// Where `order` places the participant; an unreadable number counts as 0.
fn participant_order(arg: &RegexResult) -> i32 {
    arg.get("ORDER", 0)
        .map_or(0, |order| order.parse().unwrap_or(0))
}

/// The stereotype and link a declaration gives its participant.
fn decorate(
    diagram: &mut SequenceDiagram,
    participant: crate::diagram::sequence::model::ParticipantId,
    arg: &RegexResult,
) -> CommandResult {
    let position_top = diagram
        .skin()
        .value("stereotypeposition")
        .is_none_or(|position| !position.eq_ignore_ascii_case("bottom"));
    if let Some(stereotype) = arg.get("STEREO", 0) {
        let stereotype = Stereotype::with_spot(stereotype).map_err(|_| super::no_such_color())?;
        diagram.participant_mut(participant).stereotype = Some((stereotype, position_top));
    }
    if let Some(url) = arg.get("URL", 0) {
        diagram.participant_mut(participant).url = Url::parse(url);
    }
    Ok(())
}

fn multiline_first_line() -> RegexTree {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::named(1, "TYPE", "(participant)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
    ];
    parts.extend(declaration_tail());
    parts.extend([
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\["),
        RegexTree::end(),
    ]);
    RegexTree::concat(parts)
}

static FIRST_LINE: LazyLock<RegexTree> = LazyLock::new(multiline_first_line);

/// `participant P [` ... `]`: a participant whose name spans lines.
fn multiline(diagram: &mut SequenceDiagram, lines: &BlocLines) -> CommandResult {
    let first = lines.first().expect("a block has its first line").trimmed();
    let arg = FIRST_LINE
        .matcher(first.text())
        .expect("the start pattern matched");
    let code = arg.get("CODE", 0).unwrap_or_default().to_owned();
    if diagram.contains_participant(&code) {
        diagram.put_participant_in_last(&code);
        return Ok(());
    }
    let body = lines.sub_extract(1, 1).without_empty_columns();
    let order = participant_order(&arg);
    let participant = diagram.create_new_participant(
        ParticipantType::Participant,
        &code,
        Some(body.to_display()),
        order,
    );
    decorate(diagram, participant, &arg)
}

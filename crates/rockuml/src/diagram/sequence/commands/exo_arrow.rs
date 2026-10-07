//! Messages from or to the diagram's border, like `[-> Alice` or `Alice ->]` (PlantUML's
//! `CommandExoArrowLeft`, `CommandExoArrowRight` and `CommandExoArrowAny`).

use super::arrow::{ANCHOR, apply_style, color_or_style_pattern};
use super::{PARTICIPANT_CODE_OR_QUOTED, activate, color_named, optional_url, unquoted};
use crate::command::{Command, CommandResult, SingleLine, SingleLineCommand};
use crate::creole::Display;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{
    Event, LifeEventType, MessageCommon, MessageExo, MessageExoType,
};
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree};
use crate::skin::arrow::{ArrowBody, ArrowConfiguration, ArrowDecoration, ArrowHead, ArrowPart};
use crate::text::LineLocation;

/// Which side of the participant the border is on.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

pub(super) fn left() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(SingleLine(CommandExoArrow {
        side: Side::Left,
        pattern: left_pattern(),
    }))
}

pub(super) fn right() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(SingleLine(CommandExoArrow {
        side: Side::Right,
        pattern: right_pattern(),
    }))
}

struct CommandExoArrow {
    side: Side,
    pattern: RegexTree,
}

/// The arrow itself; `ARROW_DRESSING1` points away from the first written end, `ARROW_DRESSING2` towards it.
fn arrow_body() -> RegexTree {
    RegexTree::or(vec![
        RegexTree::concat(vec![
            RegexTree::named(1, "ARROW_BOTHDRESSING", r"(<<?|//?|\\\\?)?"),
            RegexTree::named(1, "ARROW_BODYA1", r"(-+)"),
            RegexTree::named(1, "ARROW_STYLE1", color_or_style_pattern()),
            RegexTree::named(1, "ARROW_BODYB1", r"(-*)"),
            RegexTree::named(1, "ARROW_DRESSING1", r"(>>?|//?|\\\\?)"),
        ]),
        RegexTree::concat(vec![
            RegexTree::named(1, "ARROW_DRESSING2", r"(<<?|//?|\\\\?)"),
            RegexTree::named(1, "ARROW_BODYB2", r"(-*)"),
            RegexTree::named(1, "ARROW_STYLE2", color_or_style_pattern()),
            RegexTree::named(1, "ARROW_BODYA2", r"(-+)"),
        ]),
    ])
}

/// What follows the participant and the arrow: activation, colour, link and label.
fn tail() -> Vec<RegexTree> {
    vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "ACTIVATION", r"(?:([+*!-]+)?)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "LIFECOLOR", r"(?:(#\w+)?)"),
        RegexTree::spaces_zero_or_more(),
        optional_url(),
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::leaf(":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LABEL", "(.*)"),
        ])),
        RegexTree::end(),
    ]
}

fn left_pattern() -> RegexTree {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::named(1, "PARALLEL", r"(&[%s]*)?"),
        RegexTree::named(2, "ANCHOR", ANCHOR),
        RegexTree::named(1, "ARROW_SUPPCIRCLE2", r"([?\[\]][ox]?)?"),
        arrow_body(),
        RegexTree::named(1, "ARROW_SUPPCIRCLE1", r"([ox][%s]+)?"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "PARTICIPANT", PARTICIPANT_CODE_OR_QUOTED),
    ];
    parts.extend(tail());
    RegexTree::concat(parts)
}

fn right_pattern() -> RegexTree {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::named(1, "PARALLEL", r"(&[%s]*)?"),
        RegexTree::named(2, "ANCHOR", ANCHOR),
        RegexTree::named(1, "PARTICIPANT", PARTICIPANT_CODE_OR_QUOTED),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "ARROW_SUPPCIRCLE1", r"([%s]+[ox])?"),
        arrow_body(),
        RegexTree::named(1, "ARROW_SUPPCIRCLE2", r"([ox]?[?\]\[])?"),
    ];
    parts.extend(tail());
    RegexTree::concat(parts)
}

impl CommandExoArrow {
    fn message_exo_type(&self, arg: &RegexResult) -> MessageExoType {
        let border = arg.get("ARROW_SUPPCIRCLE2", 0).unwrap_or_default();
        let towards = arg.get("ARROW_DRESSING1", 0).is_some();
        match self.side {
            Side::Left if border.contains(']') => {
                if towards {
                    MessageExoType::FromRight
                } else {
                    MessageExoType::ToRight
                }
            }
            Side::Left => {
                if towards {
                    MessageExoType::FromLeft
                } else {
                    MessageExoType::ToLeft
                }
            }
            Side::Right if border.contains('[') => {
                if towards {
                    MessageExoType::ToLeft
                } else {
                    MessageExoType::FromLeft
                }
            }
            Side::Right => {
                if towards {
                    MessageExoType::ToRight
                } else {
                    MessageExoType::FromRight
                }
            }
        }
    }
}

fn arrow_part(dressing: &str, kind: MessageExoType) -> ArrowPart {
    let forward = kind.direction() == 1;
    if dressing.contains('/') {
        if forward {
            ArrowPart::BottomPart
        } else {
            ArrowPart::TopPart
        }
    } else if dressing.contains('\\') {
        if forward {
            ArrowPart::TopPart
        } else {
            ArrowPart::BottomPart
        }
    } else {
        ArrowPart::Full
    }
}

fn contains_symbol(arg: &RegexResult, name: &str, symbol: char) -> bool {
    arg.get(name, 0).is_some_and(|text| text.contains(symbol))
}

impl SingleLineCommand<SequenceDiagram> for CommandExoArrow {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(
        &self,
        diagram: &mut SequenceDiagram,
        _location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        let body = format!(
            "{}{}",
            arg.get_lazzy("ARROW_BODYA", 0).unwrap_or("null"),
            arg.get_lazzy("ARROW_BODYB", 0).unwrap_or("null")
        );
        let dressing = arg.get_lazzy("ARROW_DRESSING", 0).unwrap_or_default();
        let code = unquoted(arg.get("PARTICIPANT", 0).unwrap_or_default()).to_owned();
        let participant = diagram.get_or_create_participant(&code, None);
        let labels = match arg.get("LABEL", 0) {
            None => Display::create([""]),
            Some(label) => Display::with_newlines(label),
        };
        let mut configuration = if arg.get("ARROW_BOTHDRESSING", 0).is_some() {
            ArrowConfiguration::with_direction_both()
        } else {
            ArrowConfiguration::with_direction_normal()
        };
        if body.contains("--") {
            configuration = configuration.with_body(ArrowBody::Dotted);
        }
        if dressing.chars().count() == 2 {
            configuration = configuration.with_head(ArrowHead::Async);
        }
        let kind = self.message_exo_type(arg);
        configuration = configuration.with_part(arrow_part(dressing, kind));
        configuration = apply_style(arg.get_lazzy("ARROW_STYLE", 0), configuration)?;
        let activation_spec = arg.get("ACTIVATION", 0);
        if activation_spec.is_some_and(|spec| spec.starts_with('*')) {
            let _ = activate(diagram, participant, LifeEventType::Create, None);
        }
        let (near, far) = match kind {
            MessageExoType::ToRight | MessageExoType::ToLeft => {
                ("ARROW_SUPPCIRCLE1", "ARROW_SUPPCIRCLE2")
            }
            MessageExoType::FromRight | MessageExoType::FromLeft => {
                ("ARROW_SUPPCIRCLE2", "ARROW_SUPPCIRCLE1")
            }
        };
        if contains_symbol(arg, near, 'o') {
            configuration = configuration.with_decoration1(ArrowDecoration::Circle);
        }
        if contains_symbol(arg, near, 'x') {
            configuration = configuration.with_head1(ArrowHead::CrossX);
        }
        if contains_symbol(arg, far, 'o') {
            configuration = configuration.with_decoration2(ArrowDecoration::Circle);
        }
        if contains_symbol(arg, far, 'x') {
            configuration = configuration.with_head2(ArrowHead::CrossX);
        }
        let message_number = diagram.next_message_number();
        let mut common = MessageCommon::new(
            labels,
            configuration.clone(),
            message_number,
            diagram.style_builder(),
        );
        common.url = arg.get("URL", 0).and_then(Url::parse);
        common.parallel = arg.get("PARALLEL", 0).is_some();
        common.anchor = arg.get("ANCHOR", 1).map(str::to_owned);
        diagram.add_message(Event::MessageExo(MessageExo {
            common,
            participant,
            kind,
            short_arrow: contains_symbol(arg, "ARROW_SUPPCIRCLE2", '?'),
        }))?;

        let activation_color = arg.get("LIFECOLOR", 0).map(color_named).transpose()?;
        // PlantUML ignores refused activations here.
        let _ = match activation_spec.and_then(|spec| spec.chars().next()) {
            Some('+') => activate(
                diagram,
                participant,
                LifeEventType::Activate,
                activation_color,
            ),
            Some('-') => activate(diagram, participant, LifeEventType::Deactivate, None),
            Some('!') => activate(diagram, participant, LifeEventType::Destroy, None),
            Some(_) => Ok(()),
            None if diagram.is_autoactivate()
                && matches!(configuration.head(), ArrowHead::Normal | ArrowHead::Async) =>
            {
                if configuration.is_dotted() {
                    activate(diagram, participant, LifeEventType::Deactivate, None)
                } else {
                    activate(
                        diagram,
                        participant,
                        LifeEventType::Activate,
                        activation_color,
                    )
                }
            }
            None => Ok(()),
        };
        Ok(())
    }
}

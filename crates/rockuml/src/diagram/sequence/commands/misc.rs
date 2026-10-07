//! The smaller sequence commands, each a pattern and what it does.

use std::sync::LazyLock;

use regex::Regex;

use super::{
    PARTICIPANT_CODE_OR_QUOTED, activate as activate_participant, color_named, optional_colors,
    unquoted,
};
use crate::color::{ColorType, Colors};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, SingleLine, SingleLineCommand,
};
use crate::creole::Display;
use crate::diagram::sequence::autonumber::{DecimalFormat, DottedNumber};
use crate::diagram::sequence::model::{
    Event, GroupingType, LifeEventType, LiveColors, Message, MessageCommon, MessageExo,
    ParticipantId, Reference,
};
use crate::diagram::sequence::{LinkAnchor, SequenceDiagram};
use crate::diagram::titled::Positioned;
use crate::klimt::HorizontalAlignment;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::skin::arrow::ArrowBody;
use crate::stereo::{self, Stereotype};
use crate::text::LineLocation;

type Apply = fn(&mut SequenceDiagram, &LineLocation, &RegexResult) -> CommandResult;

/// A single-line command made of a pattern and what to do with what it matched.
struct Simple {
    pattern: RegexTree,
    apply: Apply,
}

impl SingleLineCommand<SequenceDiagram> for Simple {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(
        &self,
        diagram: &mut SequenceDiagram,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        (self.apply)(diagram, location, arg)
    }
}

/// A command whose pattern is `parts` between the start and the end of the line.
fn simple(parts: Vec<RegexTree>, apply: Apply) -> Box<dyn Command<SequenceDiagram>> {
    let mut all = vec![RegexTree::start()];
    all.extend(parts);
    all.push(RegexTree::end());
    Box::new(SingleLine(Simple {
        pattern: RegexTree::concat(all),
        apply,
    }))
}

fn leaf(pattern: &'static str) -> RegexTree {
    RegexTree::leaf(pattern)
}

fn named(group_count: usize, name: &'static str, pattern: &'static str) -> RegexTree {
    RegexTree::named(group_count, name, pattern)
}

fn spaces() -> RegexTree {
    RegexTree::spaces_zero_or_more()
}

fn some_spaces() -> RegexTree {
    RegexTree::spaces_one_or_more()
}

fn optional_color(
    arg: &RegexResult,
    name: &str,
) -> Result<Option<crate::color::HColor>, CommandError> {
    arg.get(name, 0).map(color_named).transpose()
}

pub(super) fn hide_unlinked() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            named(1, "HIDE", "(hide|show)"),
            some_spaces(),
            leaf("@?unlinked"),
        ],
        |diagram, _, arg| {
            diagram.set_hide_unlinked(
                arg.get("HIDE", 0)
                    .is_some_and(|hide| hide.eq_ignore_ascii_case("hide")),
            );
            Ok(())
        },
    )
}

pub(super) fn activate() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            named(1, "TYPE", "(activate|deactivate|destroy|create)"),
            some_spaces(),
            named(1, "WHO", PARTICIPANT_CODE_OR_QUOTED),
            spaces(),
            named(1, "BACK", r"(#\w+)?"),
            RegexTree::optional(RegexTree::concat(vec![
                some_spaces(),
                named(1, "LINE", r"(#\w+)"),
            ])),
        ],
        |diagram, location, arg| {
            let kind = match arg
                .get("TYPE", 0)
                .unwrap_or_default()
                .to_lowercase()
                .as_str()
            {
                "activate" => LifeEventType::Activate,
                "deactivate" => LifeEventType::Deactivate,
                "destroy" => LifeEventType::Destroy,
                _ => LifeEventType::Create,
            };
            let code = unquoted(arg.get("WHO", 0).unwrap_or_default()).to_owned();
            let participant = diagram.get_or_create_participant(location, &code, None);
            let colors = LiveColors {
                back: optional_color(arg, "BACK")?,
                line: optional_color(arg, "LINE")?,
            };
            diagram
                .activate(participant, kind, colors)
                .map_err(CommandError::new)
        },
    )
}

pub(super) fn deactivate_short() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![named(0, "TYPE", "deactivate"), spaces()],
        |diagram, _, _| {
            let message = diagram
                .activating_message()
                .ok_or_else(|| CommandError::new("Nothing to deactivate."))?;
            let participant = receiver(diagram, message);
            activate_participant(diagram, participant, LifeEventType::Deactivate, None)
        },
    )
}

/// The participant a message goes to.
fn receiver(diagram: &SequenceDiagram, message: usize) -> ParticipantId {
    match diagram.event(message) {
        Event::Message(message) => message.participant2,
        Event::MessageExo(exo) => exo.participant,
        _ => unreachable!("activations come from messages"),
    }
}

pub(super) fn activate_shortcut() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            named(1, "NAME", r"([%pLN_.@]+)"),
            spaces(),
            named(1, "TYPE", r"(\+\+|--)"),
            spaces(),
            named(1, "COLOR", r"(#\w+)?"),
        ],
        |diagram, location, arg| {
            let kind = if arg.get("TYPE", 0) == Some("++") {
                LifeEventType::Activate
            } else {
                LifeEventType::Deactivate
            };
            let code = arg.get("NAME", 0).unwrap_or_default().to_owned();
            let participant = diagram.get_or_create_participant(location, &code, None);
            let color = optional_color(arg, "COLOR")?;
            activate_participant(diagram, participant, kind, color)
        },
    )
}

pub(super) fn box_start() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("box"),
            RegexTree::optional(RegexTree::or(vec![
                RegexTree::concat(vec![some_spaces(), named(1, "NAME1", r"[%g]([^%g]+)[%g]")]),
                RegexTree::concat(vec![some_spaces(), named(1, "NAME2", r"([^#]+)")]),
            ])),
            stereo::optional_pattern("STEREO"),
            optional_colors(),
        ],
        |diagram, _, arg| {
            let title = arg.get_lazzy("NAME", 0).unwrap_or_default();
            let stereotype = arg.get("STEREO", 0).map(Stereotype::new);
            let colors = arg
                .get("COLOR", 0)
                .map(|data| {
                    Colors::parse(data, ColorType::Back).map_err(|_| super::no_such_color())
                })
                .transpose()?
                .unwrap_or_default();
            diagram.box_start(
                Display::with_newlines(title),
                colors.get(ColorType::Back).cloned(),
                stereotype,
            );
            Ok(())
        },
    )
}

pub(super) fn box_end() -> Box<dyn Command<SequenceDiagram>> {
    simple(vec![leaf("end"), spaces(), leaf("box")], |diagram, _, _| {
        if diagram.end_box() {
            Ok(())
        } else {
            Err(CommandError::new("Missing starting box"))
        }
    })
}

pub(super) fn grouping() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            named(1, "PARALLEL", r"(&[%s]*)?"),
            named(
                1,
                "TYPE",
                "(opt|alt|loop|par|par2|break|critical|else|end|also|group|partition)",
            ),
            named(
                2,
                "COLORS",
                r"((?<!else)(?<!also)(?<!end)#\w+)?(?:[%s]+(#\w+))?",
            ),
            RegexTree::optional(RegexTree::concat(vec![
                some_spaces(),
                named(1, "COMMENT", "(.*?)"),
            ])),
        ],
        |diagram, _, arg| {
            static TRAILING_BRACKET: LazyLock<Regex> =
                LazyLock::new(|| plantuml_regex(r"^(.*\[\[.*\]\].*?|.*?)\[(.*)\]$"));
            let back_color_element = optional_color_at(arg, 0)?;
            let back_color_general = optional_color_at(arg, 1)?;
            let mut kind_name = arg.get("TYPE", 0).unwrap_or_default().to_lowercase();
            let mut comment = arg.get("COMMENT", 0).map(str::to_owned);
            let kind =
                GroupingType::named(&kind_name).expect("the pattern only matches group keywords");
            if kind_name == "group" {
                match comment.as_deref() {
                    None | Some("") => comment = Some("group".to_owned()),
                    Some(text) => {
                        if let Some(captures) = TRAILING_BRACKET.captures(text) {
                            kind_name = captures[1].to_owned();
                            comment = Some(captures[2].to_owned());
                        }
                    }
                }
            }
            let parallel = arg.get("PARALLEL", 0).is_some();
            if diagram.grouping(
                &kind_name,
                comment.as_deref(),
                kind,
                back_color_general,
                back_color_element,
                parallel,
            ) {
                Ok(())
            } else {
                Err(CommandError::new("Cannot create group"))
            }
        },
    )
}

fn optional_color_at(
    arg: &RegexResult,
    index: usize,
) -> Result<Option<crate::color::HColor>, CommandError> {
    arg.get("COLORS", index).map(color_named).transpose()
}

pub(super) fn return_command() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            named(1, "PARALLEL", r"(&[%s]*)?"),
            spaces(),
            leaf("return"),
            spaces(),
            RegexTree::optional(RegexTree::concat(vec![
                named(1, "COLOR", r"(#\w+)"),
                some_spaces(),
            ])),
            named(1, "MESSAGE", "(.*)"),
        ],
        |diagram, location, arg| {
            let (message1, deactivate) = match diagram.activating_message() {
                Some(message) => (message, true),
                None => match diagram.last_event_with_deactivate_scan() {
                    Some(last) if matches!(diagram.event(last), Event::Message(_)) => (last, false),
                    _ => return Err(CommandError::new("Nowhere to return to.")),
                },
            };
            let common = diagram.event(message1).message_common().expect("a message");
            let mut arrow = common.arrow_configuration.with_body(ArrowBody::Dotted);
            if let Some(color) = optional_color(arg, "COLOR")? {
                arrow = arrow.with_color(color);
            }
            let display = Display::with_newlines(arg.get("MESSAGE", 0).unwrap_or_default());
            let number = diagram.next_message_number();
            let mut reply = MessageCommon::new(
                display,
                arrow,
                number,
                diagram.style_builder(),
                location.clone(),
            );
            let event = match diagram.event(message1) {
                Event::MessageExo(exo) => Event::MessageExo(MessageExo {
                    common: reply,
                    participant: exo.participant,
                    kind: exo.kind.reverse(),
                    short_arrow: false,
                }),
                Event::Message(message) => {
                    reply.parallel = arg.get("PARALLEL", 0).is_some();
                    Event::Message(Message {
                        common: reply,
                        participant1: message.participant2,
                        participant2: message.participant1,
                        multicast: Vec::new(),
                    })
                }
                _ => unreachable!("only messages are returned from"),
            };
            diagram.add_message(event)?;
            if deactivate {
                let participant = receiver(diagram, message1);
                activate_participant(diagram, participant, LifeEventType::Deactivate, None)?;
            }
            Ok(())
        },
    )
}

pub(super) fn newpage() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("@?"),
            leaf("newpage"),
            RegexTree::optional(RegexTree::concat(vec![
                leaf(r"(?:[%s]*:[%s]*|[%s]+)"),
                named(1, "LABEL", r"(.*[%pLN_.].*)"),
            ])),
        ],
        |diagram, location, arg| {
            let display = arg
                .get("LABEL", 0)
                .map(Display::with_newlines)
                .unwrap_or_default();
            diagram.newpage(Positioned {
                display,
                alignment: HorizontalAlignment::Center,
                location: Some(location.clone()),
            });
            Ok(())
        },
    )
}

pub(super) fn ignore_newpage() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![leaf("ignore"), spaces(), leaf("newpage")],
        |diagram, _, _| {
            diagram.ignore_newpage();
            Ok(())
        },
    )
}

/// `autonewpage`, which PlantUML accepts and ignores.
pub(super) fn auto_newpage() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("autonewpage"),
            some_spaces(),
            named(1, "VALUE", r"(\d+)"),
        ],
        |_, _, _| Ok(()),
    )
}

pub(super) fn divider() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("=="),
            spaces(),
            named(1, "LABEL", "(.*)"),
            spaces(),
            leaf("=="),
        ],
        |diagram, _, arg| {
            diagram.divider(Display::with_newlines(
                arg.get("LABEL", 0).unwrap_or_default(),
            ));
            Ok(())
        },
    )
}

pub(super) fn hspace() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![leaf(r"\|\|"), named(1, "VALUE", r"(\d+)?"), leaf(r"\|+")],
        |diagram, _, arg| {
            let pixels = arg
                .get("VALUE", 0)
                .filter(|value| !value.is_empty())
                .map_or(25, |value| value.parse().unwrap_or(25));
            diagram.hspace(pixels);
            Ok(())
        },
    )
}

pub(super) fn delay() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("(?:\\.{3}|\u{2026})"),
            RegexTree::optional(named(1, "LABEL", "(.*)(?:\\.{3}|\u{2026})")),
        ],
        |diagram, _, arg| {
            let display = arg
                .get("LABEL", 0)
                .map(Display::with_newlines)
                .unwrap_or_default();
            diagram.delay(display);
            Ok(())
        },
    )
}

const PARTS_PATTERN: &str =
    r"(([%pLN_.@]+|[%g][^%g]+[%g])([%s]*,[%s]*([%pLN_.@]+|[%g][^%g]+[%g]))*)";

/// The participants of `ref over A, B`, created if new.
fn reference_participants(
    diagram: &mut SequenceDiagram,
    location: &LineLocation,
    parts: &str,
) -> Vec<ParticipantId> {
    let mut result: Vec<ParticipantId> = Vec::new();
    for part in parts.split(',') {
        let code = unquoted(crate::java::trim(part)).to_owned();
        let participant = diagram.get_or_create_participant(location, &code, None);
        if !result.contains(&participant) {
            result.push(participant);
        }
    }
    result
}

fn add_reference(
    diagram: &mut SequenceDiagram,
    participants: Vec<ParticipantId>,
    url: Option<&str>,
    display: Display,
    back_color_element: Option<crate::color::HColor>,
) {
    let style_builder = diagram.style_builder();
    diagram.add_event(Event::Reference(Reference {
        participants,
        url: url.and_then(Url::parse),
        display,
        back_color_element,
        notes: Vec::new(),
        style_builder,
    }));
}

pub(super) fn reference_over_several() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("ref"),
            named(1, "REF", r"(#\w+)?"),
            some_spaces(),
            leaf("over"),
            some_spaces(),
            named(4, "PARTS", PARTS_PATTERN),
            spaces(),
            RegexTree::optional(named(1, "URL", r"(\[\[.*?\]\])")),
            spaces(),
            leaf(":"),
            spaces(),
            named(1, "TEXT", "(.*)"),
        ],
        |diagram, location, arg| {
            let back_color_element = optional_color(arg, "REF")?;
            let participants =
                reference_participants(diagram, location, arg.get("PARTS", 0).unwrap_or_default());
            let text = crate::java::trim(arg.get("TEXT", 0).unwrap_or_default());
            add_reference(
                diagram,
                participants,
                arg.get("URL", 0),
                Display::with_newlines(text),
                back_color_element,
            );
            Ok(())
        },
    )
}

static REFERENCE_FIRST_LINE: LazyLock<RegexTree> = LazyLock::new(|| {
    RegexTree::concat(vec![
        RegexTree::start(),
        leaf("ref"),
        named(1, "REF", r"(#\w+)?"),
        some_spaces(),
        leaf("over"),
        some_spaces(),
        named(
            1,
            "PARTS",
            r"((?:[%pLN_.@]+|[%g][^%g]+[%g])(?:[%s]*,[%s]*(?:[%pLN_.@]+|[%g][^%g]+[%g]))*)",
        ),
        spaces(),
        RegexTree::optional(named(1, "URL", r"(\[\[.*?\]\])")),
        spaces(),
        named(1, "UNUSED", r"(#\w+)?"),
        RegexTree::end(),
    ])
});

static REFERENCE_END: LazyLock<Regex> = LazyLock::new(|| plantuml_regex("^end[%s]?(ref)?$"));

pub(super) fn reference_multiline_over_several() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(Multiline::starting_with(
        &REFERENCE_FIRST_LINE,
        &REFERENCE_END,
        reference_block,
    ))
}

fn reference_block(diagram: &mut SequenceDiagram, lines: &BlocLines) -> CommandResult {
    let first = lines.first().expect("a block has its first line").trimmed();
    let arg = REFERENCE_FIRST_LINE
        .matcher(first.text())
        .ok_or_else(|| CommandError::new(format!("Cannot parse line {}", first.text())))?;
    let back_color_element = optional_color(&arg, "REF")?;
    let body = lines.sub_extract(1, 1).without_empty_columns();
    let location = body
        .first()
        .map_or_else(|| first.location().clone(), |line| line.location().clone());
    let participants =
        reference_participants(diagram, &location, arg.get("PARTS", 0).unwrap_or_default());
    add_reference(
        diagram,
        participants,
        arg.get("URL", 0),
        body.to_display(),
        back_color_element,
    );
    Ok(())
}

pub(super) fn autonumber() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("autonumber"),
            spaces(),
            named(1, "START", r"(\d(?:(?:[^%pLN%s]+|\d+)*\d)?)?"),
            RegexTree::optional(RegexTree::concat(vec![
                some_spaces(),
                named(1, "STEP", r"(\d+)"),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                some_spaces(),
                named(1, "FORMAT", r"[%g]([^%g]+)[%g]"),
            ])),
            spaces(),
        ],
        |diagram, _, arg| {
            let start = DottedNumber::parse(arg.get("START", 0).unwrap_or("1"));
            let step = arg
                .get("STEP", 0)
                .map_or(1, |step| step.parse().unwrap_or(1));
            let pattern = arg.get("FORMAT", 0).unwrap_or("<b>0</b>");
            let format = decimal_format(pattern)?;
            diagram.autonumber().go(start, step, format);
            Ok(())
        },
    )
}

fn decimal_format(pattern: &str) -> Result<DecimalFormat, CommandError> {
    DecimalFormat::new(pattern)
        .map_err(|_| CommandError::new(format!("Error in pattern : {pattern}")))
}

pub(super) fn autonumber_stop() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![leaf("autonumber"), some_spaces(), leaf("stop"), spaces()],
        |diagram, _, _| {
            diagram.autonumber().stop();
            Ok(())
        },
    )
}

pub(super) fn autonumber_resume() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("autonumber"),
            some_spaces(),
            leaf("resume"),
            spaces(),
            RegexTree::optional(RegexTree::concat(vec![
                some_spaces(),
                named(1, "INC", r"(\d+)"),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                some_spaces(),
                named(1, "DF", r"[%g]([^%g]+)[%g]"),
            ])),
        ],
        |diagram, _, arg| {
            let format = arg.get("DF", 0).map(decimal_format).transpose()?;
            let increment = arg.get("INC", 0).map(|inc| inc.parse().unwrap_or(1));
            diagram.autonumber().resume(increment, format);
            Ok(())
        },
    )
}

pub(super) fn autonumber_increment() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("autonumber"),
            some_spaces(),
            leaf("inc"),
            RegexTree::optional(RegexTree::concat(vec![
                some_spaces(),
                named(1, "POS", "([A-Za-z])"),
            ])),
            spaces(),
        ],
        |diagram, _, arg| {
            if let Some(current) = diagram.autonumber().current_mut() {
                match arg
                    .get("POS", 0)
                    .and_then(|position| position.chars().next())
                {
                    None => current.increment_intermediate(),
                    Some(letter) => {
                        let position = letter.to_ascii_lowercase() as usize - 'a' as usize;
                        current.increment_at(position);
                    }
                }
            }
            Ok(())
        },
    )
}

pub(super) fn autoactivate() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("autoactivate"),
            some_spaces(),
            named(1, "ON", "(off|on)"),
        ],
        |diagram, _, arg| {
            diagram.set_autoactivate(
                arg.get("ON", 0)
                    .is_some_and(|on| on.eq_ignore_ascii_case("on")),
            );
            Ok(())
        },
    )
}

pub(super) fn footbox() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![named(1, "TYPE", "(hide|show)?"), spaces(), leaf("footbox")],
        |diagram, _, arg| {
            diagram.set_show_footbox(
                arg.get("TYPE", 0)
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("show")),
            );
            Ok(())
        },
    )
}

pub(super) fn footbox_old() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("footbox"),
            spaces(),
            named(1, "TYPE", "(on|off)?"),
            spaces(),
        ],
        |diagram, _, arg| {
            diagram.set_show_footbox(
                arg.get("TYPE", 0)
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("on")),
            );
            Ok(())
        },
    )
}

pub(super) fn url() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            leaf("url"),
            spaces(),
            RegexTree::optional(leaf("of|for")),
            some_spaces(),
            named(1, "CODE", PARTICIPANT_CODE_OR_QUOTED),
            some_spaces(),
            RegexTree::optional(leaf("is")),
            spaces(),
            RegexTree::named(12, "URL", Url::command_pattern()),
        ],
        |diagram, location, arg| {
            let code = arg.get("CODE", 0).unwrap_or_default().to_owned();
            let participant = diagram.get_or_create_participant(location, &code, None);
            diagram.participant_mut(participant).url = arg.get("URL", 0).and_then(Url::parse);
            Ok(())
        },
    )
}

pub(super) fn link_anchor() -> Box<dyn Command<SequenceDiagram>> {
    simple(
        vec![
            named(1, "ANCHOR1", r"\{([%pLN_]+)\}"),
            spaces(),
            named(0, "LINK", r"\<-\>"),
            spaces(),
            named(1, "ANCHOR2", r"\{([%pLN_]+)\}"),
            spaces(),
            named(1, "MESSAGE", r"(?::[%s]*(.*))?"),
        ],
        |diagram, _, arg| {
            diagram.link_anchor(LinkAnchor {
                anchor1: arg.get("ANCHOR1", 0).unwrap_or_default().to_owned(),
                anchor2: arg.get("ANCHOR2", 0).unwrap_or_default().to_owned(),
                message: arg.get("MESSAGE", 0).map(str::to_owned),
            });
            Ok(())
        },
    )
}

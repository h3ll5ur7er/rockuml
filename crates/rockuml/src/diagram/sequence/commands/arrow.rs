//! Messages between participants, like `Alice -> Bob : hello` (PlantUML's `CommandArrow`).

use super::{activate, color_named, optional_url};
use crate::color::HColor;
use crate::command::{Command, CommandError, CommandResult, SingleLine, SingleLineCommand};
use crate::creole::Display;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{
    Event, LifeEventType, Message, MessageCommon, ParticipantId,
};
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree};
use crate::skin::arrow::{ArrowBody, ArrowConfiguration, ArrowDecoration, ArrowHead, ArrowPart};
use crate::stereo::{self, Stereotype};
use crate::text::LineLocation;

/// An anchor like `{start}` that `{start} <-> {end}` refers to.
pub(super) const ANCHOR: &str = r"(\{([%pLN_]+)\}[%s]+)?";

const LINE_STYLE: &str = concat!(
    r"(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)",
    r"(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*"
);

/// `[#red,dashed]` inside an arrow's body.
pub(super) fn color_or_style_pattern() -> String {
    format!(r"(?:\[({LINE_STYLE})\])?")
}

pub(super) fn command() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(SingleLine(CommandArrow(pattern())))
}

struct CommandArrow(RegexTree);

fn participant(prefix: &'static [&'static str; 5]) -> RegexTree {
    RegexTree::named_or(
        prefix[0],
        vec![
            RegexTree::named(1, prefix[1], r"([%pLN_.@]+)"),
            RegexTree::named(1, prefix[2], r"[%g]([^%g]+)[%g]"),
            RegexTree::named(2, prefix[3], r"[%g]([^%g]+)[%g][%s]*as[%s]+([%pLN_.@]+)"),
            RegexTree::named(2, prefix[4], r"([%pLN_.@]+)[%s]+as[%s]*[%g]([^%g]+)[%g]"),
        ],
    )
}

const PART1: [&str; 5] = [
    "PART1",
    "PART1CODE",
    "PART1LONG",
    "PART1LONGCODE",
    "PART1CODELONG",
];
const PART2: [&str; 5] = [
    "PART2",
    "PART2CODE",
    "PART2LONG",
    "PART2LONGCODE",
    "PART2CODELONG",
];

fn pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::named(1, "PARALLEL", r"(&[%s]*)?"),
        RegexTree::named(2, "ANCHOR", ANCHOR),
        participant(&PART1),
        RegexTree::named(2, "PART1ANCHOR", ANCHOR),
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::named_or(
            "ARROW_DRESSING1",
            vec![
                RegexTree::leaf(r"[%s][ox]"),
                RegexTree::leaf(r"(?:[%s][ox]|\(\d+\))?<<?_?"),
                RegexTree::leaf(r"(?:[%s][ox])?//?"),
                RegexTree::leaf(r"(?:[%s][ox])?\\\\?"),
            ],
        )),
        RegexTree::or(vec![
            RegexTree::concat(vec![
                RegexTree::named(1, "ARROW_BODYA1", r"(-+)"),
                RegexTree::named(1, "ARROW_STYLE1", color_or_style_pattern()),
                RegexTree::named(1, "ARROW_BODYB1", r"(-*)"),
            ]),
            RegexTree::concat(vec![
                RegexTree::named(1, "ARROW_BODYA2", r"(-*)"),
                RegexTree::named(1, "ARROW_STYLE2", color_or_style_pattern()),
                RegexTree::named(1, "ARROW_BODYB2", r"(-+)"),
            ]),
        ]),
        RegexTree::optional(RegexTree::named_or(
            "ARROW_DRESSING2",
            vec![
                RegexTree::leaf(r"_?>>?(?:[ox][%s]|\(\d+\))?"),
                RegexTree::leaf(r"//?(?:[ox][%s])?"),
                RegexTree::leaf(r"\\\\?(?:[ox][%s])?"),
                RegexTree::leaf(r"[ox][%s]"),
            ],
        )),
        RegexTree::spaces_zero_or_more(),
        participant(&PART2),
        RegexTree::named(1, "MULTICAST", r"((?:\s&\s[%pLN_.@]+)*)"),
        RegexTree::named(2, "PART2ANCHOR", ANCHOR),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "ACTIVATION", r"(?:(\+\+|\*\*|!!|--|--\+\+|\+\+--)?)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "LIFECOLOR", r"(?:(#\w+)?)"),
        stereo::optional_pattern("STEREOTYPE"),
        optional_url(),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "MESSAGE", r"(?::[%s]*(.*))?"),
        RegexTree::end(),
    ])
}

/// A dressing as written, lowercased and without `_`.
fn dressing(arg: &RegexResult, name: &str) -> String {
    arg.get(name, 0)
        .unwrap_or_default()
        .replace('_', "")
        .to_lowercase()
}

/// The `(10)` of a slanted arrow.
fn inclination(dressing: Option<&str>) -> i32 {
    let Some(dressing) = dressing else {
        return 0;
    };
    match (dressing.find('('), dressing.find(')')) {
        (Some(open), Some(close)) => dressing[open + 1..close].parse().unwrap_or(0),
        _ => 0,
    }
}

fn contains_any(text: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| text.contains(candidate))
}

/// The participant written in one of the four forms, created if new.
fn get_or_create(
    diagram: &mut SequenceDiagram,
    location: &LineLocation,
    arg: &RegexResult,
    names: &[&str; 5],
) -> ParticipantId {
    let (code, display) = if let Some(code) = arg.get(names[1], 0) {
        (code, Display::with_newlines(code))
    } else if let Some(long) = arg.get(names[2], 0) {
        (long, Display::with_newlines(long))
    } else if let Some(long) = arg.get(names[3], 0) {
        (
            arg.get(names[3], 1).unwrap_or_default(),
            Display::with_newlines(long),
        )
    } else {
        let code = arg.get(names[4], 0).unwrap_or_default();
        (
            code,
            Display::with_newlines(arg.get(names[4], 1).unwrap_or_default()),
        )
    };
    let code = code.to_owned();
    diagram.get_or_create_participant(location, &code, Some(display))
}

/// The arrow's `[...]` style: colours, `dashed`, `dotted` and `hidden`; `bold` changes nothing here.
pub(super) fn apply_style(
    style: Option<&str>,
    mut configuration: ArrowConfiguration,
) -> Result<ArrowConfiguration, CommandError> {
    let Some(style) = style else {
        return Ok(configuration);
    };
    for part in style.split(',').filter(|part| !part.is_empty()) {
        configuration =
            if part.eq_ignore_ascii_case("dashed") || part.eq_ignore_ascii_case("dotted") {
                configuration.with_body(ArrowBody::Dotted)
            } else if part.eq_ignore_ascii_case("bold") {
                configuration
            } else if part.eq_ignore_ascii_case("hidden") {
                configuration.with_body(ArrowBody::Hidden)
            } else {
                configuration.with_color(color_named(part)?)
            };
    }
    Ok(configuration)
}

fn body_length(arg: &RegexResult) -> usize {
    let a = arg.get_lazzy("ARROW_BODYA", 0).unwrap_or_default();
    let b = arg.get_lazzy("ARROW_BODYB", 0).unwrap_or_default();
    a.len() + b.len()
}

impl SingleLineCommand<SequenceDiagram> for CommandArrow {
    fn pattern(&self) -> &RegexTree {
        &self.0
    }

    fn execute_arg(
        &self,
        diagram: &mut SequenceDiagram,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        let dressing1 = dressing(arg, "ARROW_DRESSING1");
        let dressing2 = dressing(arg, "ARROW_DRESSING2");
        let inclination1 = inclination(arg.get("ARROW_DRESSING1", 0));
        let inclination2 = inclination(arg.get("ARROW_DRESSING2", 0));
        let has_dressing1_but_x = contains_any(&dressing1, &["<", "\\", "/"]);
        let x_in_dressing1 = dressing1.contains('x');
        let has_dressing2_but_x = contains_any(&dressing2, &[">", "\\", "/"]);
        let x_in_dressing2 = dressing2.contains('x');
        let reverse_define = if has_dressing2_but_x || (x_in_dressing1 && x_in_dressing2) {
            false
        } else if has_dressing1_but_x {
            true
        } else if x_in_dressing1 || x_in_dressing2 {
            false
        } else {
            return Err(CommandError::new("Illegal sequence arrow"));
        };

        let async_marks = ["<<", "\\\\", "//"];
        let async_marks2 = [">>", "\\\\", "//"];
        let (participant1, participant2, circle_at_start, circle_at_end, sync1, sync2) =
            if reverse_define {
                let p2 = get_or_create(diagram, location, arg, &PART1);
                let p1 = get_or_create(diagram, location, arg, &PART2);
                (
                    p1,
                    p2,
                    dressing2.contains('o'),
                    dressing1.contains('o'),
                    contains_any(&dressing2, &async_marks2),
                    contains_any(&dressing1, &async_marks),
                )
            } else {
                let p1 = get_or_create(diagram, location, arg, &PART1);
                let p2 = get_or_create(diagram, location, arg, &PART2);
                (
                    p1,
                    p2,
                    dressing1.contains('o'),
                    dressing2.contains('o'),
                    contains_any(&dressing1, &async_marks),
                    contains_any(&dressing2, &async_marks2),
                )
            };

        let labels = match arg.get("MESSAGE", 0) {
            None => Display::create([""]),
            Some(message) => Display::with_newlines(message),
        };
        let mut configuration = if has_dressing1_but_x && has_dressing2_but_x {
            ArrowConfiguration::with_direction_both()
        } else {
            ArrowConfiguration::with_direction_normal()
        };
        if body_length(arg) > 1 {
            configuration = configuration.with_body(ArrowBody::Dotted);
        }
        if sync1 {
            configuration = configuration.with_head1(ArrowHead::Async);
        }
        if sync2 {
            configuration = configuration.with_head2(ArrowHead::Async);
        }
        if dressing2.contains('\\') || dressing1.contains('/') {
            configuration = configuration.with_part(ArrowPart::TopPart);
        }
        if dressing2.contains('/') || dressing1.contains('\\') {
            configuration = configuration.with_part(ArrowPart::BottomPart);
        }
        if circle_at_end {
            configuration = configuration.with_decoration2(ArrowDecoration::Circle);
        }
        if circle_at_start {
            configuration = configuration.with_decoration1(ArrowDecoration::Circle);
        }
        let (cross1, cross2) = if reverse_define {
            (x_in_dressing2, x_in_dressing1)
        } else {
            (x_in_dressing1, x_in_dressing2)
        };
        if cross1 {
            configuration = configuration.with_head1(ArrowHead::CrossX);
        }
        if cross2 {
            configuration = configuration.with_head2(ArrowHead::CrossX);
        }
        if reverse_define {
            configuration = configuration.reverse_define();
        }
        configuration = apply_style(arg.get_lazzy("ARROW_STYLE", 0), configuration)?;
        configuration = configuration.with_inclination(inclination1 + inclination2);

        let activation_spec = arg.get("ACTIVATION", 0);
        if activation_spec.is_some_and(|spec| spec.starts_with('*')) {
            // PlantUML ignores whether a creation was refused here.
            let _ = activate(diagram, participant2, LifeEventType::Create, None);
        }
        let message_number = diagram.next_message_number();
        let mut common = MessageCommon::new(
            diagram.manage_variable(labels),
            configuration.clone(),
            message_number,
            diagram.style_builder(),
            location.clone(),
        );
        let multicast = multicasts(diagram, location, arg.get("MULTICAST", 0));
        common.url = arg.get("URL", 0).and_then(Url::parse);
        common.stereotype = arg.get("STEREOTYPE", 0).map(Stereotype::new);
        common.parallel = arg.get("PARALLEL", 0).is_some();
        common.anchor = arg.get("ANCHOR", 1).map(str::to_owned);
        common.part1_anchor = arg.get("PART1ANCHOR", 1).map(str::to_owned);
        common.part2_anchor = arg.get("PART2ANCHOR", 1).map(str::to_owned);
        diagram.add_message(Event::Message(Message {
            common,
            participant1,
            participant2,
            multicast,
        }))?;

        let activation_color = arg.get("LIFECOLOR", 0).map(color_named).transpose()?;
        if let Some(spec) = activation_spec {
            manage_activations(diagram, spec, participant1, participant2, activation_color);
            return Ok(());
        }
        if diagram.is_autoactivate()
            && matches!(configuration.head(), ArrowHead::Normal | ArrowHead::Async)
        {
            // PlantUML ignores whether an automatic activation was refused.
            let _ = if configuration.is_dotted() {
                activate(diagram, participant1, LifeEventType::Deactivate, None)
            } else {
                activate(
                    diagram,
                    participant2,
                    LifeEventType::Activate,
                    activation_color,
                )
            };
        }
        Ok(())
    }
}

/// `++`, `--`, `!!`, `--++` and `++--` after a message; refusals are ignored, as in PlantUML.
fn manage_activations(
    diagram: &mut SequenceDiagram,
    spec: &str,
    participant1: ParticipantId,
    participant2: ParticipantId,
    color: Option<HColor>,
) {
    let mut apply = |sign| {
        let _ = match sign {
            '+' => activate(
                diagram,
                participant2,
                LifeEventType::Activate,
                color.clone(),
            ),
            '-' => activate(diagram, participant1, LifeEventType::Deactivate, None),
            '!' => activate(diagram, participant2, LifeEventType::Destroy, None),
            _ => Ok(()),
        };
    };
    let signs: Vec<char> = spec.chars().collect();
    apply(signs[0]);
    if signs.len() == 4 {
        apply(signs[2]);
    }
}

/// `Alice -> Bob & Carol`: the other receivers.
fn multicasts(
    diagram: &mut SequenceDiagram,
    location: &LineLocation,
    multicast: Option<&str>,
) -> Vec<ParticipantId> {
    multicast
        .unwrap_or_default()
        .split('&')
        .map(crate::java::trim)
        .filter(|code| !code.is_empty())
        .map(|code| diagram.get_or_create_participant(location, code, None))
        .collect()
}

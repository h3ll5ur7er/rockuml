//! The commands of timing diagrams (PlantUML's `timingdiagram.command` package).

use super::player::{
    NotePosition, PlayerAnalog, PlayerBinary, PlayerClock, PlayerKind, TimingStyle,
};
use super::ruler::TimeAxisStategy;
use super::time::{BigDecimal, TimeTick, TimingFormat};
use super::{TimingDiagram, constraint_color};
use crate::color::{self, ColorType, Colors};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, PatternCommand, SingleLine,
};
use crate::creole::Display;
use crate::diagram::common_commands::add_common_commands1;
use crate::diagram::cuca_commands;
use crate::diagram::description::arrow_style;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stereo::{self, Stereotype};
use crate::text::LineLocation;

/// The commands in PlantUML's order.
pub(super) fn all() -> Vec<Box<dyn Command<TimingDiagram>>> {
    let mut commands = add_common_commands1();
    commands.extend([
        cuca_commands::footbox_ignored(),
        robust_concise(),
        clock(),
        analog(),
        binary(),
        define_state_short(),
        define_state_long(),
        change_state_by_player_code(),
        change_state_by_time(),
        at_time(),
        at_player(),
        time_message(),
        note(),
        note_long(),
        constraint(),
        scale_pixel(),
        hide_time_axis(),
        highlight(),
        mode_compact(),
        ticks(),
        pixel_height(),
        use_date_format(),
    ]);
    commands
}

fn command(
    pattern: RegexTree,
    apply: fn(&mut TimingDiagram, &RegexResult) -> CommandResult,
) -> Box<dyn Command<TimingDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        pattern,
        move |diagram: &mut TimingDiagram, _: &LineLocation, arg: &RegexResult| apply(diagram, arg),
    )))
}

/// The names a time expression is captured under (`TimeTickBuilder.expressionAtWithoutArobase`).
struct TimeNames {
    code: &'static str,
    date: &'static str,
    hour: &'static str,
    digit: &'static str,
    clock: &'static str,
}

macro_rules! time_names {
    ($prefix:literal) => {
        TimeNames {
            code: concat!($prefix, "CODE"),
            date: concat!($prefix, "DATE"),
            hour: concat!($prefix, "HOUR"),
            digit: concat!($prefix, "DIGIT"),
            clock: concat!($prefix, "CLOCK"),
        }
    };
}

const TIME: TimeNames = time_names!("TIME");
const TIME1: TimeNames = time_names!("TIME1");
const TIME2: TimeNames = time_names!("TIME2");
const FROM: TimeNames = time_names!("FROM");
const TO: TimeNames = time_names!("TO");

/// `:code`, `:code+5`, `2019/07/02`, `10:15:00`, `+50`, `100` or `clk*3`.
fn expression_at_without_arobase(names: &TimeNames) -> RegexTree {
    RegexTree::or(vec![
        RegexTree::named(2, names.code, r":([%pLN_.]+)([-+][.\d]+)?"),
        RegexTree::named(3, names.date, r"(\d+)/(\d+)/(\d+)"),
        RegexTree::named(3, names.hour, r"(\d+):(\d+):(\d+)"),
        RegexTree::named(2, names.digit, r"(\+?)(-?\d+\.?\d*)"),
        RegexTree::named(2, names.clock, r"([%pLN_.@]+)\*(\d+)"),
    ])
}

fn expression_at_with_arobase(names: &TimeNames) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::leaf(r"@"),
        expression_at_without_arobase(names),
    ])
}

fn integer(arg: &RegexResult, name: &str, index: usize) -> i64 {
    arg.get(name, index)
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

/// The time an expression stands for, `None` for one naming an unknown time or clock
/// (`TimeTickBuilder.parseTimeTick`).
fn parse_time_tick(
    names: &TimeNames,
    arg: &RegexResult,
    diagram: &TimingDiagram,
) -> Option<TimeTick> {
    if let Some(code) = arg.get(names.code, 0) {
        let result = diagram.get_code_value(code)?;
        return match arg.get(names.code, 1) {
            None => Some(result.clone()),
            Some(delta) => Some(TimeTick::new(
                result.time.add(BigDecimal::parse(delta)?),
                TimingFormat::Decimal,
            )),
        };
    }
    if let Some(clock_name) = arg.get(names.clock, 0) {
        return diagram.get_clock_value(clock_name, integer(arg, names.clock, 1));
    }
    if arg.get(names.hour, 0).is_some() {
        let [h, m, s] = [0, 1, 2].map(|index| integer(arg, names.hour, index));
        return Some(TimeTick::new(
            BigDecimal::from_long(3600 * h + 60 * m + s),
            TimingFormat::Hour,
        ));
    }
    if arg.get(names.date, 0).is_some() {
        let [year, month, day] = [0, 1, 2].map(|index| integer(arg, names.date, index));
        return Some(TimingFormat::create_date(
            year,
            month,
            day,
            diagram.get_timing_format_date(),
        ));
    }
    let Some(number) = arg.get(names.digit, 1) else {
        return diagram.now.clone();
    };
    let mut value = BigDecimal::parse(number)?;
    if arg.get(names.digit, 0) == Some("+")
        && let Some(now) = &diagram.now
    {
        value = now.time.add(value);
    }
    Some(TimeTick::new(value, TimingFormat::Decimal))
}

fn parse_time_tick_or_fail(
    names: &TimeNames,
    arg: &RegexResult,
    diagram: &TimingDiagram,
) -> Result<TimeTick, CommandError> {
    parse_time_tick(names, arg, diagram).ok_or_else(|| CommandError::new("What time?"))
}

/// The stereotype written before `as`, else the one after the code.
fn stereotype(arg: &RegexResult) -> Option<Stereotype> {
    arg.get("STEREOTYPE", 0)
        .or_else(|| arg.get("STEREOTYPE2", 0))
        .map(Stereotype::new)
}

/// PlantUML's `CommandRobustConcise`.
fn robust_concise() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "TYPE", r"(robust|concise|rectangle)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
                stereo::optional_pattern("STEREOTYPE"),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            stereo::optional_pattern("STEREOTYPE2"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::or(vec![color::optional_pattern("COLOR")]),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let style = match arg.get("TYPE", 0).unwrap_or_default() {
                "robust" => TimingStyle::Robust,
                "concise" => TimingStyle::Concise,
                _ => TimingStyle::Rectangle,
            };
            let colors = cuca_commands::colors(arg, ColorType::Back)?;
            diagram.create_player_state(
                arg.get("CODE", 0).unwrap_or_default(),
                arg.get("FULL", 0).unwrap_or_default(),
                arg.get("COMPACT", 0).is_some(),
                stereotype(arg),
                colors.get(ColorType::Back).cloned(),
                style,
            );
            Ok(())
        },
    )
}

fn optional_decimal(arg: &RegexResult, name: &str) -> BigDecimal {
    arg.get(name, 0)
        .and_then(BigDecimal::parse)
        .unwrap_or(BigDecimal::ZERO)
}

/// PlantUML's `CommandClock`.
fn clock() -> Box<dyn Command<TimingDiagram>> {
    let number = r"([0-9]+(?:\.[0-9]+)?)";
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(0, "TYPE", r"clock"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"with"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"period"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "PERIOD", number),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"pulse"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "PULSE", number),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"offset"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "OFFSET", number),
            ])),
            RegexTree::spaces_zero_or_more(),
            stereo::optional_pattern("STEREO"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let clock = PlayerClock {
                period: optional_decimal(arg, "PERIOD"),
                pulse: optional_decimal(arg, "PULSE"),
                offset: optional_decimal(arg, "OFFSET"),
            };
            diagram.create_player_clock(
                arg.get("CODE", 0).unwrap_or_default(),
                arg.get("FULL", 0).unwrap_or_default(),
                clock,
                arg.get("STEREO", 0).map(Stereotype::new),
            );
            Ok(())
        },
    )
}

/// PlantUML's `CommandAnalog`.
fn analog() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::leaf(r"analog"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::or(vec![RegexTree::leaf(r"between"), RegexTree::leaf(r"from")]),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "START", r"(-?[0-9]*\.?[0-9]+)"),
                RegexTree::spaces_one_or_more(),
                RegexTree::or(vec![RegexTree::leaf(r"and"), RegexTree::leaf(r"to")]),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "END", r"(-?[0-9]*\.?[0-9]+)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            stereo::optional_pattern("STEREOTYPE2"),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let mut analog = PlayerAnalog::default();
            if let (Some(start), Some(end)) = (arg.get("START", 0), arg.get("END", 0)) {
                analog.time_series.set_bounds(start, end);
            }
            diagram.create_player_signal(
                arg.get("CODE", 0).unwrap_or_default(),
                arg.get("FULL", 0).unwrap_or_default(),
                stereotype(arg),
                PlayerKind::Analog(analog),
            );
            Ok(())
        },
    )
}

/// PlantUML's `CommandBinary`.
fn binary() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "COMPACT", r"(compact)"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::leaf(r"binary"),
            RegexTree::spaces_one_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "FULL", r"[%g]([^%g]+)[%g]"),
                stereo::optional_pattern("STEREOTYPE"),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE", r"([%pLN_.@]+)"),
            stereo::optional_pattern("STEREOTYPE2"),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            diagram.create_player_signal(
                arg.get("CODE", 0).unwrap_or_default(),
                arg.get("FULL", 0).unwrap_or_default(),
                stereotype(arg),
                PlayerKind::Binary(PlayerBinary::default()),
            );
            Ok(())
        },
    )
}

fn unknown_player(code: &str) -> CommandError {
    CommandError::new(format!("Unknown {code}"))
}

/// PlantUML's `CommandDefineStateShort`.
fn define_state_short() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"has"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "STATE", r"([-%pLN_.@]+)"),
            RegexTree::named(3, "STATES", r"((,([-%pLN_.@]+))*)"),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or_default();
            let player = diagram
                .player_mut(code)
                .ok_or_else(|| unknown_player(code))?;
            let states = arg.get("STATES", 0).unwrap_or_default();
            for state in std::iter::once(arg.get("STATE", 0).unwrap_or_default())
                .chain(states.split(',').filter(|state| !state.is_empty()))
            {
                player.define_state(state, state);
            }
            Ok(())
        },
    )
}

/// PlantUML's `CommandDefineStateLong`.
fn define_state_long() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"has"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "LABEL", r"[%g]([^%g]+)[%g]"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "STATE", r"([%pLN_.@]+)"),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or_default();
            diagram
                .player_mut(code)
                .ok_or_else(|| unknown_player(code))?
                .define_state(
                    arg.get("STATE", 0).unwrap_or_default(),
                    arg.get("LABEL", 0).unwrap_or_default(),
                );
            Ok(())
        },
    )
}

const PLAYER_CODE: &str = r"([\p{L}_][%pLN_.]*)";
const STATE_CODE: &str = r"([-%pLN_][-%pLN_.]*)";

/// A state, two states at once, or one of the special states (`CommandChangeState.getStateOrHidden`).
fn state_or_hidden() -> RegexTree {
    RegexTree::or(vec![
        RegexTree::named(1, "STATE1", r"[%g]([^%g]*)[%g]"),
        RegexTree::named(1, "STATE2", STATE_CODE),
        RegexTree::named(1, "STATE3", r"(\{hidden\})"),
        RegexTree::named(1, "STATE4", r"(\{\.\.\.\})"),
        RegexTree::named(1, "STATE5", r"(\{-\})"),
        RegexTree::named(1, "STATE6", r"(\{\?\})"),
        RegexTree::named(
            2,
            "STATE7",
            r"(?:\{([-%pLN_][-%pLN_.]*),([-%pLN_][-%pLN_.]*)\})",
        ),
    ])
}

/// The state, its colour and its comment, after `is`.
fn change_state_tail() -> Vec<RegexTree> {
    vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"is"),
        RegexTree::spaces_zero_or_more(),
        state_or_hidden(),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "COMMENT", r"(.*?)"),
        ])),
        RegexTree::spaces_zero_or_more(),
        RegexTree::end(),
    ]
}

/// `CommandChangeState.addState`.
fn add_state(
    diagram: &mut TimingDiagram,
    arg: &RegexResult,
    player: usize,
    now: Option<TimeTick>,
) -> CommandResult {
    let colors: Colors = cuca_commands::colors(arg, ColorType::Back)?;
    let states = match (arg.get("STATE7", 0), arg.get("STATE7", 1)) {
        (Some(state1), Some(state2)) => vec![state1.to_owned(), state2.to_owned()],
        _ => vec![arg.get_lazzy("STATE", 0).unwrap_or_default().to_owned()],
    };
    diagram.players[player].1.set_state(
        now,
        arg.get("COMMENT", 0).map(str::to_owned),
        colors,
        states,
    );
    Ok(())
}

/// PlantUML's `CommandChangeStateByPlayerCode`: `WU is Idle`.
fn change_state_by_player_code() -> Box<dyn Command<TimingDiagram>> {
    let mut pattern = vec![RegexTree::start(), RegexTree::named(1, "CODE", PLAYER_CODE)];
    pattern.extend(change_state_tail());
    command(RegexTree::concat(pattern), |diagram, arg| {
        let code = arg.get("CODE", 0).unwrap_or_default();
        let player = diagram
            .player_index(code)
            .ok_or_else(|| CommandError::new(format!("Unknown \"{code}\"")))?;
        add_state(diagram, arg, player, diagram.now.clone())
    })
}

/// PlantUML's `CommandChangeStateByTime`: `+100 is Idle` after `@WU`.
fn change_state_by_time() -> Box<dyn Command<TimingDiagram>> {
    let mut pattern = vec![
        RegexTree::start(),
        RegexTree::spaces_zero_or_more(),
        expression_at_without_arobase(&TIME),
    ];
    pattern.extend(change_state_tail());
    command(RegexTree::concat(pattern), |diagram, arg| {
        let player = diagram
            .last_player
            .ok_or_else(|| CommandError::new("Missing @ line before this"))?;
        let tick = parse_time_tick_or_fail(&TIME, arg, diagram)?;
        diagram.add_time(tick.clone(), None);
        add_state(diagram, arg, player, Some(tick))
    })
}

/// PlantUML's `CommandAtTime`: `@100`, `@100 as :start`.
fn at_time() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            expression_at_with_arobase(&TIME),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r":"),
                RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let tick = parse_time_tick_or_fail(&TIME, arg, diagram)?;
            diagram.add_time(tick, arg.get("CODE", 0));
            Ok(())
        },
    )
}

fn no_such_participant(code: &str) -> CommandError {
    CommandError::new(format!("No such participant {code}"))
}

/// PlantUML's `CommandAtPlayer`: `@WU`.
fn at_player() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"@"),
            RegexTree::named(1, "PLAYER", PLAYER_CODE),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or_default();
            diagram.last_player = Some(
                diagram
                    .player_index(code)
                    .ok_or_else(|| no_such_participant(code))?,
            );
            Ok(())
        },
    )
}

/// PlantUML's `CommandTimeMessage`: `WU -> WB@+50 : URL`.
fn time_message() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PART1", PLAYER_CODE),
            RegexTree::optional(expression_at_with_arobase(&TIME1)),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "ARROW_BODY", r"(-+)"),
            RegexTree::named(1, "ARROW_STYLE", arrow_style()),
            RegexTree::named(0, "ARROW_HEAD", r"\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "PART2", PLAYER_CODE),
            RegexTree::optional(expression_at_with_arobase(&TIME2)),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "MESSAGE", r"(.*)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let player = |name: &str| {
                let code = arg.get(name, 0).unwrap_or_default();
                diagram
                    .player_index(code)
                    .ok_or_else(|| CommandError::new(format!("No such element: {code}")))
            };
            let (player1, player2) = (player("PART1")?, player("PART2")?);
            let tick1 = parse_time_tick(&TIME1, arg, diagram);
            let tick2 = parse_time_tick(&TIME2, arg, diagram);
            diagram.create_time_message(
                player1,
                tick1,
                player2,
                tick2,
                arg.get("MESSAGE", 0),
                arg.get_lazzy("ARROW_STYLE", 0),
            );
            Ok(())
        },
    )
}

fn note_position(arg: &RegexResult) -> NotePosition {
    if arg.get("POSITION", 0) == Some("top") {
        NotePosition::Top
    } else {
        NotePosition::Bottom
    }
}

fn add_note(diagram: &mut TimingDiagram, arg: &RegexResult, note: Display) -> CommandResult {
    let code = arg.get("CODE", 0).unwrap_or_default();
    let now = diagram.now.clone();
    let stereotype = arg.get("STEREO", 0).map(Stereotype::new);
    let skin = diagram.titled.skin.clone();
    diagram
        .player_mut(code)
        .ok_or_else(|| CommandError::new(format!("Unknown \"{code}\"")))?
        .add_note(now, note, note_position(arg), stereotype.as_ref(), &skin);
    Ok(())
}

/// PlantUML's `CommandNote`: `note top of WU : text`.
fn note() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "POSITION", r"(top|bottom)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"of"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", PLAYER_CODE),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS"),
            stereo::optional_pattern("STEREO"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NOTE", r"(.+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let note = Display::with_newlines(arg.get("NOTE", 0).unwrap_or_default());
            add_note(diagram, arg, note)
        },
    )
}

fn note_long_start() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"note"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "POSITION", r"(top|bottom)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf(r"of"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "CODE", PLAYER_CODE),
        RegexTree::spaces_zero_or_more(),
        stereo::optional_pattern("STEREO"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandNoteLong`: `note top of WU` up to `end note`.
fn note_long() -> Box<dyn Command<TimingDiagram>> {
    let first_line = note_long_start();
    Box::new(
        Multiline::starting_with_owned(
            note_long_start(),
            &plantuml_regex(r"^end[%s]?note$"),
            move |diagram: &mut TimingDiagram, lines: &BlocLines| {
                let first = lines.first().expect("a block has its first line").trimmed();
                let arg = first_line
                    .matcher(first.text())
                    .expect("the block's first line matched");
                let note = lines.sub_extract(1, 1).without_empty_columns().to_display();
                add_note(diagram, &arg, note)
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandConstraint`: `WB@0 <-> @50 : {50 ms lag}`.
fn constraint() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(2, "PART1", r"(([\p{L}_][%pLN_.]*))?"),
            expression_at_with_arobase(&TIME1),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\<"),
            RegexTree::counted(1, r"(-+)"),
            RegexTree::named(1, "ARROW_STYLE1", arrow_style()),
            RegexTree::counted(1, r"(-*)"),
            RegexTree::leaf(r"\>"),
            RegexTree::spaces_zero_or_more(),
            expression_at_with_arobase(&TIME2),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "MESSAGE", r"(.*)"),
            ])),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let player = match arg.get("PART1", 0) {
                None => diagram
                    .last_player
                    .ok_or_else(|| CommandError::new("You have to provide a participant"))?,
                Some(code) => diagram
                    .player_index(code)
                    .ok_or_else(|| no_such_participant(code))?,
            };
            let unknown_time = || CommandError::new("Unknown time label");
            let tick1 = parse_time_tick(&TIME1, arg, diagram).ok_or_else(unknown_time)?;
            let restore = diagram.now.replace(tick1.clone());
            let tick2 = parse_time_tick(&TIME2, arg, diagram);
            diagram.now = restore;
            let tick2 = tick2.ok_or_else(unknown_time)?;
            let color = constraint_color(arg.get_lazzy("ARROW_STYLE", 0))?;
            diagram.players[player]
                .1
                .create_constraint(tick1, tick2, arg.get("MESSAGE", 0), color);
            Ok(())
        },
    )
}

/// PlantUML's `CommandScalePixel`: `scale 100 as 50 pixels`.
fn scale_pixel() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"scale"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "TICK", r"(\d+)"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::optional(RegexTree::leaf(r"[%s]")),
                RegexTree::named(1, "UNIT", r"([smhdDyY])"),
            ])),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "PIXEL", r"(\d+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"pixels?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let unit_factor = match arg.get("UNIT", 0) {
                Some("m") => 60,
                Some("h") => 3600,
                Some("d" | "D") => 3600 * 24,
                Some("y" | "Y") => 3600 * 8766,
                _ => 1,
            };
            let tick = integer(arg, "TICK", 0) * unit_factor;
            let pixel = integer(arg, "PIXEL", 0);
            if tick <= 0 || pixel <= 0 {
                return Err(CommandError::new("Bad value"));
            }
            diagram.ruler.scale_in_pixels(tick, pixel);
            Ok(())
        },
    )
}

/// PlantUML's `CommandHideTimeAxis`: `hide time-axis` or `manual time-axis`.
fn hide_time_axis() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "COMMAND", r"(hide|manual)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"time"),
            RegexTree::leaf(r".?"),
            RegexTree::leaf(r"axis"),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            diagram.time_axis_stategy = if arg
                .get("COMMAND", 0)
                .is_some_and(|command| command.eq_ignore_ascii_case("manual"))
            {
                TimeAxisStategy::Manual
            } else {
                TimeAxisStategy::Hidden
            };
            Ok(())
        },
    )
}

/// PlantUML's `CommandHighlight`: `highlight 200 to 450 #Gold : caption`.
fn highlight() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"highlight"),
            RegexTree::spaces_one_or_more(),
            expression_at_without_arobase(&FROM),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"to"),
            RegexTree::spaces_one_or_more(),
            expression_at_without_arobase(&TO),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "CAPTION", r"(.*)"),
            ])),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let tick_from = parse_time_tick_or_fail(&FROM, arg, diagram)?;
            let tick_to = parse_time_tick_or_fail(&TO, arg, diagram)?;
            let caption = Display::with_newlines(arg.get("CAPTION", 0).unwrap_or_default());
            let colors = cuca_commands::colors(arg, ColorType::Back)?;
            diagram.highlight(tick_from, tick_to, caption, colors);
            Ok(())
        },
    )
}

/// PlantUML's `CommandModeCompact`.
fn mode_compact() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"mode"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"compact"),
            RegexTree::end(),
        ]),
        |diagram, _| {
            diagram.compact_by_default = true;
            Ok(())
        },
    )
}

/// PlantUML's `CommandTicks`: `A ticks num on multiple 3`, which only analog players use.
fn ticks() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"ticks"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::leaf(r"every"),
                RegexTree::concat(vec![
                    RegexTree::leaf(r"num"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"on"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"multiple"),
                ]),
            ]),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NUM", r"([0-9]+)"),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or_default();
            let ticks_every = arg.get("NUM", 0).and_then(|num| num.parse().ok());
            let player = diagram
                .player_mut(code)
                .ok_or_else(|| no_such_participant(code))?;
            if let PlayerKind::Analog(analog) = &mut player.kind {
                analog.ticks_every = ticks_every;
            }
            Ok(())
        },
    )
}

/// PlantUML's `CommandPixelHeight`: `A is 200 pixels height`.
fn pixel_height() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "PLAYER", r"([%pLN_.@]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"is"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "NUM", r"([0-9]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"pixels?"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"height"),
            RegexTree::end(),
        ]),
        |diagram, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or_default();
            let height = arg
                .get("NUM", 0)
                .and_then(|num| num.parse().ok())
                .unwrap_or_default();
            diagram
                .player_mut(code)
                .ok_or_else(|| no_such_participant(code))?
                .suggested_height = height;
            Ok(())
        },
    )
}

/// PlantUML's `CommandUseDateFormat`: `use date format "YY-MM-DD"`.
fn use_date_format() -> Box<dyn Command<TimingDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"use"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"date"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"format"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "FORMAT", r"[%g]([^%g]+)[%g]"),
            RegexTree::end(),
        ]),
        |diagram, arg| diagram.use_date_format(arg.get("FORMAT", 0).unwrap_or_default()),
    )
}

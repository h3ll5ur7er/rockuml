//! The sentences of gantt diagrams, like `[Task] starts at [Other]'s end and lasts 5 days`: a subject, then
//! verb phrases joined by `and` (PlantUML's `gantt.lang` and `gantt.ulang` packages and
//! `NaturalGanttCommand`).

use super::GanttDiagram;
use super::model::{
    CenterBorderColor, GanttConstraint, GanttConstraintMode, Moment, TaskAttribute, TaskCode,
    TaskId, TaskInstant,
};
use super::ngm::SolverValue;
use super::time::{
    TimePoint, day_of_week_from_string, day_of_week_ubrex, month_from_string, month_ubrex,
};
use crate::color::HColor;
use crate::command::{BlocLines, Command, CommandControl, CommandError, CommandResult};
use crate::decoration::WithLinkType;
use crate::klimt::url::Url;
use crate::local_date::{DayOfWeek, LocalDate, LocalDateTime};
use crate::stereo::Stereotype;
use crate::ubrex::builder::UBrexPart;
use crate::ubrex::{UMatcher, UnicodeBracketedExpression};

/// What the subject of a sentence stands for.
#[derive(Clone)]
pub(super) enum Subject {
    Gantt,
    Today,
    Task(TaskId),
    Resource(String),
    Days(DaysAsDates),
    DayOfWeek(DayOfWeek),
    Date(LocalDate),
}

/// What a complement stands for.
enum Value {
    Date(LocalDate),
    /// Seconds of work.
    Load(i64),
    TimePoint(TimePoint),
    Instant(TaskInstant),
    Colors(CenterBorderColor),
    ColorsFromTo(CenterBorderColor, CenterBorderColor),
    InstantWithLink(TaskInstant, CenterBorderColor),
    Days(DaysAsDates),
    DayOfWeek(DayOfWeek),
    Name(String),
    Completion(i32),
    Url(Url),
    Task(TaskId),
    TwoNames(String, String),
    Nothing,
}

/// The days from one date to another, both included (`DaysAsDates`).
#[derive(Clone, Copy)]
pub(super) struct DaysAsDates {
    date1: LocalDate,
    date2: LocalDate,
}

impl DaysAsDates {
    /// `count` open days from `date1`; the day after the last of them ends the run, as in PlantUML.
    fn counting_open_days(gantt: &GanttDiagram, date1: LocalDate, mut count: i64) -> Self {
        let mut day = date1;
        while count > 0 {
            if gantt.model.calendar.is_open(day) {
                count -= 1;
            }
            day = day.plus_days(1);
        }
        Self { date1, date2: day }
    }

    pub(super) fn days(self) -> impl Iterator<Item = LocalDate> {
        std::iter::successors(Some(self.date1), |day| Some(day.plus_days(1)))
            .take_while(move |day| *day <= self.date2)
    }
}

type Failable<T> = Result<T, String>;

fn leaf(definition: &str) -> UBrexPart {
    UBrexPart::leaf(definition)
}

fn named(name: &str, part: UBrexPart) -> UBrexPart {
    UBrexPart::named(name, part)
}

fn concat(parts: Vec<UBrexPart>) -> UBrexPart {
    UBrexPart::concat(parts)
}

fn spaces() -> UBrexPart {
    UBrexPart::space_one_or_more()
}

/// Any of the words, each followed by spaces, any number of times (`Words.uzeroOrMore`).
fn zero_or_more_words(words: &[&str]) -> UBrexPart {
    UBrexPart::zero_or_more(concat(vec![
        UBrexPart::or(words.iter().map(|word| leaf(word)).collect()),
        spaces(),
    ]))
}

/// Spaces before each word (`Words.uexactly`).
fn exactly(words: &[&str]) -> UBrexPart {
    concat(
        words
            .iter()
            .flat_map(|word| [spaces(), leaf(word)])
            .collect(),
    )
}

/// Spaces between the words (`Words.uexactly2`).
fn exactly2(words: &[&str]) -> UBrexPart {
    let mut parts = Vec::new();
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            parts.push(spaces());
        }
        parts.push(leaf(word));
    }
    concat(parts)
}

/// `[code]`, the code captured under `name` (`SubjectTask.taskCode`).
fn task_code(name: &str) -> UBrexPart {
    concat(vec![leaf("["), named(name, leaf("〇+「〤[]」")), leaf("]")])
}

fn digits1to2() -> UBrexPart {
    leaf("〇{1-2}〴d")
}

fn digits1to4() -> UBrexPart {
    leaf("〇{1-4}〴d")
}

/// The three ways of writing a day, its parts captured under names ending with `id` (`DayPattern`).
struct DayPattern {
    id: &'static str,
}

impl DayPattern {
    fn key(&self, kind: &str) -> String {
        format!("{kind}{}", self.id)
    }

    fn to_ubrex(&self) -> UBrexPart {
        UBrexPart::or(vec![
            // 20th of september 2020
            concat(vec![
                named(&self.key("ADAY"), digits1to2()),
                UBrexPart::upto(leaf("〴D"), named(&self.key("AMONTH"), month_ubrex())),
                leaf("〇+〴D"),
                named(&self.key("AYEAR"), digits1to4()),
            ]),
            // 2020-09-20
            ymd_ubrex(&self.key("BYEAR"), &self.key("BMONTH"), &self.key("BDAY")),
            // september 20 2020
            concat(vec![
                named(&self.key("CMONTH"), month_ubrex()),
                leaf("〇+〴D"),
                named(&self.key("CDAY"), digits1to2()),
                leaf("〇+〴D"),
                named(&self.key("CYEAR"), digits1to4()),
            ]),
        ])
    }

    fn get_day(&self, arg: &UMatcher) -> Option<Failable<LocalDate>> {
        let value = |kind: &str| arg.find_first_value_by_key(&self.key(kind));
        let number = |kind: &str| value(kind).and_then(|v| v.parse::<i64>().ok());
        let month_name = |kind: &str| value(kind).and_then(month_from_string);
        let (year, month, day) = if value("ADAY").is_some() {
            (number("AYEAR"), month_name("AMONTH"), number("ADAY"))
        } else if value("BDAY").is_some() {
            (number("BYEAR"), number("BMONTH"), number("BDAY"))
        } else if value("CDAY").is_some() {
            (number("CYEAR"), month_name("CMONTH"), number("CDAY"))
        } else {
            return None;
        };
        Some(date_of(year, month, day))
    }
}

fn date_of(year: Option<i64>, month: Option<i64>, day: Option<i64>) -> Failable<LocalDate> {
    year.zip(month)
        .zip(day)
        .and_then(|((year, month), day)| LocalDate::of(year, month, day))
        .ok_or_else(|| "Invalid date".to_owned())
}

/// `2020-09-20` (`TimeResolution.toUbrexB_YYYY_MM_DD`).
fn ymd_ubrex(year: &str, month: &str, day: &str) -> UBrexPart {
    concat(vec![
        named(year, digits1to4()),
        leaf("〇+〴D"),
        named(month, digits1to2()),
        leaf("〇+〴D"),
        named(day, digits1to2()),
    ])
}

fn number(arg: &UMatcher, key: &str) -> Option<i64> {
    arg.find_first_value_by_key(key)
        .and_then(|value| value.parse().ok())
}

fn color_or_white(name: Option<&str>) -> Option<HColor> {
    name.map(HColor::parse_or_white)
}

/// The kinds of complement a verb takes (`Something` and its implementations).
#[derive(Clone, Copy)]
enum Complement {
    /// `ComplementDate.any`, `onlyRelative` and `onlyAbsolute`.
    Date {
        absolute: bool,
        relative: bool,
    },
    Duration,
    TimePoint,
    BeforeOrAfterOrAtTaskStartOrEnd,
    InColors,
    InColors2,
    InColorsFromTo,
    WithColorLink,
    Intervals,
    IntervalsSmart,
    DayOfWeek,
    Named,
    Close,
    Open,
    Completed,
    Deleted,
    Url,
    Anything,
    Task,
    FromTo,
    WorkingHours,
    /// `PairOfSomething` of a task instant and a coloured link.
    InstantWithColorLink,
}

const ANY_DATE: Complement = Complement::Date {
    absolute: true,
    relative: true,
};
const RELATIVE_DATE: Complement = Complement::Date {
    absolute: false,
    relative: true,
};
const ABSOLUTE_DATE: Complement = Complement::Date {
    absolute: true,
    relative: false,
};

/// `3 days before`, `2 working weeks and 1 day after` (`ComplementBeforeOrAfterOrAtTaskStartOrEnd`).
fn duration_before_or_after() -> UBrexPart {
    let amount = |i: &str| {
        concat(vec![
            named(&format!("COMPLEMENT_NB{i}"), leaf("〇+〴d")),
            spaces(),
            UBrexPart::optional(concat(vec![
                named(&format!("COMPLEMENT_WORKING{i}"), leaf("working")),
                spaces(),
            ])),
            named(&format!("COMPLEMENT_DAY_OR_WEEK{i}"), leaf("【day┇week】")),
            leaf("〇?s"),
        ])
    };
    concat(vec![
        amount("1"),
        UBrexPart::optional(concat(vec![spaces(), leaf("and"), spaces(), amount("2")])),
        spaces(),
        named(
            "COMPLEMENT_BEFORE_OR_AFTER",
            UBrexPart::or(vec![leaf("before"), leaf("after")]),
        ),
    ])
}

/// `[Task]'s start` or `[Task]'s end`.
fn task_start_or_end() -> UBrexPart {
    concat(vec![
        task_code("COMPLEMENT_CODE_OTHER"),
        leaf("〴.s"),
        spaces(),
        named(
            "COMPLEMENT_START_OR_END",
            UBrexPart::or(vec![leaf("start"), leaf("end")]),
        ),
    ])
}

fn colour_word() -> UBrexPart {
    leaf("〇?# 〇+〴w")
}

fn colours(name1: &str, name2: &str) -> UBrexPart {
    concat(vec![
        named(name1, colour_word()),
        UBrexPart::optional(concat(vec![leaf("/"), named(name2, colour_word())])),
    ])
}

/// A date, `5 days after start` or `D+5`, as the complement allows (`ComplementDate`).
fn date_ubrex(absolute: bool, relative: bool) -> UBrexPart {
    let mut parts = Vec::new();
    if absolute {
        parts.push(DayPattern { id: "" }.to_ubrex());
    }
    if relative {
        parts.push(concat(vec![
            named("DCOUNT", leaf("〇+〴d")),
            spaces(),
            leaf("day〇?s"),
            spaces(),
            leaf("after"),
            spaces(),
            leaf("start"),
        ]));
        parts.push(concat(vec![
            leaf("「dD」+"),
            named("ECOUNT", leaf("〇+〴d")),
        ]));
    }
    if parts.len() == 1 {
        parts.pop().expect("one part")
    } else {
        UBrexPart::or(parts)
    }
}

/// `2 weeks, 3 days and 4 hours` (`ComplementDuration`).
fn duration_ubrex() -> UBrexPart {
    let element = |prefix: &str| {
        concat(vec![
            named(&format!("CNUM{prefix}"), leaf("〇+〴d")),
            spaces(),
            named(
                &format!("CUNIT{prefix}"),
                leaf("【hour┇minute┇second┇day┇week┇month】"),
            ),
            leaf("〇?s"),
        ])
    };
    concat(vec![
        element("0"),
        UBrexPart::zero_or_more(concat(vec![
            leaf("【〇+「∙,」and〇+「∙,」┇〇+「∙,」】"),
            element("1"),
        ])),
    ])
}

/// `with red dashed link` (`ComplementWithColorLink`).
fn with_color_link_ubrex() -> UBrexPart {
    let optional_style = || {
        UBrexPart::optional(concat(vec![
            named(
                "STYLE",
                UBrexPart::or(vec![leaf("dotted"), leaf("bold"), leaf("dashed")]),
            ),
            spaces(),
        ]))
    };
    concat(vec![
        leaf("with"),
        spaces(),
        optional_style(),
        named("COLOR", colour_word()),
        spaces(),
        optional_style(),
        leaf("link"),
    ])
}

/// `2020-07-13 to 2020-07-16` or `D+3 to D+5` (`ComplementIntervals`).
fn intervals_ubrex() -> UBrexPart {
    let interval_tail = || vec![exactly(&["to"]), zero_or_more_words(&["the"]), spaces()];
    let mut by_date = vec![DayPattern { id: "1" }.to_ubrex()];
    by_date.extend(interval_tail());
    by_date.push(DayPattern { id: "2" }.to_ubrex());
    let mut by_count = vec![leaf("「dD」+"), named("ECOUNT1", leaf("〇+〴d"))];
    by_count.extend(interval_tail());
    by_count.push(leaf("「dD」+"));
    by_count.push(named("ECOUNT2", leaf("〇+〴d")));
    UBrexPart::or(vec![concat(by_date), concat(by_count)])
}

impl Complement {
    fn to_ubrex(self) -> UBrexPart {
        match self {
            Self::Date { absolute, relative } => date_ubrex(absolute, relative),
            Self::Duration => duration_ubrex(),
            Self::TimePoint => concat(vec![
                DayPattern { id: "" }.to_ubrex(),
                leaf("「Tt∙」"),
                named("HOUR", digits1to2()),
                leaf(":"),
                named("MINUTE", digits1to2()),
                UBrexPart::optional(concat(vec![leaf(":"), named("SECOND", digits1to2())])),
            ]),
            Self::BeforeOrAfterOrAtTaskStartOrEnd => concat(vec![
                UBrexPart::optional(UBrexPart::or(vec![
                    leaf("at"),
                    leaf("with"),
                    leaf("after"),
                    duration_before_or_after(),
                ])),
                UBrexPart::space_zero_or_more(),
                task_start_or_end(),
            ]),
            Self::InColors => concat(vec![
                leaf("in"),
                spaces(),
                colours("COMPLEMENT1", "COMPLEMENT2"),
            ]),
            Self::InColors2 => concat(vec![
                leaf("colo〇?ured"),
                spaces(),
                UBrexPart::optional(leaf("in〇+〴s")),
                named("COMPLEMENT1", leaf("〇?#〇+〴w")),
                UBrexPart::optional(concat(vec![
                    leaf("/"),
                    named("COMPLEMENT2", leaf("〇?#〇+〴w")),
                ])),
            ]),
            Self::InColorsFromTo => concat(vec![
                leaf("from"),
                spaces(),
                colours("FROM1", "FROM2"),
                spaces(),
                leaf("to"),
                spaces(),
                colours("TO1", "TO2"),
            ]),
            Self::WithColorLink => with_color_link_ubrex(),
            Self::InstantWithColorLink => concat(vec![
                Self::BeforeOrAfterOrAtTaskStartOrEnd.to_ubrex(),
                spaces(),
                Self::WithColorLink.to_ubrex(),
            ]),
            Self::Intervals => intervals_ubrex(),
            Self::IntervalsSmart => concat(vec![
                DayPattern { id: "1" }.to_ubrex(),
                exactly(&["to"]),
                UBrexPart::space_zero_or_more(),
                UBrexPart::optional(duration_before_or_after()),
                UBrexPart::space_zero_or_more(),
                task_start_or_end(),
            ]),
            Self::DayOfWeek => named("COMPLEMENT", day_of_week_ubrex()),
            Self::Named => concat(vec![
                leaf("["),
                named("COMPLEMENT", leaf("〇+「〤[]」")),
                leaf("]"),
            ]),
            Self::Close => named(
                "CLOSED",
                concat(vec![
                    leaf("close〇?d"),
                    UBrexPart::optional(concat(vec![leaf("∙for∙"), task_code("FOO")])),
                ]),
            ),
            Self::Open => named(
                "OPEN",
                concat(vec![
                    leaf("open〇?e〇?d"),
                    UBrexPart::optional(concat(vec![leaf("∙for∙"), task_code("FOO")])),
                ]),
            ),
            Self::Completed => concat(vec![
                named("COMPLEMENT", leaf("〇+〴d")),
                leaf("〇+〴W complete〇?d"),
            ]),
            Self::Deleted => leaf("deleted"),
            Self::Url => named("COMPLEMENT", leaf("[[〄>]]")),
            Self::Anything => named("ANYTHING", leaf("〇*〴.")),
            Self::Task => named("COMPLEMENT", task_code("FOO")),
            Self::FromTo => concat(vec![
                leaf("from"),
                spaces(),
                task_code("COMPLEMENT1"),
                spaces(),
                leaf("to"),
                spaces(),
                task_code("COMPLEMENT2"),
            ]),
            Self::WorkingHours => concat(vec![leaf("working"), spaces(), leaf("hours")]),
        }
    }

    fn get_me(self, gantt: &GanttDiagram, arg: &UMatcher) -> Failable<Value> {
        let value = |key: &str| arg.find_first_value_by_key(key);
        match self {
            Self::Date { .. } => complement_date(gantt, arg).map(Value::Date),
            Self::Duration => Ok(Value::Load(complement_duration(gantt, arg))),
            Self::TimePoint => complement_time_point(arg).map(Value::TimePoint),
            Self::BeforeOrAfterOrAtTaskStartOrEnd => {
                complement_task_instant(gantt, arg).map(Value::Instant)
            }
            Self::InColors | Self::InColors2 => Ok(Value::Colors(CenterBorderColor::new(
                color_or_white(value("COMPLEMENT1")),
                color_or_white(value("COMPLEMENT2")),
            ))),
            Self::InColorsFromTo => Ok(Value::ColorsFromTo(
                CenterBorderColor::new(
                    color_or_white(value("FROM1")),
                    color_or_white(value("FROM2")),
                ),
                CenterBorderColor::new(color_or_white(value("TO1")), color_or_white(value("TO2"))),
            )),
            Self::WithColorLink => Ok(Value::Colors(with_color_link(arg))),
            Self::InstantWithColorLink => Ok(Value::InstantWithLink(
                complement_task_instant(gantt, arg)?,
                with_color_link(arg),
            )),
            Self::Intervals => complement_intervals(gantt, arg).map(Value::Days),
            Self::IntervalsSmart => complement_intervals_smart(gantt, arg).map(Value::Days),
            Self::DayOfWeek => Ok(Value::DayOfWeek(
                value("COMPLEMENT")
                    .and_then(day_of_week_from_string)
                    .expect("the pattern matched a day of the week"),
            )),
            Self::Named | Self::Anything => Ok(Value::Name(
                value(if matches!(self, Self::Named) {
                    "COMPLEMENT"
                } else {
                    "ANYTHING"
                })
                .unwrap_or_default()
                .to_owned(),
            )),
            Self::Close | Self::Open => {
                let text = value(if matches!(self, Self::Close) {
                    "CLOSED"
                } else {
                    "OPEN"
                })
                .unwrap_or_default();
                Ok(Value::Name(task_of_close_or_open(text)))
            }
            Self::Completed => Ok(Value::Completion(
                number(arg, "COMPLEMENT").unwrap_or_default() as i32,
            )),
            Self::Deleted | Self::WorkingHours => Ok(Value::Nothing),
            Self::Url => Url::parse(value("COMPLEMENT").unwrap_or_default())
                .map(Value::Url)
                .ok_or_else(|| "Bad URL".to_owned()),
            Self::Task => {
                let code = value("COMPLEMENT").unwrap_or_default();
                gantt
                    .existing_task(code)
                    .map(Value::Task)
                    .ok_or_else(|| format!("No such task {code}"))
            }
            Self::FromTo => Ok(Value::TwoNames(
                value("COMPLEMENT1").unwrap_or_default().to_owned(),
                value("COMPLEMENT2").unwrap_or_default().to_owned(),
            )),
        }
    }
}

/// `2020-07-01T10:30` (`ComplementTimePoint`).
fn complement_time_point(arg: &UMatcher) -> Failable<TimePoint> {
    let day = DayPattern { id: "" }
        .get_day(arg)
        .expect("the pattern matched a day")?;
    let time = LocalDateTime::of(
        day,
        number(arg, "HOUR").unwrap_or_default(),
        number(arg, "MINUTE").unwrap_or_default(),
        number(arg, "SECOND").unwrap_or_default(),
    );
    Ok(TimePoint::of(time))
}

/// `ComplementIntervals.getMe`.
fn complement_intervals(gantt: &GanttDiagram, arg: &UMatcher) -> Failable<DaysAsDates> {
    if let Some(d1) = (DayPattern { id: "1" }).get_day(arg) {
        let d2 = (DayPattern { id: "2" })
            .get_day(arg)
            .expect("both ends of an interval match")?;
        return Ok(DaysAsDates {
            date1: d1?,
            date2: d2,
        });
    }
    let min = gantt.min_time_point();
    let day = |key| min.add_days(number(arg, key).unwrap_or_default()).to_day();
    Ok(DaysAsDates {
        date1: day("ECOUNT1"),
        date2: day("ECOUNT2"),
    })
}

/// From a date to a task's start or end, the end itself excluded (`ComplementIntervalsSmart`).
fn complement_intervals_smart(gantt: &GanttDiagram, arg: &UMatcher) -> Failable<DaysAsDates> {
    let d1 = (DayPattern { id: "1" })
        .get_day(arg)
        .expect("the interval starts with a day")?;
    let end = complement_task_instant(gantt, arg)?;
    let mut precise = gantt.model.instant_precise(&end);
    if end.attribute == TaskAttribute::End {
        precise = precise.decrement();
    }
    Ok(DaysAsDates {
        date1: d1,
        date2: precise.to_day(),
    })
}

/// `closed for [Task]` gives the task's code, plain `closed` nothing.
fn task_of_close_or_open(text: &str) -> String {
    match (text.find('['), text.rfind(']')) {
        (Some(x), Some(y)) if x > 0 => text[x + 1..y].to_owned(),
        _ => String::new(),
    }
}

fn with_color_link(arg: &UMatcher) -> CenterBorderColor {
    let color = color_or_white(arg.find_first_value_by_key("COLOR"));
    CenterBorderColor {
        center: color.clone(),
        border: color,
        style: arg.find_first_value_by_key("STYLE").map(str::to_owned),
    }
}

/// Any date, as `print between` takes them.
pub(super) fn any_date_ubrex() -> UBrexPart {
    ANY_DATE.to_ubrex()
}

pub(super) fn any_date(gantt: &GanttDiagram, arg: &UMatcher) -> Failable<LocalDate> {
    complement_date(gantt, arg)
}

/// `ComplementDate.getMe`.
fn complement_date(gantt: &GanttDiagram, arg: &UMatcher) -> Failable<LocalDate> {
    if let Some(day) = (DayPattern { id: "" }).get_day(arg) {
        return day;
    }
    let count = number(arg, "DCOUNT")
        .or_else(|| number(arg, "ECOUNT"))
        .expect("a relative date has its days");
    Ok(gantt.min_day().plus_days(count))
}

/// The work a duration like `2 weeks and 3 days` stands for, in seconds (`ComplementDuration`).
fn complement_duration(gantt: &GanttDiagram, arg: &UMatcher) -> i64 {
    let mut totals = [0i64; 4];
    let mut accumulate = |num: &str, unit: &str| {
        let value: i64 = num.parse().unwrap_or_default();
        let unit_lower = unit.to_ascii_lowercase();
        match unit_lower.as_bytes() {
            [b'h', ..] => totals[1] += value,
            [b'd', ..] => totals[0] += value,
            [b'w', ..] => totals[0] += value * gantt.days_in_week(),
            [b's', ..] => totals[3] += value,
            [b'm', b'i', ..] => totals[2] += value,
            _ => totals[0] += value * 30,
        }
    };
    accumulate(
        arg.find_first_value_by_key("CNUM0").unwrap_or_default(),
        arg.find_first_value_by_key("CUNIT0").unwrap_or_default(),
    );
    for (num, unit) in arg
        .find_values_by_key("CNUM1")
        .into_iter()
        .zip(arg.find_values_by_key("CUNIT1"))
    {
        accumulate(num, unit);
    }
    totals[0] * 86_400 + totals[1] * 3600 + totals[2] * 60 + totals[3]
}

/// `3 days after [Task]'s end` (`AbstractComplementTaskInstant.getComplementTaskInstant`).
fn complement_task_instant(gantt: &GanttDiagram, arg: &UMatcher) -> Failable<TaskInstant> {
    let value = |key: &str| arg.find_first_value_by_key(key);
    let code = value("COMPLEMENT_CODE_OTHER").unwrap_or_default();
    let moment = gantt
        .existing_moment(code)
        .ok_or_else(|| format!("No such task {code}"))?;
    let attribute = if value("COMPLEMENT_START_OR_END")
        .is_some_and(|word| word.eq_ignore_ascii_case("start"))
    {
        TaskAttribute::Start
    } else {
        TaskAttribute::End
    };
    let result = TaskInstant::new(moment, attribute);
    let Some(nb1) = number(arg, "COMPLEMENT_NB1") else {
        return Ok(result);
    };
    let factor = |key: &str| {
        if value(key).is_some_and(|unit| unit.starts_with('w')) {
            gantt.days_in_week()
        } else {
            1
        }
    };
    let days1 = nb1 * factor("COMPLEMENT_DAY_OR_WEEK1");
    let days2 =
        number(arg, "COMPLEMENT_NB2").map_or(0, |nb2| nb2 * factor("COMPLEMENT_DAY_OR_WEEK2"));
    let mut delta = days1 + days2;
    if value("COMPLEMENT_BEFORE_OR_AFTER").is_some_and(|word| word.eq_ignore_ascii_case("before")) {
        delta = -delta;
    }
    let working = value("COMPLEMENT_WORKING1").is_some() || value("COMPLEMENT_WORKING2").is_some();
    let mode = if working {
        GanttConstraintMode::DoNotCountCloseDay
    } else {
        GanttConstraintMode::IgnoreCalendar
    };
    Ok(result.with_delta(delta, mode, gantt.model.calendar.default_plan()))
}

/// What a verb phrase does to the diagram.
type Action = fn(&mut GanttDiagram, &Subject, Value) -> CommandResult;

/// A verb, words that may follow it, a complement and what they do (`VerbPhraseAction`).
struct VerbPhraseAction {
    verb: UnicodeBracketedExpression,
    ignored: Option<UnicodeBracketedExpression>,
    complement: Complement,
    complement_ubrex: UnicodeBracketedExpression,
    action: Action,
}

impl VerbPhraseAction {
    fn new(verb: &str, ignored: Option<UBrexPart>, complement: Complement, action: Action) -> Self {
        Self {
            verb: UBrexPart::leaf(verb).build(),
            ignored: ignored.map(UBrexPart::build),
            complement,
            complement_ubrex: complement.to_ubrex().build(),
            action,
        }
    }

    /// The verb and its complement at the start of `text`: how far they read, and the complement's match.
    fn parse<'a>(&self, text: &'a str) -> Option<(usize, UMatcher<'a>)> {
        let mut position = accepted_length(&self.verb, text)?;
        position += skip_spaces(&text[position..]);
        if let Some(ignored) = &self.ignored {
            position += ignored
                .match_at(&text[position..])
                .map_or(0, |matcher| matcher.accepted_match().len());
            position += skip_spaces(&text[position..]);
        }
        let matcher = self.complement_ubrex.match_at(&text[position..])?;
        let length = matcher.accepted_match().len();
        (length > 0).then_some((position + length, matcher))
    }
}

/// How much of `text` the expression accepts, `None` when nothing.
fn accepted_length(expression: &UnicodeBracketedExpression, text: &str) -> Option<usize> {
    let length = expression.match_at(text)?.accepted_match().len();
    (length > 0).then_some(length)
}

fn skip_spaces(text: &str) -> usize {
    text.len() - text.trim_start_matches(crate::java::is_whitespace).len()
}

/// The kinds of subject a sentence starts with (`Subject` and its implementations).
#[derive(Clone, Copy)]
pub(super) enum SubjectKind {
    Gantt,
    Today,
    Task,
    Resource,
    DaysAsDates,
    DayOfWeek,
    DayAsDate,
    Separator,
    WorkingHours,
}

impl SubjectKind {
    pub(super) const ALL: [Self; 9] = [
        Self::Gantt,
        Self::Today,
        Self::Task,
        Self::Resource,
        Self::DaysAsDates,
        Self::DayOfWeek,
        Self::DayAsDate,
        Self::Separator,
        Self::WorkingHours,
    ];

    fn to_ubrex(self) -> UBrexPart {
        match self {
            Self::Gantt => UBrexPart::or(vec![leaf("project"), leaf("gantt")]),
            Self::Today => leaf("today"),
            Self::Task => UBrexPart::or(vec![
                named("IT", leaf("it")),
                concat(vec![
                    UBrexPart::optional(concat(vec![named("THEN", leaf("then")), spaces()])),
                    task_code("SUBJECT"),
                    UBrexPart::optional(concat(vec![
                        UBrexPart::space_zero_or_more(),
                        named("STEREOTYPE", leaf("<< 〇+「〤<>」>>")),
                        UBrexPart::space_zero_or_more(),
                    ])),
                    UBrexPart::optional(concat(vec![
                        exactly(&["as"]),
                        spaces(),
                        task_code("SHORTNAME"),
                    ])),
                    UBrexPart::optional(concat(vec![
                        exactly(&["on"]),
                        spaces(),
                        named(
                            "RESOURCE",
                            UBrexPart::one_or_more(concat(vec![
                                leaf("{〇+「〤{}」}"),
                                UBrexPart::space_zero_or_more(),
                            ])),
                        ),
                    ])),
                ]),
            ]),
            Self::Resource => UBrexPart::or(vec![
                named("THEY", leaf("【she┇he┇they】")),
                concat(vec![
                    leaf("{"),
                    named("RESOURCE", leaf("〇+「〤{}」")),
                    leaf("}"),
                ]),
            ]),
            Self::DaysAsDates => UBrexPart::or(vec![
                concat(vec![
                    ymd_ubrex("BYEAR1", "BMONTH1", "BDAY1"),
                    exactly(&["to"]),
                    spaces(),
                    ymd_ubrex("BYEAR2", "BMONTH2", "BDAY2"),
                ]),
                concat(vec![
                    leaf("「dD」+"),
                    named("ECOUNT1", leaf("〇+〴d")),
                    exactly(&["to"]),
                    spaces(),
                    leaf("「dD」+"),
                    named("ECOUNT2", leaf("〇+〴d")),
                ]),
                concat(vec![
                    ymd_ubrex("BYEAR3", "BMONTH3", "BDAY3"),
                    exactly(&["and"]),
                    spaces(),
                    named("COUNT_AND", leaf("〇+〴d")),
                    spaces(),
                    leaf("day〇?s"),
                ]),
                concat(vec![
                    leaf("then"),
                    spaces(),
                    named("COUNT_THEN", leaf("〇+〴d")),
                    spaces(),
                    leaf("day〇?s"),
                ]),
            ]),
            Self::DayOfWeek => named("SUBJECT", day_of_week_ubrex()),
            Self::DayAsDate => UBrexPart::or(vec![
                ymd_ubrex("BYEAR", "BMONTH", "BDAY"),
                concat(vec![
                    named("ETYPE", leaf("「dDtTeE」")),
                    named("EOPERATION", leaf("「-+」")),
                    named("ECOUNT", leaf("〇+〴d")),
                ]),
            ]),
            Self::Separator => leaf("separator"),
            Self::WorkingHours => concat(vec![
                leaf("from"),
                spaces(),
                named("START", leaf("〇+〴d:〇+〴d")),
                spaces(),
                leaf("to"),
                spaces(),
                named("END", leaf("〇+〴d:〇+〴d")),
            ]),
        }
    }

    /// What the subject stands for, which for a task may create it (`getMe`).
    fn get_me(self, gantt: &mut GanttDiagram, arg: &UMatcher) -> Failable<Subject> {
        let value = |key: &str| arg.find_first_value_by_key(key);
        match self {
            Self::Gantt | Self::Separator | Self::WorkingHours => Ok(Subject::Gantt),
            Self::Today => Ok(Subject::Today),
            Self::Task => task_subject(gantt, arg).map(Subject::Task),
            Self::Resource => {
                if value("THEY").is_some() {
                    return gantt
                        .they
                        .clone()
                        .map(Subject::Resource)
                        .ok_or_else(|| "Not sure who are you refering to?".to_owned());
                }
                let name = value("RESOURCE").unwrap_or_default().to_owned();
                gantt.get_resource(&name);
                gantt.they = Some(name.clone());
                Ok(Subject::Resource(name))
            }
            Self::DaysAsDates => {
                let date = |suffix: &str| -> Failable<LocalDate> {
                    if value(&format!("BDAY{suffix}")).is_some() {
                        return date_of(
                            number(arg, &format!("BYEAR{suffix}")),
                            number(arg, &format!("BMONTH{suffix}")),
                            number(arg, &format!("BDAY{suffix}")),
                        );
                    }
                    let day = number(arg, &format!("ECOUNT{suffix}")).unwrap_or_default();
                    Ok(gantt.min_time_point().add_days(day).to_day())
                };
                if let Some(count) = number(arg, "COUNT_AND") {
                    return Ok(Subject::Days(DaysAsDates::counting_open_days(
                        gantt,
                        date("3")?,
                        count,
                    )));
                }
                if let Some(count) = number(arg, "COUNT_THEN") {
                    let then = gantt.then_date().to_day();
                    return Ok(Subject::Days(DaysAsDates::counting_open_days(
                        gantt, then, count,
                    )));
                }
                Ok(Subject::Days(DaysAsDates {
                    date1: date("1")?,
                    date2: date("2")?,
                }))
            }
            Self::DayOfWeek => Ok(Subject::DayOfWeek(
                value("SUBJECT")
                    .and_then(day_of_week_from_string)
                    .expect("the pattern matched a day of the week"),
            )),
            Self::DayAsDate => {
                if value("BDAY").is_some() {
                    return date_of(
                        number(arg, "BYEAR"),
                        number(arg, "BMONTH"),
                        number(arg, "BDAY"),
                    )
                    .map(Subject::Date);
                }
                let mut day = number(arg, "ECOUNT").unwrap_or_default();
                if value("EOPERATION") == Some("-") {
                    day = -day;
                }
                let base = match value("ETYPE").unwrap_or_default() {
                    "d" | "D" => gantt.min_day(),
                    "t" | "T" => gantt.today_day(),
                    _ => gantt.max_day(),
                };
                Ok(Subject::Date(base.plus_days(day)))
            }
        }
    }

    fn verb_phrases(self) -> Vec<VerbPhraseAction> {
        let action = VerbPhraseAction::new;
        match self {
            Self::Gantt => vec![action(
                "starts",
                Some(zero_or_more_words(&["on", "for", "the", "at"])),
                ABSOLUTE_DATE,
                |gantt, _, value| gantt.update_starting_point(date(&value)),
            )],
            Self::Today => vec![
                action(
                    "is 〇+〴s colo〇?ured",
                    None,
                    Complement::InColors,
                    |gantt, _, value| {
                        gantt.set_today_colors(colors(value));
                        Ok(())
                    },
                ),
                action("is", None, ANY_DATE, |gantt, _, value| {
                    gantt.today = Some(TimePoint::of_start_of_day(date(&value)));
                    Ok(())
                }),
            ],
            Self::Task => task_verb_phrases(),
            Self::Resource => resource_verb_phrases(),
            Self::DaysAsDates => days_verb_phrases(),
            Self::DayOfWeek => day_of_week_verb_phrases(),
            Self::DayAsDate => day_as_date_verb_phrases(),
            Self::Separator => separator_verb_phrases(),
            Self::WorkingHours => vec![action(
                "are",
                None,
                Complement::WorkingHours,
                |gantt, _, _| {
                    gantt.titled.not_ported(crate::diagram::NotYetPorted(
                        "working hours in gantt diagrams",
                    ));
                    Ok(())
                },
            )],
        }
    }
}

/// What days of the week can be told (`SubjectDayOfWeek.getVerbPhrases`).
fn day_of_week_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    vec![
        action("are", None, Complement::Open, |gantt, subject, value| {
            gantt.open_day_of_week(day_of_week(subject), &name(value));
            Ok(())
        }),
        action("are", None, Complement::Close, |gantt, subject, _| {
            gantt
                .model
                .calendar
                .open_close
                .close_day_of_week(day_of_week(subject));
            Ok(())
        }),
        action(
            "【is┇are】",
            None,
            Complement::InColors2,
            |gantt, subject, value| {
                if let Some(color) = colors(value).center {
                    gantt
                        .model
                        .calendar
                        .put_color_day_of_week(day_of_week(subject), color);
                }
                Ok(())
            },
        ),
    ]
}

/// What dates can be told (`SubjectDayAsDate.getVerbPhrases`).
fn day_as_date_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    vec![
        action(
            "【is┇are】",
            None,
            Complement::Open,
            |gantt, subject, value| {
                gantt.open_day_as_date(subject_date(subject), &name(value));
                Ok(())
            },
        ),
        action(
            "【is┇are】",
            None,
            Complement::Close,
            |gantt, subject, value| {
                gantt.close_day_as_date(subject_date(subject), &name(value));
                Ok(())
            },
        ),
        action(
            "【is┇are】",
            None,
            Complement::InColors2,
            |gantt, subject, value| {
                gantt.color_day(subject_date(subject), colors(value).center);
                Ok(())
            },
        ),
    ]
}

/// Where vertical separators go (`SubjectSeparator.getVerbPhrases`).
fn separator_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    vec![
        action("just", Some(leaf("before")), ANY_DATE, |gantt, _, value| {
            gantt.model.calendar.add_separator_before(date(&value));
            Ok(())
        }),
        action("just", Some(leaf("after")), ANY_DATE, |gantt, _, value| {
            gantt
                .model
                .calendar
                .add_separator_before(date(&value).plus_days(1));
            Ok(())
        }),
        action(
            "just",
            None,
            Complement::BeforeOrAfterOrAtTaskStartOrEnd,
            |gantt, _, value| {
                let day = gantt.model.instant_precise(&instant(value)).to_day();
                gantt.model.calendar.add_separator_before(day);
                Ok(())
            },
        ),
    ]
}

/// The task a subject names, created on first use, with its stereotype and people (`SubjectTask.getMe`).
fn task_subject(gantt: &mut GanttDiagram, arg: &UMatcher) -> Failable<TaskId> {
    let value = |key: &str| arg.find_first_value_by_key(key);
    let task = if value("IT").is_some() {
        gantt
            .it
            .ok_or_else(|| "Not sure what are you refering to?".to_owned())?
    } else {
        let code =
            TaskCode::from_id_and_display(value("SHORTNAME"), value("SUBJECT").unwrap_or_default());
        let task = gantt.get_or_create_task(code, value("THEN").is_some());
        if let Some(stereotype) = value("STEREOTYPE") {
            gantt.model.tasks[task].stereotype = Some(Stereotype::new(stereotype));
        }
        gantt.it = Some(task);
        task
    };
    if let Some(resources) = value("RESOURCE") {
        for part in resources
            .split(['{', '}'])
            .map(crate::java::trim)
            .filter(|part| !part.is_empty())
        {
            if !gantt.affect_resource(task, part) {
                return Err("Bad argument for resource".to_owned());
            }
        }
    }
    Ok(task)
}

fn date(value: &Value) -> LocalDate {
    match value {
        Value::Date(date) => *date,
        _ => unreachable!("the complement is a date"),
    }
}

fn colors(value: Value) -> CenterBorderColor {
    match value {
        Value::Colors(colors) => colors,
        _ => unreachable!("the complement is colours"),
    }
}

fn instant(value: Value) -> TaskInstant {
    match value {
        Value::Instant(instant) => instant,
        _ => unreachable!("the complement is a task instant"),
    }
}

fn name(value: Value) -> String {
    match value {
        Value::Name(name) => name,
        _ => unreachable!("the complement is a name"),
    }
}

fn days(value: &Value) -> DaysAsDates {
    match value {
        Value::Days(days) => *days,
        _ => unreachable!("the complement is days"),
    }
}

fn task(subject: &Subject) -> TaskId {
    match subject {
        Subject::Task(task) => *task,
        _ => unreachable!("the subject is a task"),
    }
}

fn resource(subject: &Subject) -> &str {
    match subject {
        Subject::Resource(name) => name,
        _ => unreachable!("the subject is somebody"),
    }
}

fn day_of_week(subject: &Subject) -> DayOfWeek {
    match subject {
        Subject::DayOfWeek(day) => *day,
        _ => unreachable!("the subject is a day of the week"),
    }
}

fn subject_date(subject: &Subject) -> LocalDate {
    match subject {
        Subject::Date(date) => *date,
        _ => unreachable!("the subject is a date"),
    }
}

fn subject_days(subject: &Subject) -> DaysAsDates {
    match subject {
        Subject::Days(days) => *days,
        _ => unreachable!("the subject is days"),
    }
}

fn no_starting_date() -> CommandError {
    CommandError::new("No starting date for the project")
}

/// What tasks can be told (`SubjectTask.getVerbPhrases`).
fn task_verb_phrases() -> Vec<VerbPhraseAction> {
    let mut phrases = task_load_and_start_verb_phrases();
    phrases.extend(task_colour_and_occurrence_verb_phrases());
    phrases.extend(task_end_and_state_verb_phrases());
    phrases.extend(task_pause_and_link_verb_phrases());
    phrases
}

/// `lasts 5 days`, `starts at [Other]'s end` and the like.
fn task_load_and_start_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    let the_on_at = || Some(zero_or_more_words(&["the", "on", "at"]));
    vec![
        action(
            "【lasts┇requires】",
            Some(zero_or_more_words(&["on", "for", "the", "at"])),
            Complement::Duration,
            |gantt, subject, value| {
                let Value::Load(load) = value else {
                    unreachable!("the complement is a duration")
                };
                gantt.set_task_solver(task(subject), SolverValue::Load(load))
            },
        ),
        action(
            "starts",
            the_on_at(),
            Complement::TimePoint,
            |gantt, subject, value| {
                if gantt.is_relative() {
                    return Err(no_starting_date());
                }
                let Value::TimePoint(start) = value else {
                    unreachable!("the complement is an instant")
                };
                gantt.set_task_solver(task(subject), SolverValue::Start(start))
            },
        ),
        action(
            "starts",
            None,
            Complement::InstantWithColorLink,
            |gantt, subject, value| {
                let Value::InstantWithLink(when, link) = value else {
                    unreachable!("the complement is an instant and a link")
                };
                let task = task(subject);
                let start = gantt.model.instant_precise(&when);
                gantt.set_task_solver(task, SolverValue::Start(start))?;
                if when.is_task() {
                    let mut constraint = GanttConstraint::new(
                        when,
                        TaskInstant::new(Moment::Task(task), TaskAttribute::Start),
                        link.center.clone(),
                    );
                    constraint.apply_style(link.style.as_deref());
                    gantt.model.constraints.push(constraint);
                }
                Ok(())
            },
        ),
        action(
            "starts",
            None,
            Complement::BeforeOrAfterOrAtTaskStartOrEnd,
            |gantt, subject, value| {
                let when = instant(value);
                let task = task(subject);
                let start = gantt.model.instant_precise(&when);
                gantt.set_task_solver(task, SolverValue::Start(start))?;
                if when.is_task() {
                    gantt.model.constraints.push(GanttConstraint::new(
                        when,
                        TaskInstant::new(Moment::Task(task), TaskAttribute::Start),
                        None,
                    ));
                }
                Ok(())
            },
        ),
        action(
            "starts",
            the_on_at(),
            RELATIVE_DATE,
            |gantt, subject, value| {
                gantt.set_task_solver(
                    task(subject),
                    SolverValue::Start(TimePoint::of_start_of_day(date(&value))),
                )
            },
        ),
        action("starts", the_on_at(), ANY_DATE, |gantt, subject, value| {
            if gantt.is_relative() {
                return Err(no_starting_date());
            }
            gantt.set_task_solver(
                task(subject),
                SolverValue::Start(TimePoint::of_start_of_day(date(&value))),
            )
        }),
    ]
}

/// `is colored in`, `happens at`, `occurs from [A] to [B]`.
fn task_colour_and_occurrence_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    let the_on_at = || Some(zero_or_more_words(&["the", "on", "at"]));
    vec![
        action(
            "is 〇+〴s colo〇?ured",
            None,
            Complement::InColors,
            |gantt, subject, value| gantt.set_task_colors(task(subject), vec![colors(value)]),
        ),
        action(
            "is 〇+〴s colo〇?ured",
            Some(exactly2(&["for", "completion"])),
            Complement::InColorsFromTo,
            |gantt, subject, value| {
                let Value::ColorsFromTo(from, to) = value else {
                    unreachable!("the complement is two colours")
                };
                gantt.set_task_colors(task(subject), vec![from, to])
            },
        ),
        action("happens", the_on_at(), ANY_DATE, |gantt, subject, value| {
            let task = task(subject);
            gantt.set_task_solver(task, SolverValue::Load(86_400))?;
            gantt.set_task_solver(
                task,
                SolverValue::Start(TimePoint::of_start_of_day(date(&value))),
            )?;
            gantt.set_diamond(task)
        }),
        action(
            "happens",
            the_on_at(),
            Complement::BeforeOrAfterOrAtTaskStartOrEnd,
            |gantt, subject, value| {
                let task = task(subject);
                gantt.set_task_solver(task, SolverValue::Load(86_400))?;
                gantt.set_diamond(task)?;
                let when = instant(value);
                let mut start = gantt.model.instant_precise(&when);
                if when.attribute == TaskAttribute::End {
                    start = start.decrement();
                }
                gantt.set_task_solver(task, SolverValue::Start(start))
            },
        ),
        action(
            "occurs",
            None,
            Complement::FromTo,
            |gantt, subject, value| {
                let Value::TwoNames(name1, name2) = value else {
                    unreachable!("the complement is two tasks")
                };
                let from = gantt
                    .existing_task(&name1)
                    .ok_or_else(|| CommandError::new(format!("No such {name1} task")))?;
                let to = gantt
                    .existing_task(&name2)
                    .ok_or_else(|| CommandError::new(format!("No such {name2} task")))?;
                let task = task(subject);
                let start = gantt.model.start(from);
                gantt.set_task_solver(task, SolverValue::Start(start))?;
                let end = gantt.model.end(to);
                gantt.set_task_solver(task, SolverValue::End(end))?;
                gantt.model.constraints.push(GanttConstraint::new(
                    TaskInstant::new(Moment::Task(from), TaskAttribute::Start),
                    TaskInstant::new(Moment::Task(task), TaskAttribute::Start),
                    None,
                ));
                gantt.model.constraints.push(GanttConstraint::new(
                    TaskInstant::new(Moment::Task(to), TaskAttribute::End),
                    TaskInstant::new(Moment::Task(task), TaskAttribute::End),
                    None,
                ));
                Ok(())
            },
        ),
    ]
}

/// `ends at`, `displays on same row as`, `is deleted`, `is 40% completed`.
fn task_end_and_state_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    let the_on_at = || Some(zero_or_more_words(&["the", "on", "at"]));
    vec![
        action(
            "ends",
            the_on_at(),
            Complement::TimePoint,
            |gantt, subject, value| {
                if gantt.is_relative() {
                    return Err(no_starting_date());
                }
                let Value::TimePoint(end) = value else {
                    unreachable!("the complement is an instant")
                };
                gantt.set_task_solver(task(subject), SolverValue::End(end))
            },
        ),
        action(
            "ends",
            None,
            Complement::BeforeOrAfterOrAtTaskStartOrEnd,
            |gantt, subject, value| {
                let when = instant(value);
                let task = task(subject);
                let end = gantt.model.instant_precise(&when);
                gantt.set_task_solver(task, SolverValue::End(end))?;
                gantt.model.constraints.push(GanttConstraint::new(
                    when,
                    TaskInstant::new(Moment::Task(task), TaskAttribute::End),
                    None,
                ));
                Ok(())
            },
        ),
        action(
            "ends",
            the_on_at(),
            RELATIVE_DATE,
            |gantt, subject, value| {
                let end = TimePoint::of_start_of_day(date(&value)).increment();
                gantt.set_task_solver(task(subject), SolverValue::End(end))
            },
        ),
        action("ends", the_on_at(), ANY_DATE, |gantt, subject, value| {
            if gantt.is_relative() {
                return Err(no_starting_date());
            }
            let end = TimePoint::of_start_of_day(date(&value)).increment();
            gantt.set_task_solver(task(subject), SolverValue::End(end))
        }),
        action(
            "display〇?s 〇+〴s on 〇+〴s same 〇+〴s row 〇+〴s as",
            None,
            Complement::Named,
            |gantt, subject, value| {
                let other = name(value);
                let row = gantt
                    .existing_task(&other)
                    .ok_or_else(|| CommandError::new("No such task null"))?;
                let task = task(subject);
                if task != row {
                    gantt.model.tasks[task].row = Some(row);
                }
                Ok(())
            },
        ),
        action("is", None, Complement::Deleted, |gantt, subject, _| {
            gantt.set_task_colors(
                task(subject),
                vec![CenterBorderColor::new(
                    Some(HColor::WHITE),
                    Some(HColor::BLACK),
                )],
            )
        }),
        action(
            "is",
            None,
            Complement::Completed,
            |gantt, subject, value| {
                let Value::Completion(completion) = value else {
                    unreachable!("the complement is a completion")
                };
                gantt.task_impl_mut(task(subject))?.completion = completion;
                Ok(())
            },
        ),
    ]
}

/// `pauses on`, `links to`, `is displayed as`.
fn task_pause_and_link_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    let pause_words = || Some(zero_or_more_words(&["the", "on", "at", "from"]));
    vec![
        action(
            "pauses",
            pause_words(),
            Complement::Intervals,
            |gantt, subject, value| gantt.pause_days(task(subject), days(&value)),
        ),
        action(
            "pauses",
            pause_words(),
            Complement::IntervalsSmart,
            |gantt, subject, value| gantt.pause_days(task(subject), days(&value)),
        ),
        action(
            "pauses",
            pause_words(),
            ANY_DATE,
            |gantt, subject, value| {
                gantt.task_impl_mut(task(subject))?.add_pause(date(&value));
                Ok(())
            },
        ),
        action(
            "pauses",
            pause_words(),
            Complement::DayOfWeek,
            |gantt, subject, value| {
                let Value::DayOfWeek(day) = value else {
                    unreachable!("the complement is a day of the week")
                };
                gantt
                    .task_impl_mut(task(subject))?
                    .add_pause_day_of_week(day);
                Ok(())
            },
        ),
        action(
            "links 〇+〴s to",
            None,
            Complement::Url,
            |gantt, subject, value| {
                let Value::Url(url) = value else {
                    unreachable!("the complement is a link")
                };
                gantt.task_impl_mut(task(subject))?.url = Some(url);
                Ok(())
            },
        ),
        action(
            "is 〇+〴s displayed 〇+〴s as",
            None,
            Complement::Anything,
            |gantt, subject, value| {
                gantt.model.tasks[task(subject)].display_string = Some(name(value));
                Ok(())
            },
        ),
    ]
}

/// What people can be told (`SubjectResource.getVerbPhrases`).
fn resource_verb_phrases() -> Vec<VerbPhraseAction> {
    let mut phrases = vec![VerbPhraseAction::new(
        "works 〇+〴s on",
        None,
        Complement::Task,
        |gantt, subject, value| {
            let Value::Task(task) = value else {
                unreachable!("the complement is a task")
            };
            gantt.add_resource(task, resource(subject), 100)
        },
    )];
    phrases.extend(resource_off_verb_phrases());
    phrases.extend(resource_on_verb_phrases());
    phrases
}

fn off_words() -> UBrexPart {
    zero_or_more_words(&["from", "on", "for", "the", "at"])
}

/// `is off on`, `is off before`, `is off after`.
fn resource_off_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    vec![
        action(
            "is 〇+〴s off",
            Some(concat(vec![leaf("before"), zero_or_more_words(&["the"])])),
            ANY_DATE,
            |gantt, subject, value| {
                gantt
                    .resource_mut(resource(subject))
                    .open_close
                    .set_off_before_date(date(&value));
                Ok(())
            },
        ),
        action(
            "is 〇+〴s off",
            Some(concat(vec![leaf("after"), zero_or_more_words(&["the"])])),
            ANY_DATE,
            |gantt, subject, value| {
                gantt
                    .resource_mut(resource(subject))
                    .open_close
                    .set_off_after_date(date(&value));
                Ok(())
            },
        ),
        action(
            "is 〇+〴s off",
            Some(off_words()),
            Complement::Intervals,
            |gantt, subject, value| {
                let open_close = &mut gantt.resource_mut(resource(subject)).open_close;
                for day in days(&value).days() {
                    open_close.close(day);
                }
                Ok(())
            },
        ),
        action(
            "is 〇+〴s off",
            Some(off_words()),
            Complement::DayOfWeek,
            |gantt, subject, value| {
                let Value::DayOfWeek(day) = value else {
                    unreachable!("the complement is a day of the week")
                };
                gantt
                    .resource_mut(resource(subject))
                    .open_close
                    .close_day_of_week(day);
                Ok(())
            },
        ),
        action(
            "is 〇+〴s off",
            Some(off_words()),
            ANY_DATE,
            |gantt, subject, value| {
                gantt
                    .resource_mut(resource(subject))
                    .open_close
                    .close(date(&value));
                Ok(())
            },
        ),
    ]
}

/// `is on`, which works on days otherwise off.
fn resource_on_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    vec![
        action(
            "is 〇+〴s on",
            Some(off_words()),
            Complement::Intervals,
            |gantt, subject, value| {
                let open_close = &mut gantt.resource_mut(resource(subject)).open_close;
                for day in days(&value).days() {
                    open_close.open(day);
                }
                Ok(())
            },
        ),
        action(
            "is 〇+〴s on",
            Some(off_words()),
            ANY_DATE,
            |gantt, subject, value| {
                gantt
                    .resource_mut(resource(subject))
                    .open_close
                    .open(date(&value));
                Ok(())
            },
        ),
    ]
}

/// What runs of days can be told (`SubjectDaysAsDates.getVerbPhrases`).
fn days_verb_phrases() -> Vec<VerbPhraseAction> {
    let action = VerbPhraseAction::new;
    vec![
        action(
            "【is┇are】",
            None,
            Complement::Close,
            |gantt, subject, value| {
                let task = name(value);
                for day in subject_days(subject).days() {
                    gantt.close_day_as_date(day, &task);
                }
                Ok(())
            },
        ),
        action(
            "【is┇are】",
            None,
            Complement::Open,
            |gantt, subject, value| {
                let task = name(value);
                for day in subject_days(subject).days() {
                    gantt.open_day_as_date(day, &task);
                }
                Ok(())
            },
        ),
        action(
            "【is┇are】",
            None,
            Complement::InColors2,
            |gantt, subject, value| {
                let color = colors(value).center;
                for day in subject_days(subject).days() {
                    gantt.color_day(day, color.clone());
                }
                Ok(())
            },
        ),
        action(
            "【is┇are】〇+〴s named",
            None,
            Complement::Named,
            |gantt, subject, value| {
                let name = name(value);
                for day in subject_days(subject).days() {
                    gantt
                        .model
                        .calendar
                        .put_name_day(TimePoint::of_start_of_day(day), &name);
                }
                Ok(())
            },
        ),
    ]
}

/// A sentence with one kind of subject (`NaturalGanttCommand`).
pub(super) struct NaturalGanttCommand {
    subject: SubjectKind,
    subject_ubrex: UnicodeBracketedExpression,
    verb_phrases: Vec<VerbPhraseAction>,
    and: UnicodeBracketedExpression,
}

impl NaturalGanttCommand {
    pub(super) fn new(subject: SubjectKind) -> Self {
        Self {
            subject,
            subject_ubrex: subject.to_ubrex().build(),
            verb_phrases: subject.verb_phrases(),
            and: UBrexPart::leaf("〇+〴s and 〇+〴s").build(),
        }
    }

    /// Where the verb phrases start, after the subject: `None` when the subject does not match.
    fn after_subject<'a>(&self, line: &'a str) -> Option<(usize, UMatcher<'a>)> {
        let start = skip_spaces(line);
        let matcher = self.subject_ubrex.match_at(&line[start..])?;
        let length = matcher.accepted_match().len();
        if length == 0 {
            return None;
        }
        let position = start + length;
        Some((position + skip_spaces(&line[position..]), matcher))
    }
}

impl Command<GanttDiagram> for NaturalGanttCommand {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let Some(line) = lines.first() else {
            return CommandControl::NotOk;
        };
        let line = line.text();
        match self.after_subject(line) {
            Some((position, _))
                if self
                    .verb_phrases
                    .iter()
                    .any(|verb_phrase| verb_phrase.parse(&line[position..]).is_some()) =>
            {
                CommandControl::Ok
            }
            _ => CommandControl::NotOk,
        }
    }

    fn execute(&self, gantt: &mut GanttDiagram, lines: BlocLines) -> CommandResult {
        let line = lines.first().expect("a command has its line").text();
        let (mut position, subject_matcher) = self
            .after_subject(line)
            .expect("the subject matched when the command was chosen");
        let subject = self
            .subject
            .get_me(gantt, &subject_matcher)
            .map_err(CommandError::new)?;
        let mut candidates = self.verb_phrases.iter();
        while let Some(verb_phrase) = candidates.next() {
            let Some((length, complement_matcher)) = verb_phrase.parse(&line[position..]) else {
                continue;
            };
            let complement = verb_phrase
                .complement
                .get_me(gantt, &complement_matcher)
                .map_err(CommandError::new)?;
            (verb_phrase.action)(gantt, &subject, complement)?;
            position += length;
            let Some(and_length) = self
                .and
                .match_at(&line[position..])
                .map(|matcher| matcher.accepted_match().len())
                .filter(|length| *length > 0)
            else {
                return Ok(());
            };
            position += and_length;
            candidates = self.verb_phrases.iter();
        }
        Err(CommandError::new("No matching verb phrase"))
    }
}

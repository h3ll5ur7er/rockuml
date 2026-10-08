//! Instants of a project and how they print (PlantUML's `gantt.time` package and `GanttI18n`).

use super::i18n_time_data::{DAY_OF_WEEK_SHORT, MONTH_LONG, MONTH_SHORT};
use crate::local_date::{DayOfWeek, LocalDate, LocalDateTime, MONTH_NAMES};
use crate::ubrex::builder::UBrexPart;

const SECONDS_PER_DAY: i64 = 86_400;

/// An instant of the project, to the second, in UTC (`TimePoint`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct TimePoint(LocalDateTime);

impl TimePoint {
    pub(super) fn of(time: LocalDateTime) -> Self {
        Self(time)
    }

    pub(super) fn of_start_of_day(day: LocalDate) -> Self {
        Self(day.at_start_of_day())
    }

    pub(super) fn of_end_of_day_minus_one_second(day: LocalDate) -> Self {
        Self::of_start_of_day(day.plus_days(1)).minus_one_second()
    }

    pub(super) fn to_local_date_time(self) -> LocalDateTime {
        self.0
    }

    pub(super) fn to_day(self) -> LocalDate {
        self.0.to_local_date()
    }

    pub(super) fn to_day_of_week(self) -> DayOfWeek {
        self.to_day().day_of_week()
    }

    #[must_use]
    pub(super) fn increment(self) -> Self {
        self.add_days(1)
    }

    #[must_use]
    pub(super) fn decrement(self) -> Self {
        self.add_days(-1)
    }

    #[must_use]
    pub(super) fn add_days(self, days: i64) -> Self {
        Self(self.0.plus_days(days))
    }

    #[must_use]
    pub(super) fn minus_one_second(self) -> Self {
        Self(self.0.plus_seconds(-1))
    }

    pub(super) fn millis(self) -> i64 {
        self.0.epoch_second() * 1000
    }

    pub(super) fn absolute_day_num(self) -> i64 {
        self.0.epoch_second().div_euclid(SECONDS_PER_DAY)
    }

    pub(super) fn year(self) -> i64 {
        self.to_day().year()
    }

    /// 1 for January.
    pub(super) fn month_value(self) -> i64 {
        self.to_day().month_value()
    }

    pub(super) fn quarter(self) -> String {
        format!("Q{}", (self.month_value() + 2) / 3)
    }

    pub(super) fn day_of_month(self) -> i64 {
        self.to_day().day_of_month()
    }

    /// The year and month, which `monthYear` compares.
    pub(super) fn month_year(self) -> (i64, i64) {
        (self.year(), self.month_value())
    }

    pub(super) fn min(self, other: Self) -> Self {
        if self <= other { self } else { other }
    }

    pub(super) fn max(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }

    /// `toStringShort`: the month and day the language writes.
    pub(super) fn to_string_short(self, language: &str) -> String {
        TimePointFormat::MonthAndDay.format(self, language)
    }
}

/// The ways headers and tables write an instant (`TimePointFormat`).
#[derive(Clone, Copy)]
pub(super) enum TimePointFormat {
    DayOfWeekShort,
    MonthYearShort,
    MonthYearLong,
    MonthLong,
    MonthShort,
    Year,
    Quarter,
    DayOfMonth,
    MonthAndDay,
}

impl TimePointFormat {
    pub(super) fn format(self, when: TimePoint, language: &str) -> String {
        let month = (when.month_value() - 1) as usize;
        match self {
            Self::DayOfMonth => when.day_of_month().to_string(),
            Self::Quarter => when.quarter(),
            Self::Year => when.year().to_string(),
            Self::DayOfWeekShort => DAY_OF_WEEK_SHORT
                .get(when.to_day_of_week().ordinal(), language)
                .to_owned(),
            Self::MonthLong => MONTH_LONG.get(month, language).to_owned(),
            Self::MonthShort => MONTH_SHORT.get(month, language).to_owned(),
            Self::MonthYearShort => format!("{} {}", MONTH_SHORT.get(month, language), when.year()),
            Self::MonthYearLong => format!("{} {}", MONTH_LONG.get(month, language), when.year()),
            Self::MonthAndDay => month_and_day(when, language),
        }
    }
}

fn month_and_day(when: TimePoint, language: &str) -> String {
    let day = when.day_of_month();
    let short = || TimePointFormat::MonthShort.format(when, language);
    match language {
        "fr" => format!("{day} {}", short()),
        "es" => format!("{day} de {}", short()),
        "de" => format!("{day}. {}", short()),
        "ja" => format!("{}/{day}日", short()),
        "ko" => format!("{} {day}일", short()),
        "ru" => format!(
            "{day} {}",
            TimePointFormat::MonthLong
                .format(when, language)
                .to_lowercase()
        ),
        "zh" => format!("{}{day}日", short()),
        _ => format!("{} {day}", short()),
    }
}

/// Any case of the first three letters of a name, then any letters (`DayOfWeekUtils.getUbrex` and
/// `MonthUtils.getUbrex`).
fn names_ubrex(names: &[&str], separator: &str) -> UBrexPart {
    UBrexPart::or(
        names
            .iter()
            .map(|name| {
                let mut pattern = String::new();
                for c in name[..3].chars() {
                    pattern.push('「');
                    pattern.push(c.to_ascii_uppercase());
                    pattern.push(c.to_ascii_lowercase());
                    pattern.push('」');
                }
                pattern.push_str(separator);
                pattern.push_str("〇*〴le");
                UBrexPart::leaf(&pattern)
            })
            .collect(),
    )
}

pub(super) fn day_of_week_ubrex() -> UBrexPart {
    let names: Vec<&str> = DayOfWeek::ALL.iter().map(|day| day.name()).collect();
    names_ubrex(&names, "")
}

pub(super) fn month_ubrex() -> UBrexPart {
    names_ubrex(&MONTH_NAMES, " ")
}

/// The day named by its first three letters (`DayOfWeekUtils.fromString`).
pub(super) fn day_of_week_from_string(value: &str) -> Option<DayOfWeek> {
    let prefix = value.get(..3)?.to_ascii_uppercase();
    DayOfWeek::ALL
        .into_iter()
        .find(|day| day.name().starts_with(&prefix))
}

/// The month named by its first three letters, 1 for January (`MonthUtils.fromString`).
pub(super) fn month_from_string(value: &str) -> Option<i64> {
    let prefix = value.get(..3)?.to_ascii_uppercase();
    MONTH_NAMES
        .iter()
        .position(|name| name.starts_with(&prefix))
        .map(|index| index as i64 + 1)
}

/// The words of the task table (`GanttI18n`).
pub(super) mod i18n {
    pub(in super::super) fn task(language: &str) -> &'static str {
        match language {
            "fr" => "Tâche",
            "es" => "Tarea",
            "de" => "Aufgabe",
            "ja" => "タスク",
            "ko" => "작업",
            "ru" => "Задача",
            "zh" => "任务",
            _ => "Task",
        }
    }

    pub(in super::super) fn start(language: &str) -> &'static str {
        match language {
            "fr" => "Début",
            "es" => "Inicio",
            "ja" => "開始",
            "ko" => "시작",
            "ru" => "Начало",
            "zh" => "开始",
            _ => "Start",
        }
    }

    pub(in super::super) fn end(language: &str) -> &'static str {
        match language {
            "fr" | "es" => "Fin",
            "de" => "Ende",
            "ja" => "終了",
            "ko" => "끝",
            "ru" => "Окончание",
            "zh" => "结束",
            _ => "End",
        }
    }

    pub(in super::super) fn duration(language: &str) -> &'static str {
        match language {
            "fr" => "Durée",
            "es" => "Duración",
            "de" => "Dauer",
            "ja" => "期間",
            "ko" => "기간",
            "ru" => "Длительность",
            "zh" => "持续时间",
            _ => "Duration",
        }
    }

    /// `Day 3` for a project without dates.
    pub(in super::super) fn day_number(language: &str, day: i64) -> String {
        match language {
            "fr" => format!("Jour {day}"),
            "es" => format!("Día {day}"),
            "de" => format!("Tag {day}"),
            "ja" => format!("{day}日目"),
            "ko" => format!("{day}일째"),
            "ru" => format!("День {day}"),
            "zh" => format!("第{day}天"),
            _ => format!("Day {day}"),
        }
    }

    /// The words for no time, the separator between units, whether numbers stick to their unit, and each
    /// unit's forms: singular and plural, one invariable form, or Russian's singular, genitive plural and
    /// paucal.
    struct Units {
        none: &'static str,
        separator: &'static str,
        suffix_no_space: bool,
        labels: [&'static [&'static str]; 4],
    }

    impl Units {
        fn of(language: &str) -> Self {
            let units = |none, separator, suffix_no_space, labels| Self {
                none,
                separator,
                suffix_no_space,
                labels,
            };
            match language {
                "fr" => units(
                    "aucune",
                    ", ",
                    false,
                    [
                        &["jour", "jours"],
                        &["heure", "heures"],
                        &["minute", "minutes"],
                        &["seconde", "secondes"],
                    ],
                ),
                "es" => units(
                    "ninguna",
                    ", ",
                    false,
                    [
                        &["día", "días"],
                        &["hora", "horas"],
                        &["minuto", "minutos"],
                        &["segundo", "segundos"],
                    ],
                ),
                "de" => units(
                    "keine",
                    ", ",
                    false,
                    [
                        &["Tag", "Tage"],
                        &["Stunde", "Stunden"],
                        &["Minute", "Minuten"],
                        &["Sekunde", "Sekunden"],
                    ],
                ),
                "ja" => units("なし", "", true, [&["日"], &["時間"], &["分"], &["秒"]]),
                "ko" => units("없음", ", ", true, [&["일"], &["시간"], &["분"], &["초"]]),
                "ru" => units(
                    "нет",
                    ", ",
                    false,
                    [
                        &["день", "дней", "дня"],
                        &["час", "часов", "часа"],
                        &["минута", "минут", "минуты"],
                        &["секунда", "секунд", "секунды"],
                    ],
                ),
                "zh" => units("无", "", true, [&["天"], &["小时"], &["分钟"], &["秒"]]),
                _ => units(
                    "none",
                    ", ",
                    false,
                    [
                        &["day", "days"],
                        &["hour", "hours"],
                        &["minute", "minutes"],
                        &["second", "seconds"],
                    ],
                ),
            }
        }

        fn label(&self, unit: usize, value: i64) -> &'static str {
            match self.labels[unit] {
                [only] => only,
                [singular, genitive_plural, paucal] => {
                    let (mod100, mod10) = (value % 100, value % 10);
                    if mod10 == 1 && mod100 != 11 {
                        singular
                    } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
                        paucal
                    } else {
                        genitive_plural
                    }
                }
                [singular, plural] => {
                    if value > 1 {
                        plural
                    } else {
                        singular
                    }
                }
                _ => unreachable!("every unit has one to three forms"),
            }
        }
    }

    /// `3 days, 4 hours` (`durationHumanReadable`).
    pub(in super::super) fn duration_human_readable(language: &str, seconds: i64) -> String {
        let units = Units::of(language);
        if seconds == 0 {
            return units.none.to_owned();
        }
        let values = [
            seconds / 86_400,
            seconds % 86_400 / 3600,
            seconds % 3600 / 60,
            seconds % 60,
        ];
        let mut result = String::new();
        for (unit, value) in values.into_iter().enumerate() {
            if value <= 0 {
                continue;
            }
            if !result.is_empty() {
                result.push_str(units.separator);
            }
            result.push_str(&value.to_string());
            if !units.suffix_no_space {
                result.push(' ');
            }
            result.push_str(units.label(unit, value));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(year: i64, month: i64, day: i64) -> TimePoint {
        TimePoint::of_start_of_day(LocalDate::of(year, month, day).unwrap())
    }

    #[test]
    fn instants_print_in_the_project_language() {
        let when = point(2020, 10, 15);
        assert_eq!(when.to_string_short("en"), "Oct 15");
        assert_eq!(point(2021, 3, 1).to_string_short("de"), "1. Mär");
        assert_eq!(
            TimePointFormat::MonthYearLong.format(when, "en"),
            "October 2020"
        );
        assert_eq!(TimePointFormat::DayOfWeekShort.format(when, "en"), "Th");
        assert_eq!(when.quarter(), "Q4");
    }

    #[test]
    fn durations_read_as_words() {
        assert_eq!(i18n::duration_human_readable("en", 10 * 86_400), "10 days");
        assert_eq!(
            i18n::duration_human_readable("en", 86_400 + 7200),
            "1 day, 2 hours"
        );
        assert_eq!(i18n::duration_human_readable("de", 7 * 86_400), "7 Tage");
        assert_eq!(i18n::duration_human_readable("ru", 22 * 86_400), "22 дня");
    }

    #[test]
    fn names_read_from_their_first_letters() {
        assert_eq!(
            day_of_week_from_string("saturday"),
            Some(DayOfWeek::Saturday)
        );
        assert_eq!(month_from_string("september"), Some(9));
    }
}

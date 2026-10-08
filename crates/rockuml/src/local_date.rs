//! Dates and times of the proleptic Gregorian calendar, as `java.time.LocalDate`, `LocalDateTime` and
//! `DayOfWeek` compute them. Times carry no zone: PlantUML reads and prints them in UTC.

use std::fmt;

/// A day, kept as its distance from 1970-01-01.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LocalDate {
    epoch_day: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl DayOfWeek {
    pub(crate) const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    /// The Java constant's name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Monday => "MONDAY",
            Self::Tuesday => "TUESDAY",
            Self::Wednesday => "WEDNESDAY",
            Self::Thursday => "THURSDAY",
            Self::Friday => "FRIDAY",
            Self::Saturday => "SATURDAY",
            Self::Sunday => "SUNDAY",
        }
    }

    /// `getValue`: 1 for Monday to 7 for Sunday.
    pub(crate) fn value(self) -> i64 {
        self as i64 + 1
    }

    pub(crate) fn ordinal(self) -> usize {
        self as usize
    }
}

/// The names of the months, as Java's `Month` constants.
pub(crate) const MONTH_NAMES: [&str; 12] = [
    "JANUARY",
    "FEBRUARY",
    "MARCH",
    "APRIL",
    "MAY",
    "JUNE",
    "JULY",
    "AUGUST",
    "SEPTEMBER",
    "OCTOBER",
    "NOVEMBER",
    "DECEMBER",
];

const SECONDS_PER_DAY: i64 = 86_400;

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn month_length(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

impl LocalDate {
    pub(crate) const EPOCH: Self = Self { epoch_day: 0 };

    /// `LocalDate.of`; `None` for a day that does not exist.
    pub(crate) fn of(year: i64, month: i64, day: i64) -> Option<Self> {
        if !(1..=12).contains(&month) || day < 1 || day > month_length(year, month) {
            return None;
        }
        let year = if month <= 2 { year - 1 } else { year };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month_from_march = (month + 9) % 12;
        let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        Some(Self {
            epoch_day: era * 146_097 + day_of_era - 719_468,
        })
    }

    pub(crate) fn of_epoch_day(epoch_day: i64) -> Self {
        Self { epoch_day }
    }

    pub(crate) fn epoch_day(self) -> i64 {
        self.epoch_day
    }

    /// The year, month and day of the month.
    fn civil(self) -> (i64, i64, i64) {
        let days = self.epoch_day + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = if month_from_march < 10 {
            month_from_march + 3
        } else {
            month_from_march - 9
        };
        (year_of_era + era * 400 + i64::from(month <= 2), month, day)
    }

    pub(crate) fn year(self) -> i64 {
        self.civil().0
    }

    /// 1 for January to 12 for December.
    pub(crate) fn month_value(self) -> i64 {
        self.civil().1
    }

    pub(crate) fn day_of_month(self) -> i64 {
        self.civil().2
    }

    /// 1 for January 1.
    pub(crate) fn day_of_year(self) -> i64 {
        let first = Self::of(self.year(), 1, 1).expect("January 1 exists");
        self.epoch_day - first.epoch_day + 1
    }

    pub(crate) fn day_of_week(self) -> DayOfWeek {
        DayOfWeek::ALL[(self.epoch_day + 3).rem_euclid(7) as usize]
    }

    #[must_use]
    pub(crate) fn plus_days(self, days: i64) -> Self {
        Self {
            epoch_day: self.epoch_day + days,
        }
    }

    pub(crate) fn at_start_of_day(self) -> LocalDateTime {
        LocalDateTime {
            epoch_second: self.epoch_day * SECONDS_PER_DAY,
        }
    }

    /// The week of the year with weeks starting on `first_day_of_week`, the first holding at least
    /// `minimal_days` days of the year; days before it are in week 0 (`WeekFields.weekOfYear`).
    pub(crate) fn week_of_year(self, first_day_of_week: DayOfWeek, minimal_days: i64) -> i64 {
        let dow = (self.day_of_week().value() - first_day_of_week.value()).rem_euclid(7) + 1;
        let doy = self.day_of_year();
        let week_start = (doy - dow).rem_euclid(7);
        let offset = if week_start + 1 > minimal_days {
            7 - week_start
        } else {
            -week_start
        };
        (7 + offset + (doy - 1)) / 7
    }
}

/// `toString`: `2020-07-01`.
impl fmt::Display for LocalDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.civil();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

/// A date and time to the second, kept as seconds from 1970-01-01T00:00.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LocalDateTime {
    epoch_second: i64,
}

impl LocalDateTime {
    pub(crate) fn of(date: LocalDate, hour: i64, minute: i64, second: i64) -> Self {
        Self {
            epoch_second: date.epoch_day * SECONDS_PER_DAY + hour * 3600 + minute * 60 + second,
        }
    }

    pub(crate) fn of_epoch_second(epoch_second: i64) -> Self {
        Self { epoch_second }
    }

    pub(crate) fn epoch_second(self) -> i64 {
        self.epoch_second
    }

    pub(crate) fn to_local_date(self) -> LocalDate {
        LocalDate::of_epoch_day(self.epoch_second.div_euclid(SECONDS_PER_DAY))
    }

    fn second_of_day(self) -> i64 {
        self.epoch_second.rem_euclid(SECONDS_PER_DAY)
    }

    pub(crate) fn hour(self) -> i64 {
        self.second_of_day() / 3600
    }

    pub(crate) fn minute(self) -> i64 {
        self.second_of_day() / 60 % 60
    }

    pub(crate) fn second(self) -> i64 {
        self.second_of_day() % 60
    }

    pub(crate) fn is_midnight(self) -> bool {
        self.second_of_day() == 0
    }

    #[must_use]
    pub(crate) fn plus_seconds(self, seconds: i64) -> Self {
        Self {
            epoch_second: self.epoch_second.saturating_add(seconds),
        }
    }

    #[must_use]
    pub(crate) fn plus_days(self, days: i64) -> Self {
        self.plus_seconds(days * SECONDS_PER_DAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i64, month: i64, day: i64) -> LocalDate {
        LocalDate::of(year, month, day).unwrap()
    }

    #[test]
    fn dates_count_days_from_1970() {
        assert_eq!(date(1970, 1, 1).epoch_day(), 0);
        assert_eq!(date(2019, 7, 2).epoch_day(), 18_079);
        assert_eq!(date(2019, 7, 2).to_string(), "2019-07-02");
        assert_eq!(date(1969, 12, 31).epoch_day(), -1);
        assert_eq!(date(2020, 2, 29).plus_days(1), date(2020, 3, 1));
        assert!(LocalDate::of(2021, 2, 29).is_none());
        assert_eq!(date(2019, 7, 2).day_of_year(), 183);
    }

    #[test]
    fn weekdays_and_weeks_follow_java() {
        assert_eq!(date(2020, 10, 15).day_of_week(), DayOfWeek::Thursday);
        assert_eq!(date(1970, 1, 1).day_of_week(), DayOfWeek::Thursday);
        assert_eq!(date(2021, 1, 1).week_of_year(DayOfWeek::Monday, 4), 0);
        assert_eq!(date(2021, 1, 4).week_of_year(DayOfWeek::Monday, 4), 1);
        assert_eq!(date(2020, 9, 21).week_of_year(DayOfWeek::Monday, 4), 39);
    }

    #[test]
    fn times_split_into_hours_minutes_and_seconds() {
        let time = LocalDateTime::of(date(2020, 1, 1), 13, 5, 9);
        assert_eq!((time.hour(), time.minute(), time.second()), (13, 5, 9));
        assert_eq!(time.to_local_date(), date(2020, 1, 1));
        assert!(date(2020, 1, 1).at_start_of_day().is_midnight());
    }
}

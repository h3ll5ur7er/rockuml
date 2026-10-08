//! Times as timing diagrams read and print them (PlantUML's `TimeTick`, `TimingFormat` and the `BigDecimal`
//! times are kept in).

use std::cmp::Ordering;
use std::fmt;

/// A decimal number as `java.math.BigDecimal` keeps it: its digits and how many of them are decimals, so
/// that `1.50` prints back as `1.50`.
#[derive(Clone, Copy, Debug)]
pub(super) struct BigDecimal {
    unscaled: i128,
    scale: u32,
}

impl BigDecimal {
    pub(super) const ZERO: Self = Self::from_long(0);

    pub(super) const fn from_long(value: i64) -> Self {
        Self {
            unscaled: value as i128,
            scale: 0,
        }
    }

    /// `new BigDecimal(text)` for the plain decimals the commands accept, like `-12`, `+.5` or `100.`.
    pub(super) fn parse(text: &str) -> Option<Self> {
        let (negative, digits) = match text.as_bytes().first()? {
            b'-' => (true, &text[1..]),
            b'+' => (false, &text[1..]),
            _ => (false, text),
        };
        let (integer, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        if integer.is_empty() && fraction.is_empty()
            || !integer
                .bytes()
                .chain(fraction.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let unscaled: i128 = format!("{integer}{fraction}").parse().ok()?;
        Some(Self {
            unscaled: if negative { -unscaled } else { unscaled },
            scale: u32::try_from(fraction.len()).ok()?,
        })
    }

    fn rescaled(self, scale: u32) -> i128 {
        self.unscaled * 10i128.pow(scale - self.scale)
    }

    #[must_use]
    pub(super) fn add(self, other: Self) -> Self {
        let scale = self.scale.max(other.scale);
        Self {
            unscaled: self.rescaled(scale) + other.rescaled(scale),
            scale,
        }
    }

    #[must_use]
    pub(super) fn multiply(self, factor: i64) -> Self {
        Self {
            unscaled: self.unscaled * i128::from(factor),
            scale: self.scale,
        }
    }

    pub(super) fn double_value(self) -> f64 {
        self.unscaled as f64 / 10f64.powi(i32::try_from(self.scale).expect("a short decimal"))
    }

    /// `longValue`: the integer part.
    pub(super) fn long_value(self) -> i64 {
        i64::try_from(self.unscaled / 10i128.pow(self.scale)).expect("a time that fits a long")
    }

    pub(super) fn signum(self) -> i32 {
        match self.unscaled.cmp(&0) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

impl PartialEq for BigDecimal {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for BigDecimal {}

impl PartialOrd for BigDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `compareTo`, which ignores the scale.
impl Ord for BigDecimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let scale = self.scale.max(other.scale);
        self.rescaled(scale).cmp(&other.rescaled(scale))
    }
}

/// `toPlainString`.
impl fmt::Display for BigDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.unscaled.unsigned_abs().to_string();
        let sign = if self.unscaled < 0 { "-" } else { "" };
        let scale = self.scale as usize;
        if scale == 0 {
            return write!(f, "{sign}{digits}");
        }
        let digits = format!("{digits:0>width$}", width = scale + 1);
        let (integer, fraction) = digits.split_at(digits.len() - scale);
        write!(f, "{sign}{integer}.{fraction}")
    }
}

/// How times print: plain numbers, hours, dates, or dates in a `use date format` pattern.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum TimingFormat {
    Decimal,
    Hour,
    Date,
    SimpleDate(String),
}

impl TimingFormat {
    pub(super) fn format_decimal(&self, time: BigDecimal) -> String {
        match self {
            Self::Decimal => time.to_string(),
            _ => self.format_long(time.long_value()),
        }
    }

    pub(super) fn format_long(&self, time: i64) -> String {
        match self {
            Self::Decimal => time.to_string(),
            Self::Hour => {
                let s = (time as i32) % 60;
                let m = ((time / 60) as i32) % 60;
                let h = (time / 3600) as i32;
                format!("{h}:{m:02}:{s:02}")
            }
            Self::Date => {
                let (_, month, day) = civil_from_days(time.div_euclid(SECONDS_PER_DAY));
                format!("{month:02}/{day:02}")
            }
            Self::SimpleDate(pattern) => simple_date_format(pattern, time),
        }
    }

    /// `createDate`: midnight GMT of the day, in seconds.
    pub(super) fn create_date(year: i64, month: i64, day: i64, format: Self) -> TimeTick {
        TimeTick::new(
            BigDecimal::from_long(days_from_civil(year, month, day) * SECONDS_PER_DAY),
            format,
        )
    }
}

const SECONDS_PER_DAY: i64 = 86_400;

/// Days since 1970-01-01 of a date in the proleptic Gregorian calendar.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The year, month and day of a day since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
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

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Whether `SimpleDateFormat` would accept the pattern, as far as rockuml formats patterns.
pub(super) fn is_supported_date_format(pattern: &str) -> bool {
    pattern_pieces(pattern).is_some()
}

enum PatternPiece {
    Literal(String),
    Field(char, usize),
}

fn pattern_pieces(pattern: &str) -> Option<Vec<PatternPiece>> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut pieces = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            if chars.get(i + 1) == Some(&'\'') {
                pieces.push(PatternPiece::Literal("'".to_owned()));
                i += 2;
                continue;
            }
            let end = chars[i + 1..].iter().position(|&c| c == '\'')? + i + 1;
            pieces.push(PatternPiece::Literal(chars[i + 1..end].iter().collect()));
            i = end + 1;
        } else if c.is_ascii_alphabetic() {
            if !"GyYMLdDEuHkKhmsSa".contains(c) {
                return None;
            }
            let count = chars[i..].iter().take_while(|&&next| next == c).count();
            pieces.push(PatternPiece::Field(c, count));
            i += count;
        } else {
            pieces.push(PatternPiece::Literal(c.to_string()));
            i += 1;
        }
    }
    Some(pieces)
}

/// `SimpleDateFormat.format` in US English. PlantUML formats in the JVM's time zone; rockuml in UTC, as the
/// dates themselves are read.
fn simple_date_format(pattern: &str, seconds: i64) -> String {
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let weekday = (days + 4).rem_euclid(7);
    let hour = second_of_day / 3600;
    let mut result = String::new();
    for piece in pattern_pieces(pattern).unwrap_or_default() {
        match piece {
            PatternPiece::Literal(text) => result.push_str(&text),
            PatternPiece::Field(letter, count) => {
                let number = |value: i64| format!("{value:0count$}");
                let text = match letter {
                    'G' => "AD".to_owned(),
                    'y' | 'Y' => {
                        let year = if letter == 'Y' {
                            week_year(days, year)
                        } else {
                            year
                        };
                        if count == 2 {
                            format!("{:02}", year.rem_euclid(100))
                        } else {
                            number(year)
                        }
                    }
                    'M' | 'L' => text_or_number(MONTHS[(month - 1) as usize], count, month),
                    'd' => number(day),
                    'D' => number(days - days_from_civil(year, 1, 1) + 1),
                    'E' => text_or_name(WEEKDAYS[weekday as usize], count),
                    'u' => number(if weekday == 0 { 7 } else { weekday }),
                    'H' => number(hour),
                    'k' => number(if hour == 0 { 24 } else { hour }),
                    'K' => number(hour % 12),
                    'h' => number(if hour % 12 == 0 { 12 } else { hour % 12 }),
                    'm' => number(second_of_day / 60 % 60),
                    's' => number(second_of_day % 60),
                    'S' => number(0),
                    'a' => if hour < 12 { "AM" } else { "PM" }.to_owned(),
                    _ => unreachable!("pattern_pieces keeps known letters"),
                };
                result.push_str(&text);
            }
        }
    }
    result
}

/// A month as text from three letters on, else as a number.
fn text_or_number(name: &str, count: usize, value: i64) -> String {
    if count >= 3 {
        text_or_name(name, count)
    } else {
        format!("{value:0count$}")
    }
}

/// The full name from four letters on, else its first three letters.
fn text_or_name(name: &str, count: usize) -> String {
    if count >= 4 {
        name.to_owned()
    } else {
        name[..3].to_owned()
    }
}

/// The year the US week of the day belongs to: weeks start on Sunday and the first holds January 1.
fn week_year(days: i64, year: i64) -> i64 {
    let next_new_year = days_from_civil(year + 1, 1, 1);
    let first_week_start = next_new_year - (next_new_year + 4).rem_euclid(7);
    if days >= first_week_start {
        year + 1
    } else {
        year
    }
}

/// A moment of the diagram, ordered by time only (`TimeTick`).
#[derive(Clone, Debug)]
pub(super) struct TimeTick {
    pub(super) time: BigDecimal,
    pub(super) format: TimingFormat,
}

impl TimeTick {
    pub(super) fn new(time: BigDecimal, format: TimingFormat) -> Self {
        Self { time, format }
    }
}

impl PartialEq for TimeTick {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for TimeTick {}

impl PartialOrd for TimeTick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeTick {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decimal(text: &str) -> BigDecimal {
        BigDecimal::parse(text).unwrap()
    }

    #[test]
    fn decimals_keep_their_scale() {
        assert_eq!(decimal("1.50").to_string(), "1.50");
        assert_eq!(decimal("+.5").to_string(), "0.5");
        assert_eq!(decimal("-0.05").to_string(), "-0.05");
        assert_eq!(decimal("100.").to_string(), "100");
        assert_eq!(decimal("1.5").add(decimal("2.25")).to_string(), "3.75");
        assert_eq!(decimal("0.5").multiply(3).to_string(), "1.5");
        assert_eq!(decimal("1.50"), decimal("1.5"));
        assert_eq!(decimal("-2.7").long_value(), -2);
        assert!(BigDecimal::parse("+.").is_none());
    }

    #[test]
    fn hours_and_dates_print_like_plantuml() {
        assert_eq!(TimingFormat::Hour.format_long(3600 + 120 + 5), "1:02:05");
        let tick = TimingFormat::create_date(2019, 7, 2, TimingFormat::Date);
        assert_eq!(tick.time.to_string(), "1562025600");
        assert_eq!(
            TimingFormat::Date.format_long(tick.time.long_value()),
            "07/02"
        );
    }

    #[test]
    fn date_patterns_print_like_simple_date_format() {
        let seconds = TimingFormat::create_date(2019, 7, 2, TimingFormat::Date)
            .time
            .long_value();
        assert_eq!(simple_date_format("YY-MM-DD", seconds), "19-07-183");
        assert_eq!(simple_date_format("yyyy/MM/dd", seconds), "2019/07/02");
        assert_eq!(simple_date_format("EEE d MMM", seconds), "Tue 2 Jul");
        assert_eq!(
            simple_date_format("EEEE, MMMM 'the' d", seconds),
            "Tuesday, July the 2"
        );
        let new_year_week = TimingFormat::create_date(2019, 12, 30, TimingFormat::Date)
            .time
            .long_value();
        assert_eq!(simple_date_format("YYYY yyyy", new_year_week), "2020 2019");
        assert!(!is_supported_date_format("yyyy-bb"));
    }
}

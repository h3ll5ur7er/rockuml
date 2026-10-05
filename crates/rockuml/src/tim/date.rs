//! `%date`: Java's `Date.toString()` and `SimpleDateFormat`, in English.

use jiff::Timestamp;
use jiff::Zoned;
use jiff::tz::TimeZone;

use super::error::{TimResult, fail};
use super::value::TValue;
use crate::host::Host;
use crate::text::StringLocated;

/// `%date()`, `%date(format)`, `%date(format, epochSeconds)` or `%date(format, epochSeconds, zone)`.
pub fn format_date(
    host: &dyn Host,
    arguments: &[TValue],
    location: &StringLocated,
) -> TimResult<TValue> {
    let local_zone = || {
        host.local_time_zone()
            .and_then(|name| TimeZone::get(&name).ok())
            .unwrap_or(TimeZone::UTC)
    };
    let Some(format) = arguments.first() else {
        return Ok(TValue::string(java_date_to_string(
            host.current_time_millis(),
            &local_zone(),
        )));
    };
    let millis = match arguments.get(1) {
        Some(seconds) => 1000 * i64::from(seconds.to_int()),
        None => host.current_time_millis(),
    };
    let zone = match arguments.get(2) {
        Some(name) => match TimeZone::get(&name.to_string()) {
            Ok(zone) => zone,
            Err(_) => return fail(format!("Unknown time zone: {name}"), location),
        },
        None => local_zone(),
    };
    match simple_date_format(&format.to_string(), millis, &zone) {
        Some(text) => Ok(TValue::string(text)),
        None => fail("Bad date pattern", location),
    }
}

/// `Date.toString()`, e.g. `Mon Oct 05 14:49:18 CEST 2026`.
pub fn java_date_to_string(millis: i64, zone: &TimeZone) -> String {
    simple_date_format("EEE MMM dd HH:mm:ss zzz yyyy", millis, zone).unwrap_or_default()
}

const DAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const MONTH_NAMES: [&str; 12] = [
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

/// `None` for patterns `SimpleDateFormat` rejects.
pub fn simple_date_format(pattern: &str, millis: i64, zone: &TimeZone) -> Option<String> {
    let time = Timestamp::from_millisecond(millis)
        .ok()?
        .to_zoned(zone.clone());
    let chars: Vec<char> = pattern.chars().collect();
    let mut result = String::new();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        if c == '\'' {
            index += 1;
            if chars.get(index) == Some(&'\'') {
                result.push('\'');
                index += 1;
                continue;
            }
            loop {
                match chars.get(index) {
                    None => return None,
                    Some('\'') if chars.get(index + 1) == Some(&'\'') => {
                        result.push('\'');
                        index += 2;
                    }
                    Some('\'') => {
                        index += 1;
                        break;
                    }
                    Some(&quoted) => {
                        result.push(quoted);
                        index += 1;
                    }
                }
            }
            continue;
        }
        if !c.is_ascii_alphabetic() {
            result.push(c);
            index += 1;
            continue;
        }
        let count = chars[index..].iter().take_while(|&&next| next == c).count();
        result.push_str(&field(c, count, &time)?);
        index += count;
    }
    Some(result)
}

fn field(letter: char, count: usize, time: &Zoned) -> Option<String> {
    let padded = |value: i64| format!("{value:0count$}");
    let date = time.date();
    Some(match letter {
        'G' => "AD".to_owned(),
        'y' | 'Y' if count == 2 => format!("{:02}", i64::from(date.year()).rem_euclid(100)),
        'y' | 'Y' => padded(i64::from(date.year())),
        'M' | 'L' => month(i64::from(date.month()), count),
        'd' => padded(i64::from(date.day())),
        'D' => padded(i64::from(date.day_of_year())),
        'E' => {
            let name = DAY_NAMES[usize::try_from(date.weekday().to_monday_zero_offset()).ok()?];
            if count >= 4 {
                name.to_owned()
            } else {
                name[..3].to_owned()
            }
        }
        'u' => padded(i64::from(date.weekday().to_monday_one_offset())),
        'F' => padded((i64::from(date.day()) - 1) / 7 + 1),
        'a' => if time.hour() < 12 { "AM" } else { "PM" }.to_owned(),
        'H' => padded(i64::from(time.hour())),
        'k' => padded(if time.hour() == 0 {
            24
        } else {
            i64::from(time.hour())
        }),
        'K' => padded(i64::from(time.hour()) % 12),
        'h' => padded(match i64::from(time.hour()) % 12 {
            0 => 12,
            hour => hour,
        }),
        'm' => padded(i64::from(time.minute())),
        's' => padded(i64::from(time.second())),
        'S' => padded(i64::from(time.millisecond())),
        'z' => abbreviation(time),
        'Z' => offset(time, false, true),
        'X' if time.offset().seconds() == 0 => "Z".to_owned(),
        'X' => match count {
            1 => offset(time, false, false)[..3].to_owned(),
            2 => offset(time, false, true),
            3 => offset(time, true, true),
            _ => return None,
        },
        'w' => padded(i64::from(date.iso_week_date().week())),
        _ => return None,
    })
}

fn month(month: i64, count: usize) -> String {
    let name = MONTH_NAMES[usize::try_from(month - 1).unwrap_or(0)];
    match count {
        1 => month.to_string(),
        2 => format!("{month:02}"),
        3 => name[..3].to_owned(),
        _ => name.to_owned(),
    }
}

fn abbreviation(time: &Zoned) -> String {
    let abbreviation = time
        .time_zone()
        .to_offset_info(time.timestamp())
        .abbreviation()
        .to_owned();
    if abbreviation.starts_with(['+', '-']) {
        format!("GMT{}", offset(time, true, true))
    } else {
        abbreviation
    }
}

fn offset(time: &Zoned, with_colon: bool, with_minutes: bool) -> String {
    let seconds = time.offset().seconds();
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.abs() / 60;
    let (hours, minutes) = (minutes / 60, minutes % 60);
    match (with_minutes, with_colon) {
        (false, _) => format!("{sign}{hours:02}"),
        (true, true) => format!("{sign}{hours:02}:{minutes:02}"),
        (true, false) => format!("{sign}{hours:02}{minutes:02}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(pattern: &str, millis: i64) -> Option<String> {
        simple_date_format(pattern, millis, &TimeZone::UTC)
    }

    #[test]
    fn formats_numeric_fields_with_padding() {
        let millis = 1_759_668_558_007;
        assert_eq!(
            utc("yyyy-MM-dd HH:mm:ss.SSS", millis).as_deref(),
            Some("2025-10-05 12:49:18.007")
        );
        assert_eq!(utc("yy/M/d h a", millis).as_deref(), Some("25/10/5 12 PM"));
    }

    #[test]
    fn formats_names_and_quoted_text() {
        assert_eq!(
            utc("EEE, MMM d ''yy 'at' HH", 0).as_deref(),
            Some("Thu, Jan 1 '70 at 00")
        );
        assert_eq!(utc("EEEE MMMM", 0).as_deref(), Some("Thursday January"));
    }

    #[test]
    fn rejects_unknown_letters_and_open_quotes() {
        assert_eq!(utc("yyyy-q", 0), None);
        assert_eq!(utc("'open", 0), None);
    }

    #[test]
    fn date_to_string_uses_the_zone_abbreviation() {
        let zone = TimeZone::get("Europe/Zurich").unwrap();
        assert_eq!(
            java_date_to_string(1_759_668_558_000, &zone),
            "Sun Oct 05 14:49:18 CEST 2025"
        );
    }
}

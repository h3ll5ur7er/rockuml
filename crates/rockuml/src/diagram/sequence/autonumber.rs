//! Message numbering (`autonumber`): plain numbers formatted with a `java.text.DecimalFormat` pattern, or
//! dotted numbers like `1.2.3` (PlantUML's `AutoNumber` and `DottedNumber`).

/// A number like `1`, `10` or `1.1.1`: numbers and the separators between them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DottedNumber {
    numbers: Vec<i64>,
    separators: Vec<String>,
}

impl DottedNumber {
    pub(crate) fn parse(value: &str) -> Self {
        let mut numbers = Vec::new();
        let mut separators = Vec::new();
        let mut rest = value;
        while let Some(first) = rest.chars().next() {
            let is_digit = first.is_ascii_digit();
            let end = rest
                .find(|c: char| c.is_ascii_digit() != is_digit)
                .unwrap_or(rest.len());
            let (part, tail) = rest.split_at(end);
            if is_digit {
                numbers.push(part.parse().unwrap_or(0));
            } else {
                separators.push(part.to_owned());
            }
            rest = tail;
        }
        Self {
            numbers,
            separators,
        }
    }

    fn increment_minor(&mut self, step: i64) {
        if let Some(last) = self.numbers.last_mut() {
            *last += step;
        }
    }

    /// `autonumber inc`: the second last number, or the only one.
    pub(crate) fn increment_intermediate(&mut self) {
        let position = self.numbers.len().saturating_sub(2);
        self.increment_at(position);
    }

    /// Increments one number and restarts the ones after it at 1.
    pub(crate) fn increment_at(&mut self, position: usize) {
        if position >= self.numbers.len() {
            return;
        }
        self.numbers[position] += 1;
        for number in &mut self.numbers[position + 1..] {
            *number = 1;
        }
    }

    /// A single number goes through the format; dotted numbers are written bold.
    fn format(&self, format: &DecimalFormat) -> String {
        if self.numbers.len() == 1 && self.separators.is_empty() {
            return format.format(self.numbers[0]);
        }
        let mut text = String::from("<b>");
        for (index, number) in self.numbers.iter().enumerate() {
            text.push_str(&number.to_string());
            if let Some(separator) = self.separators.get(index) {
                text.push_str(separator);
            }
        }
        text.push_str("</b>");
        text
    }
}

/// The part of `java.text.DecimalFormat` numbering patterns use: literal text around a run of `0` (shown
/// digits) and `#` (digits shown only when needed), quotes escaping special characters.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DecimalFormat {
    prefix: String,
    suffix: String,
    minimum_digits: usize,
    grouping: Option<usize>,
}

/// An unreadable pattern, which makes PlantUML reject the command.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BadPattern;

impl DecimalFormat {
    pub(crate) fn new(pattern: &str) -> Result<Self, BadPattern> {
        let mut prefix = String::new();
        let mut suffix = String::new();
        let mut number = String::new();
        let mut phase = 0;
        let mut chars = pattern.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' {
                let mut quoted = String::new();
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    quoted.push('\'');
                } else {
                    loop {
                        match chars.next() {
                            None => return Err(BadPattern),
                            Some('\'') if chars.peek() == Some(&'\'') => {
                                chars.next();
                                quoted.push('\'');
                            }
                            Some('\'') => break,
                            Some(other) => quoted.push(other),
                        }
                    }
                }
                if phase == 1 {
                    phase = 2;
                }
                if phase == 0 {
                    prefix.push_str(&quoted);
                } else {
                    suffix.push_str(&quoted);
                }
                continue;
            }
            let numeric = matches!(c, '0' | '#' | ',' | '.');
            match phase {
                0 if numeric => {
                    phase = 1;
                    number.push(c);
                }
                0 => prefix.push(c),
                1 if numeric => number.push(c),
                _ => {
                    if c == ';' {
                        break;
                    }
                    phase = 2;
                    suffix.push(c);
                }
            }
        }
        let integer = number.split('.').next().unwrap_or_default();
        let grouping = integer
            .rfind(',')
            .map(|comma| integer.len() - comma - 1)
            .filter(|&size| size > 0);
        Ok(Self {
            prefix,
            suffix,
            minimum_digits: integer.chars().filter(|&c| c == '0').count(),
            grouping,
        })
    }

    pub(crate) fn format(&self, value: i64) -> String {
        let digits = value.unsigned_abs().to_string();
        let padded = format!("{digits:0>width$}", width = self.minimum_digits);
        let grouped = match self.grouping {
            Some(size) => group(&padded, size),
            None => padded,
        };
        let sign = if value < 0 { "-" } else { "" };
        format!("{}{sign}{grouped}{}", self.prefix, self.suffix)
    }
}

fn group(digits: &str, size: usize) -> String {
    let mut result = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(size) {
            result.push(',');
        }
        result.push(digit);
    }
    result
}

#[derive(Default)]
pub(crate) struct AutoNumber {
    running: bool,
    current: Option<DottedNumber>,
    increment: i64,
    format: Option<DecimalFormat>,
    last: String,
}

impl AutoNumber {
    pub(crate) fn go(&mut self, start: DottedNumber, increment: i64, format: DecimalFormat) {
        self.running = true;
        self.current = Some(start);
        self.increment = increment;
        self.format = Some(format);
    }

    pub(crate) fn stop(&mut self) {
        self.running = false;
    }

    /// Goes on numbering, with a new increment and format if given.
    pub(crate) fn resume(&mut self, increment: Option<i64>, format: Option<DecimalFormat>) {
        self.running = true;
        if let Some(increment) = increment {
            self.increment = increment;
        }
        if format.is_some() {
            self.format = format;
        }
    }

    pub(crate) fn current_mut(&mut self) -> Option<&mut DottedNumber> {
        self.current.as_mut()
    }

    pub(crate) fn next_message_number(&mut self) -> Option<String> {
        if !self.running {
            return None;
        }
        let current = self.current.as_mut()?;
        self.last = current.format(self.format.as_ref()?);
        current.increment_minor(self.increment);
        Some(self.last.clone())
    }

    /// The last number given, without bold tags, for `%autonumber%`.
    pub(crate) fn current_message_number(&self) -> String {
        self.last.replace("<b>", "").replace("</b>", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_formats_pad_and_keep_literal_text() {
        let format = |pattern: &str, value| DecimalFormat::new(pattern).unwrap().format(value);
        assert_eq!(format("<b>0</b>", 7), "<b>7</b>");
        assert_eq!(format("<b>[000]", 10), "<b>[010]");
        assert_eq!(format("<b>(<u>##</u>)", 15), "<b>(<u>15</u>)");
        assert_eq!(
            format("<font color=red><b>Message 0  ", 40),
            "<font color=red><b>Message 40  "
        );
        assert_eq!(format("'#'0", 3), "#3");
    }

    #[test]
    fn dotted_numbers_increment_the_last_part_and_render_bold() {
        let mut number = DottedNumber::parse("1.1.1");
        let format = DecimalFormat::new("<b>0</b>").unwrap();
        assert_eq!(number.format(&format), "<b>1.1.1</b>");
        number.increment_minor(1);
        number.increment_at(0);
        assert_eq!(number.format(&format), "<b>2.1.1</b>");
        number.increment_intermediate();
        assert_eq!(number.format(&format), "<b>2.2.1</b>");
    }

    #[test]
    fn numbering_stops_and_resumes() {
        let mut autonumber = AutoNumber::default();
        assert_eq!(autonumber.next_message_number(), None);
        autonumber.go(
            DottedNumber::parse("10"),
            10,
            DecimalFormat::new("<b>0</b>").unwrap(),
        );
        assert_eq!(
            autonumber.next_message_number().as_deref(),
            Some("<b>10</b>")
        );
        autonumber.stop();
        assert_eq!(autonumber.next_message_number(), None);
        autonumber.resume(Some(1), None);
        assert_eq!(
            autonumber.next_message_number().as_deref(),
            Some("<b>20</b>")
        );
        assert_eq!(autonumber.current_message_number(), "20");
    }
}

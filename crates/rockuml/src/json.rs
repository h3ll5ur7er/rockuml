//! The JSON model PlantUML uses (a fork of minimal-json). Its parsing leniency and its compact text form
//! both show up in output, so it is ported rather than replaced by a general-purpose JSON crate.

use std::fmt::{self, Write};

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    /// Kept as written: minimal-json never reformats numbers.
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(JsonObject),
}

/// Members in insertion order. Duplicate names are allowed; lookups see the last one, like minimal-json.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JsonObject {
    members: Vec<(String, JsonValue)>,
}

impl JsonObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: impl Into<String>, value: JsonValue) {
        self.members.push((name.into(), value));
    }

    pub fn set(&mut self, name: &str, value: JsonValue) {
        match self.index_of(name) {
            Some(index) => self.members[index].1 = value,
            None => self.add(name, value),
        }
    }

    pub fn remove(&mut self, name: &str) {
        if let Some(index) = self.index_of(name) {
            self.members.remove(index);
        }
    }

    pub fn get(&self, name: &str) -> Option<&JsonValue> {
        self.index_of(name).map(|index| &self.members[index].1)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.index_of(name).is_some()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.members.iter().map(|(name, _)| name.as_str())
    }

    pub fn members(&self) -> impl Iterator<Item = (&str, &JsonValue)> {
        self.members.iter().map(|(name, value)| (name.as_str(), value))
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn merge(&mut self, other: &JsonObject) {
        for (name, value) in other.members() {
            self.set(name, value.clone());
        }
    }

    pub fn deep_merge(&mut self, other: &JsonObject) {
        for (name, value) in other.members() {
            let merged = match (value, self.get(name)) {
                (JsonValue::Object(incoming), Some(JsonValue::Object(existing))) => {
                    let mut existing = existing.clone();
                    existing.deep_merge(incoming);
                    JsonValue::Object(existing)
                }
                _ => value.clone(),
            };
            self.set(name, merged);
        }
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.members.iter().rposition(|(member, _)| member == name)
    }
}

impl JsonValue {
    pub fn from_int(value: i32) -> Self {
        JsonValue::Number(value.to_string())
    }

    pub fn is_string(&self) -> bool {
        matches!(self, JsonValue::String(_))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(string) => Some(string),
            _ => None,
        }
    }

    /// Counts members or elements; other values have no size.
    pub fn container_len(&self) -> Option<usize> {
        match self {
            JsonValue::Array(values) => Some(values.len()),
            JsonValue::Object(object) => Some(object.len()),
            _ => None,
        }
    }
}

impl fmt::Display for JsonValue {
    /// The compact form minimal-json writes: no spaces, `\u` escapes only for control characters.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonValue::Null => f.write_str("null"),
            JsonValue::Bool(value) => write!(f, "{value}"),
            JsonValue::Number(number) => f.write_str(number),
            JsonValue::String(string) => write_json_string(f, string),
            JsonValue::Array(values) => {
                f.write_char('[')?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        f.write_char(',')?;
                    }
                    write!(f, "{value}")?;
                }
                f.write_char(']')
            }
            JsonValue::Object(object) => {
                f.write_char('{')?;
                for (index, (name, value)) in object.members().enumerate() {
                    if index > 0 {
                        f.write_char(',')?;
                    }
                    write_json_string(f, name)?;
                    write!(f, ":{value}")?;
                }
                f.write_char('}')
            }
        }
    }
}

fn write_json_string(f: &mut fmt::Formatter<'_>, string: &str) -> fmt::Result {
    f.write_char('"')?;
    for c in string.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            '\u{2028}' => f.write_str("\\u2028")?,
            '\u{2029}' => f.write_str("\\u2029")?,
            c if c <= '\u{1F}' => write!(f, "\\u{:04x}", c as u32)?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}:{}", self.message, self.line, self.column)
    }
}

pub fn parse(text: &str) -> Result<JsonValue, ParseError> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        index: 0,
        line: 1,
        line_start: 0,
    };
    parser.skip_white_space();
    let value = parser.read_value(0)?;
    parser.skip_white_space();
    if parser.current().is_some() {
        return Err(parser.error("Unexpected character"));
    }
    Ok(value)
}

const MAX_NESTING_LEVEL: usize = 1000;

struct Parser {
    chars: Vec<char>,
    index: usize,
    line: usize,
    line_start: usize,
}

impl Parser {
    fn current(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn advance(&mut self) {
        if self.current() == Some('\n') {
            self.line += 1;
            self.line_start = self.index + 1;
        }
        self.index += 1;
    }

    fn read_char(&mut self, expected: char) -> bool {
        if self.current() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn read_required_char(&mut self, expected: char) -> Result<(), ParseError> {
        if self.read_char(expected) {
            Ok(())
        } else {
            Err(self.expected(&format!("'{expected}'")))
        }
    }

    fn read_digit(&mut self) -> bool {
        if self.current().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_white_space(&mut self) {
        while matches!(self.current(), Some(' ' | '\t' | '\n' | '\r')) {
            self.advance();
        }
    }

    fn read_value(&mut self, nesting: usize) -> Result<JsonValue, ParseError> {
        match self.current() {
            Some('n') => self.read_literal("null", JsonValue::Null),
            Some('t') => self.read_literal("true", JsonValue::Bool(true)),
            Some('f') => self.read_literal("false", JsonValue::Bool(false)),
            Some('"') => Ok(JsonValue::String(self.read_string()?)),
            Some('[') => self.read_array(nesting + 1),
            Some('{') => self.read_object(nesting + 1),
            Some('-' | '0'..='9') => self.read_number(),
            _ => Err(self.expected("value")),
        }
    }

    fn read_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, ParseError> {
        self.advance();
        for expected in literal.chars().skip(1) {
            self.read_required_char(expected)?;
        }
        Ok(value)
    }

    fn read_array(&mut self, nesting: usize) -> Result<JsonValue, ParseError> {
        self.advance();
        if nesting > MAX_NESTING_LEVEL {
            return Err(self.error("Nesting too deep"));
        }
        let mut values = Vec::new();
        self.skip_white_space();
        if self.read_char(']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            self.skip_white_space();
            values.push(self.read_value(nesting)?);
            self.skip_white_space();
            if !self.read_char(',') {
                break;
            }
        }
        if !self.read_char(']') {
            return Err(self.expected("',' or ']'"));
        }
        Ok(JsonValue::Array(values))
    }

    fn read_object(&mut self, nesting: usize) -> Result<JsonValue, ParseError> {
        self.advance();
        if nesting > MAX_NESTING_LEVEL {
            return Err(self.error("Nesting too deep"));
        }
        let mut object = JsonObject::new();
        self.skip_white_space();
        if self.read_char('}') {
            return Ok(JsonValue::Object(object));
        }
        loop {
            self.skip_white_space();
            if self.read_char('/') {
                self.skip_comment()?;
            }
            if self.current() != Some('"') {
                return Err(self.expected("name"));
            }
            let name = self.read_string()?;
            self.skip_white_space();
            if !self.read_char(':') {
                return Err(self.expected("':'"));
            }
            self.skip_white_space();
            let value = self.read_value(nesting)?;
            object.add(name, value);
            self.skip_white_space();
            if self.read_char('/') {
                self.skip_comment()?;
            }
            if !self.read_char(',') {
                break;
            }
        }
        if !self.read_char('}') {
            return Err(self.expected("',' or '}'"));
        }
        Ok(JsonValue::Object(object))
    }

    /// PlantUML's fork accepts `// comments` between object members.
    fn skip_comment(&mut self) -> Result<(), ParseError> {
        if !self.read_char('/') {
            return Err(self.expected("Error in comment"));
        }
        while !matches!(self.current(), Some('\n' | '\r') | None) {
            self.advance();
        }
        self.skip_white_space();
        Ok(())
    }

    fn read_string(&mut self) -> Result<String, ParseError> {
        self.advance();
        let mut string = String::new();
        let mut utf16_units = Vec::new();
        loop {
            if self.current() != Some('\\') && !utf16_units.is_empty() {
                string.push_str(&String::from_utf16_lossy(&utf16_units));
                utf16_units.clear();
            }
            match self.current() {
                Some('"') => break,
                Some('\\') => {
                    self.advance();
                    // `\u` escapes are UTF-16 units: a surrogate pair spans two escapes.
                    utf16_units.push(self.read_escape()?);
                }
                Some(c) if c >= '\u{20}' => {
                    string.push(c);
                    self.advance();
                }
                _ => return Err(self.expected("valid string character")),
            }
        }
        self.advance();
        Ok(string)
    }

    fn read_escape(&mut self) -> Result<u16, ParseError> {
        let escaped = match self.current() {
            Some(c @ ('"' | '/' | '\\')) => c as u16,
            Some('b') => 0x8,
            Some('f') => 0xC,
            Some('n') => 0xA,
            Some('r') => 0xD,
            Some('t') => 0x9,
            Some('u') => {
                let mut code = 0;
                for _ in 0..4 {
                    self.advance();
                    let digit = self
                        .current()
                        .and_then(|c| c.to_digit(16))
                        .ok_or_else(|| self.expected("hexadecimal digit"))?;
                    code = code * 16 + u16::try_from(digit).unwrap();
                }
                code
            }
            _ => return Err(self.expected("valid escape sequence")),
        };
        self.advance();
        Ok(escaped)
    }

    fn read_number(&mut self) -> Result<JsonValue, ParseError> {
        let start = self.index;
        self.read_char('-');
        let first_digit = self.current();
        if !self.read_digit() {
            return Err(self.expected("digit"));
        }
        if first_digit != Some('0') {
            while self.read_digit() {}
        }
        if self.read_char('.') {
            if !self.read_digit() {
                return Err(self.expected("digit"));
            }
            while self.read_digit() {}
        }
        if self.read_char('e') || self.read_char('E') {
            if !self.read_char('+') {
                self.read_char('-');
            }
            if !self.read_digit() {
                return Err(self.expected("digit"));
            }
            while self.read_digit() {}
        }
        Ok(JsonValue::Number(self.chars[start..self.index].iter().collect()))
    }

    fn expected(&self, what: &str) -> ParseError {
        if self.current().is_none() {
            self.error("Unexpected end of input")
        } else {
            self.error(&format!("Expected {what}"))
        }
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_owned(),
            line: self.line,
            column: self.index.min(self.chars.len()) - self.line_start + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_in_compact_form() {
        let text = r#"{"a":[1,2.50,-3e2],"b":{"c":null,"d":true},"e":"x\"y"}"#;
        assert_eq!(parse(text).unwrap().to_string(), text);
    }

    #[test]
    fn whitespace_is_dropped_and_control_characters_escaped() {
        let value = parse("[ \"tab\\there\" , \"\\u0001\" ]").unwrap();
        assert_eq!(value.to_string(), r#"["tab\there","\u0001"]"#);
    }

    #[test]
    fn surrogate_pair_escapes_form_one_character() {
        assert_eq!(parse(r#""😀!""#).unwrap(), JsonValue::String("😀!".into()));
    }

    #[test]
    fn comments_between_object_members_are_ignored() {
        let value = parse("{ // first\n \"a\": 1 // trailing\n }").unwrap();
        assert_eq!(value.to_string(), r#"{"a":1}"#);
    }

    #[test]
    fn duplicate_names_resolve_to_the_last_member() {
        let JsonValue::Object(mut object) = parse(r#"{"k":1,"k":2}"#).unwrap() else {
            panic!("expected an object");
        };
        assert_eq!(object.get("k"), Some(&JsonValue::Number("2".into())));
        object.remove("k");
        assert_eq!(object.get("k"), Some(&JsonValue::Number("1".into())));
    }

    #[test]
    fn errors_report_line_and_column() {
        let error = parse("{\n  \"a\" 1}").unwrap_err();
        assert_eq!((error.message.as_str(), error.line, error.column), ("Expected ':'", 2, 7));
        assert_eq!(parse("[1,").unwrap_err().message, "Unexpected end of input");
    }

    #[test]
    fn deep_merge_combines_nested_objects() {
        let JsonValue::Object(mut base) = parse(r#"{"a":{"x":1},"b":2}"#).unwrap() else { panic!() };
        let JsonValue::Object(patch) = parse(r#"{"a":{"y":3},"b":4}"#).unwrap() else { panic!() };
        base.deep_merge(&patch);
        assert_eq!(JsonValue::Object(base).to_string(), r#"{"a":{"x":1,"y":3},"b":4}"#);
    }
}

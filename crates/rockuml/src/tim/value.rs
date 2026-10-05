//! Values of preprocessor expressions: 32-bit integers, strings or JSON.

use std::cmp::Ordering;
use std::fmt;

use crate::json::JsonValue;

#[derive(Clone, Debug, PartialEq)]
pub enum TValue {
    Int(i32),
    String(String),
    Json(JsonValue),
}

impl TValue {
    pub fn from_bool(value: bool) -> Self {
        TValue::Int(i32::from(value))
    }

    pub fn string(value: impl Into<String>) -> Self {
        TValue::String(value.into())
    }

    pub fn is_number(&self) -> bool {
        matches!(self, TValue::Int(_))
    }

    pub fn to_int(&self) -> i32 {
        match self {
            TValue::Int(value) => *value,
            _ => 0,
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            TValue::Int(value) => *value != 0,
            _ => !self.to_string().is_empty(),
        }
    }

    pub fn as_json(&self) -> Option<&JsonValue> {
        match self {
            TValue::Json(json) => Some(json),
            _ => None,
        }
    }

    pub fn to_json_value(&self) -> JsonValue {
        match self {
            TValue::Int(value) => JsonValue::from_int(*value),
            TValue::String(string) => JsonValue::String(string.clone()),
            TValue::Json(json) => json.clone(),
        }
    }

    /// `+` adds numbers and concatenates anything else.
    pub fn add(&self, other: &TValue) -> TValue {
        match (self, other) {
            (TValue::Int(a), TValue::Int(b)) => TValue::Int(a.wrapping_add(*b)),
            _ => TValue::String(format!("{self}{other}")),
        }
    }

    /// `-` on non-numbers concatenates, as PlantUML does.
    pub fn minus(&self, other: &TValue) -> TValue {
        match (self, other) {
            (TValue::Int(a), TValue::Int(b)) => TValue::Int(a.wrapping_sub(*b)),
            _ => TValue::String(format!("{self}{other}")),
        }
    }

    pub fn multiply(&self, other: &TValue) -> TValue {
        match (self, other) {
            (TValue::Int(a), TValue::Int(b)) => TValue::Int(a.wrapping_mul(*b)),
            _ => TValue::String(format!("{self}*{other}")),
        }
    }

    /// `None` stands for Java's `ArithmeticException` on integer division by zero.
    pub fn divided_by(&self, other: &TValue) -> Option<TValue> {
        match (self, other) {
            (TValue::Int(_), TValue::Int(0)) => None,
            (TValue::Int(a), TValue::Int(b)) => Some(TValue::Int(a.wrapping_div(*b))),
            _ => Some(TValue::String(format!("{self}/{other}"))),
        }
    }

    /// Numbers compare numerically; anything else compares as text, by UTF-16 code units like Java.
    pub fn compare(&self, other: &TValue) -> Ordering {
        match (self, other) {
            (TValue::Int(a), TValue::Int(b)) => a.cmp(b),
            _ => self.to_string().encode_utf16().cmp(other.to_string().encode_utf16()),
        }
    }
}

impl fmt::Display for TValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TValue::Int(value) => write!(f, "{value}"),
            TValue::String(string) => f.write_str(string),
            TValue::Json(JsonValue::String(string)) => f.write_str(string),
            TValue::Json(json) => write!(f, "{json}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_on_numbers_wraps_like_java_int() {
        assert_eq!(TValue::Int(i32::MAX).add(&TValue::Int(1)), TValue::Int(i32::MIN));
        assert_eq!(TValue::Int(7).divided_by(&TValue::Int(2)), Some(TValue::Int(3)));
        assert_eq!(TValue::Int(-7).divided_by(&TValue::Int(2)), Some(TValue::Int(-3)));
        assert_eq!(TValue::Int(1).divided_by(&TValue::Int(0)), None);
    }

    #[test]
    fn operators_on_text_concatenate() {
        assert_eq!(TValue::string("a").add(&TValue::Int(1)).to_string(), "a1");
        assert_eq!(TValue::string("a").minus(&TValue::string("b")).to_string(), "ab");
        assert_eq!(TValue::string("a").multiply(&TValue::Int(2)).to_string(), "a*2");
    }

    #[test]
    fn truthiness_is_non_zero_or_non_empty() {
        assert!(TValue::Int(-1).to_bool());
        assert!(!TValue::string("").to_bool());
        assert!(TValue::string("0").to_bool());
    }

    #[test]
    fn json_strings_print_without_quotes() {
        assert_eq!(TValue::Json(JsonValue::String("x".into())).to_string(), "x");
        assert_eq!(TValue::Json(crate::json::parse("[\"x\"]").unwrap()).to_string(), "[\"x\"]");
    }
}

//! The subset of HCL PlantUML reads: named blocks of attributes whose values are strings, lists, maps and
//! function calls, all kept as text (PlantUML's `hcl` package).

use crate::json::{JsonObject, JsonValue};

/// The document as JSON, or `None` where PlantUML fails to read it (PlantUML's `HclParser`).
pub(super) fn parse(lines: &[String]) -> Option<JsonValue> {
    let terms = tokenize(hcl_source(lines))?;
    parse_me(&mut terms.into_iter())
}

/// The characters of the lines that are not comments (PlantUML's `HclSource`). Lines are joined without a separator, so a value at the
/// end of an unindented line runs into the next line's name.
fn hcl_source(lines: &[String]) -> impl Iterator<Item = char> + '_ {
    lines
        .iter()
        .filter(|line| !crate::java::trim(line).starts_with('#'))
        .flat_map(|line| line.chars())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SymbolType {
    StringSimple,
    StringQuoted,
    FunctionName,
    SquareBracketOpen,
    SquareBracketClose,
    CurlyBracketOpen,
    CurlyBracketClose,
    ParenthesisOpen,
    ParenthesisClose,
    TwoPoints,
    Equals,
    Comma,
    Space,
}

impl SymbolType {
    fn of(c: char) -> Option<Self> {
        Some(match c {
            _ if crate::java::is_space_char(c) => Self::Space,
            '{' => Self::CurlyBracketOpen,
            '}' => Self::CurlyBracketClose,
            '[' => Self::SquareBracketOpen,
            ']' => Self::SquareBracketClose,
            '(' => Self::ParenthesisOpen,
            ')' => Self::ParenthesisClose,
            '=' => Self::Equals,
            ',' => Self::Comma,
            ':' => Self::TwoPoints,
            _ => return None,
        })
    }
}

#[derive(Debug)]
struct HclTerm {
    symbol: SymbolType,
    /// The text of strings and function names.
    data: String,
}

impl HclTerm {
    fn new(symbol: SymbolType) -> Self {
        Self::with_data(symbol, String::new())
    }

    fn with_data(symbol: SymbolType, data: String) -> Self {
        Self { symbol, data }
    }
}

type Terms = std::vec::IntoIter<HclTerm>;

/// Text between symbols becomes a simple string, or a function name before `(`; text left at the end is dropped.
fn tokenize(mut it: impl Iterator<Item = char>) -> Option<Vec<HclTerm>> {
    let mut terms = Vec::new();
    let mut pending_string = String::new();
    while let Some(c) = it.next() {
        let symbol = SymbolType::of(c);
        if symbol == Some(SymbolType::ParenthesisOpen) {
            if pending_string.is_empty() {
                return None;
            }
            terms.push(HclTerm::with_data(
                SymbolType::FunctionName,
                std::mem::take(&mut pending_string),
            ));
        } else if symbol.is_some() && !pending_string.is_empty() {
            terms.push(HclTerm::with_data(
                SymbolType::StringSimple,
                std::mem::take(&mut pending_string),
            ));
        }
        match symbol {
            Some(SymbolType::Space) => {}
            Some(symbol) => terms.push(HclTerm::new(symbol)),
            None if c == '"' => terms.push(HclTerm::with_data(
                SymbolType::StringQuoted,
                eat_until_double_quote(&mut it)?,
            )),
            None => pending_string.push(c),
        }
    }
    Some(terms)
}

/// A backslash keeps the character after it; an unclosed string runs to the end.
fn eat_until_double_quote(it: &mut impl Iterator<Item = char>) -> Option<String> {
    let mut result = String::new();
    while let Some(c) = it.next() {
        match c {
            '\\' => result.push(it.next()?),
            '"' => return Some(result),
            _ => result.push(c),
        }
    }
    Some(result)
}

/// The blocks by name, a later block replacing an earlier one of the same name; a lone block is its content.
fn parse_me(it: &mut Terms) -> Option<JsonValue> {
    let mut map = JsonObject::new();
    while it.len() > 0 {
        let (name, data) = get_module_or_something(it)?;
        map.set(&name, JsonValue::Object(data));
    }
    if map.len() == 1 {
        return map.members().next().map(|(_, data)| data.clone());
    }
    Some(JsonValue::Object(map))
}

/// A block's name, its labels quoted as written, and its content.
fn get_module_or_something(it: &mut Terms) -> Option<(String, JsonObject)> {
    let mut name = String::new();
    loop {
        let current = it.next()?;
        match current.symbol {
            SymbolType::StringQuoted => {
                name.push('"');
                name.push_str(&current.data);
                name.push_str("\" ");
            }
            SymbolType::StringSimple => {
                name.push_str(&current.data);
                name.push(' ');
            }
            SymbolType::CurlyBracketOpen => {
                return Some((crate::java::trim(&name).to_owned(), get_bracket_data(it)?));
            }
            _ => return None,
        }
    }
}

/// What `get_value` reads: a value, or the comma or parenthesis that ends a function's argument.
enum Value {
    Json(JsonValue),
    Separator(SymbolType),
}

/// A call without arguments is the text `name()`; with arguments, a map from `name()` to them.
fn get_function_data(function_name: &str, it: &mut Terms) -> Option<JsonValue> {
    let mut args = Vec::new();
    if it.next()?.symbol != SymbolType::ParenthesisOpen {
        return None;
    }
    loop {
        match get_value(it)? {
            Value::Separator(SymbolType::ParenthesisClose) => {
                let name = format!("{function_name}()");
                if args.is_empty() {
                    return Some(JsonValue::String(name));
                }
                let mut result = JsonObject::new();
                result.add(name, JsonValue::Array(args));
                return Some(JsonValue::Object(result));
            }
            Value::Separator(_) => {}
            Value::Json(value) => args.push(value),
        }
    }
}

/// The attributes of a block or map, up to its `}`.
fn get_bracket_data(it: &mut Terms) -> Option<JsonObject> {
    let mut result = JsonObject::new();
    loop {
        let current = it.next()?;
        match current.symbol {
            SymbolType::CurlyBracketClose => return Some(result),
            SymbolType::StringSimple | SymbolType::StringQuoted => {
                let next = it.next()?;
                if !matches!(next.symbol, SymbolType::Equals | SymbolType::TwoPoints) {
                    return None;
                }
                let Value::Json(value) = get_value(it)? else {
                    return None;
                };
                result.add(current.data, value);
            }
            _ => return None,
        }
    }
}

fn get_value(it: &mut Terms) -> Option<Value> {
    let current = it.next()?;
    Some(match current.symbol {
        SymbolType::Comma | SymbolType::ParenthesisClose => Value::Separator(current.symbol),
        SymbolType::StringQuoted | SymbolType::StringSimple => {
            Value::Json(JsonValue::String(current.data))
        }
        SymbolType::SquareBracketOpen => Value::Json(get_array(it)?),
        SymbolType::CurlyBracketOpen => Value::Json(JsonValue::Object(get_bracket_data(it)?)),
        SymbolType::FunctionName => Value::Json(get_function_data(&current.data, it)?),
        _ => return None,
    })
}

/// Only quoted strings and maps make it into a list; anything else in it is skipped.
fn get_array(it: &mut Terms) -> Option<JsonValue> {
    let mut result = Vec::new();
    loop {
        let current = it.next()?;
        match current.symbol {
            SymbolType::CurlyBracketOpen => {
                result.push(JsonValue::Object(get_bracket_data(it)?));
            }
            SymbolType::SquareBracketClose => return Some(JsonValue::Array(result)),
            SymbolType::StringQuoted => result.push(JsonValue::String(current.data)),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(lines: &[&str]) -> Option<String> {
        let lines: Vec<String> = lines.iter().map(|&line| line.to_owned()).collect();
        parse(&lines).map(|value| value.to_string())
    }

    #[test]
    fn a_lone_block_is_its_attributes_as_text() {
        assert_eq!(
            json(&[
                "resource \"a\" \"b\" {",
                "  n = 1",
                "  s: \"x \\\"y\\\"\"",
                "}"
            ])
            .as_deref(),
            Some(r#"{"n":"1","s":"x \"y\""}"#)
        );
    }

    #[test]
    fn blocks_are_named_by_their_labels_and_replaced_by_later_ones() {
        assert_eq!(
            json(&[
                "a \"x\" {",
                "}",
                "b {",
                "  n = 1",
                "}",
                "a \"x\" {",
                "  m = 2",
                "}"
            ])
            .as_deref(),
            Some(r#"{"a \"x\"":{"m":"2"},"b":{"n":"1"}}"#)
        );
    }

    #[test]
    fn lists_keep_quoted_strings_and_maps_only() {
        assert_eq!(
            json(&["b {", "  l = [\"p\", 2, { k = v }]", "}"]).as_deref(),
            Some(r#"{"l":["p",{"k":"v"}]}"#)
        );
    }

    #[test]
    fn functions_map_their_name_to_their_arguments() {
        assert_eq!(
            json(&["b {", "  t = now()", "  f = f(a, \"b\", g(c))", "}"]).as_deref(),
            Some(r#"{"t":"now()","f":{"f()":["a","b",{"g()":["c"]}]}}"#)
        );
    }

    #[test]
    fn comment_lines_are_skipped() {
        assert_eq!(
            json(&["# c", "b {", "  # c", "  n = 1", "}"]).as_deref(),
            Some(r#"{"n":"1"}"#)
        );
    }

    #[test]
    fn what_plantuml_cannot_read_gives_nothing() {
        assert_eq!(json(&["b {", "  inner {", "  }", "}"]), None);
        assert_eq!(json(&["b {", "n = 1", "m = 2", "}"]), None);
        assert_eq!(json(&["b {", "  n = (1)", "}"]), None);
        assert_eq!(json(&["b {", "  n = 1"]), None);
        assert_eq!(json(&["a = 1"]), None);
    }

    #[test]
    fn nothing_is_an_empty_map() {
        assert_eq!(json(&[]).as_deref(), Some("{}"));
    }
}

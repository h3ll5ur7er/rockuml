//! The subset of YAML PlantUML reads: block maps and lists by indentation, flow lists, `|` blocks and plain
//! scalars, all values kept as text (PlantUML's `yaml.parser` package).

use crate::json::{JsonObject, JsonValue};

/// The document as JSON, or `None` where PlantUML fails to read it.
pub(super) fn parse(lines: &[&str]) -> Option<JsonValue> {
    let arena = parse_monomorphs(lines)?;
    monomorph_to_json(&arena, Monomorphs::ROOT)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum YamlLineType {
    EmptyLine,
    NoKeyOnlyText,
    KeyOnly,
    KeyAndValue,
    /// `key: [a, b]`.
    KeyAndFlowSequence,
    /// `key: |`.
    KeyAndBlockStyle,
    /// `key: >`, which PlantUML does not read.
    KeyAndFoldedStyle,
    /// `- value`.
    PlainElementList,
    /// `-` alone.
    PlainDash,
}

#[derive(Debug)]
struct YamlLine {
    kind: YamlLineType,
    indent: usize,
    key: String,
    value: String,
    values: Vec<String>,
    list_item: bool,
}

impl YamlLine {
    fn build(line: &str) -> Self {
        let line = line.replace('\t', "    ");
        let count = line.bytes().take_while(|&byte| byte == b' ').count();
        let trimmed_line = remove_yaml_comment(crate::java::trim(&line[count..]));
        let simple = |kind, indent, value: Option<String>, list_item| Self {
            kind,
            indent,
            key: String::new(),
            value: value.unwrap_or_default(),
            values: Vec::new(),
            list_item,
        };
        if trimmed_line.is_empty() {
            return simple(YamlLineType::EmptyLine, 0, None, false);
        }
        if trimmed_line == "-" {
            return simple(YamlLineType::PlainDash, count + 1, None, true);
        }
        let list_item = trimmed_line.starts_with("- ");
        let (count, trimmed_line) = if list_item {
            (count + 2, &trimmed_line[2..])
        } else {
            (count, trimmed_line)
        };
        let Some(colon_index) = find_colon_separator(trimmed_line) else {
            let kind = if list_item {
                YamlLineType::PlainElementList
            } else {
                YamlLineType::NoKeyOnlyText
            };
            return simple(
                kind,
                count,
                Some(unquote(trimmed_line).to_owned()),
                list_item,
            );
        };
        let raw_key = crate::java::trim(&trimmed_line[..colon_index]);
        let raw_value = crate::java::trim(&trimmed_line[colon_index + 1..]);
        let mut result = Self {
            kind: YamlLineType::KeyAndValue,
            indent: count,
            key: unquote(raw_key).to_owned(),
            value: unquote(raw_value).to_owned(),
            values: Vec::new(),
            list_item,
        };
        if raw_value.is_empty() {
            result.kind = YamlLineType::KeyOnly;
        } else if raw_value == "|" {
            result.kind = YamlLineType::KeyAndBlockStyle;
        } else if raw_value == ">" {
            result.kind = YamlLineType::KeyAndFoldedStyle;
        } else if raw_value.starts_with('[') && raw_value.ends_with(']') {
            result.kind = YamlLineType::KeyAndFlowSequence;
            result.values = to_list(&raw_value[1..raw_value.len() - 1]);
        }
        result
    }
}

/// The colon between key and value, skipping colons in quotes.
fn find_colon_separator(line: &str) -> Option<usize> {
    let mut in_quote = None;
    for (index, c) in line.char_indices() {
        match in_quote {
            Some(quote) if c == quote => in_quote = None,
            None if c == '"' || c == '\'' => in_quote = Some(c),
            None if c == ':' => return Some(index),
            Some(_) | None => {}
        }
    }
    None
}

/// The items of a flow list, quoted ones kept as written, others trimmed; `\` escapes a character.
fn to_list(raw_value: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quoted_string = None;
    let mut field_start_with_quote = false;
    let mut chars = raw_value.chars();
    let field = |current: &str, quoted: bool| {
        if quoted {
            current.to_owned()
        } else {
            crate::java::trim(current).to_owned()
        }
    };
    while let Some(c) = chars.next() {
        match in_quoted_string {
            Some(quote) => {
                if c == '\\' {
                    current.extend(chars.next());
                } else if c == quote {
                    in_quoted_string = None;
                } else {
                    current.push(c);
                }
            }
            None => {
                if !field_start_with_quote
                    && crate::java::trim(&current).is_empty()
                    && (c == '\'' || c == '"')
                {
                    in_quoted_string = Some(c);
                    field_start_with_quote = true;
                    current.clear();
                } else if c == ',' {
                    result.push(field(&current, field_start_with_quote));
                    current.clear();
                    field_start_with_quote = false;
                } else if c == '\\' {
                    current.extend(chars.next());
                } else {
                    current.push(c);
                }
            }
        }
    }
    result.push(field(&current, field_start_with_quote));
    result
}

fn unquote(text: &str) -> &str {
    if text.len() >= 2
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('\'') && text.ends_with('\'')))
    {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

/// The line without a ` #` comment outside quotes, or nothing for a whole-line comment.
fn remove_yaml_comment(s: &str) -> &str {
    if s.starts_with('#') {
        return "";
    }
    let mut in_quote_char = None;
    let bytes = s.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if c == b'\'' || c == b'"' {
            match in_quote_char {
                None => in_quote_char = Some(c),
                Some(quote) if quote == c => in_quote_char = None,
                Some(_) => {}
            }
        }
        if in_quote_char.is_none() && i + 1 < bytes.len() && c == b' ' && bytes[i + 1] == b'#' {
            return &s[..i];
        }
    }
    s
}

/// A YAML node whose kind is set by its first use: a scalar, a list or a map (PlantUML's `Monomorph`).
#[derive(Debug)]
enum Monomorph {
    Undeterminate,
    Scalar(String),
    List(Vec<MonomorphId>),
    /// In insertion order; a repeated key replaces the value in place.
    Map(Vec<(String, MonomorphId)>),
}

type MonomorphId = usize;

#[derive(Debug)]
struct Monomorphs {
    nodes: Vec<Monomorph>,
}

impl Monomorphs {
    const ROOT: MonomorphId = 0;

    fn new() -> Self {
        Self {
            nodes: vec![Monomorph::Undeterminate],
        }
    }

    fn create(&mut self, node: Monomorph) -> MonomorphId {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn scalar(&mut self, value: &str) -> MonomorphId {
        self.create(Monomorph::Scalar(value.to_owned()))
    }

    fn list(&mut self, values: &[String]) -> MonomorphId {
        let items = values.iter().map(|value| self.scalar(value)).collect();
        self.create(Monomorph::List(items))
    }

    fn is_list(&self, id: MonomorphId) -> bool {
        matches!(self.nodes[id], Monomorph::List(_))
    }

    /// `None` where PlantUML throws: the node is already something else.
    fn add_in_list(&mut self, id: MonomorphId, element: MonomorphId) -> Option<()> {
        match &mut self.nodes[id] {
            node @ Monomorph::Undeterminate => *node = Monomorph::List(vec![element]),
            Monomorph::List(items) => items.push(element),
            _ => return None,
        }
        Some(())
    }

    fn put_in_map(&mut self, id: MonomorphId, key: &str, value: MonomorphId) -> Option<()> {
        match &mut self.nodes[id] {
            node @ Monomorph::Undeterminate => {
                *node = Monomorph::Map(vec![(key.to_owned(), value)]);
            }
            Monomorph::Map(entries) => {
                match entries.iter_mut().find(|(existing, _)| existing == key) {
                    Some(entry) => entry.1 = value,
                    None => entries.push((key.to_owned(), value)),
                }
            }
            _ => return None,
        }
        Some(())
    }
}

/// Builds the node tree from the lines' events, `nodes` holding the open containers and `indents` the
/// indentation of each depth (PlantUML's `YamlBuilder`).
struct YamlBuilder {
    arena: Monomorphs,
    indents: Vec<usize>,
    nodes: Vec<MonomorphId>,
}

impl YamlBuilder {
    fn new() -> Self {
        Self {
            arena: Monomorphs::new(),
            indents: Vec::new(),
            nodes: vec![Monomorphs::ROOT],
        }
    }

    fn get_last(&self) -> Option<MonomorphId> {
        self.nodes.last().copied()
    }

    fn adjust_indentation(&mut self, indent: usize) -> Option<()> {
        let Some(&last_indent) = self.indents.last() else {
            self.indents.push(indent);
            return Some(());
        };
        if indent > last_indent {
            self.indents.push(indent);
            return Some(());
        }
        while self.indents.last().is_some_and(|&last| indent < last) {
            self.indents.pop();
            self.nodes.pop()?;
            if self.arena.is_list(self.get_last()?) {
                self.nodes.pop()?;
            }
        }
        Some(())
    }

    /// Whether the last open node is the last item of the list open below it.
    fn is_array_already_there(&self) -> bool {
        let [.., potential_list, last] = self.nodes.as_slice() else {
            return false;
        };
        match &self.arena.nodes[*potential_list] {
            Monomorph::List(items) => items.last() == Some(last),
            _ => false,
        }
    }

    fn close_list_item(&mut self) {
        if self.is_array_already_there() {
            self.nodes.pop();
        }
    }

    fn on_list_item_plain_dash(&mut self) -> Option<()> {
        self.close_list_item();
        let new_element = self.arena.create(Monomorph::Undeterminate);
        self.arena.add_in_list(self.get_last()?, new_element)?;
        self.nodes.push(new_element);
        Some(())
    }

    fn on_key_and_value(&mut self, key: &str, value: &str) -> Option<()> {
        let scalar = self.arena.scalar(value);
        self.arena.put_in_map(self.get_last()?, key, scalar)
    }

    fn on_key_and_flow_sequence(&mut self, key: &str, values: &[String]) -> Option<()> {
        let list = self.arena.list(values);
        self.arena.put_in_map(self.get_last()?, key, list)
    }

    fn on_only_key(&mut self, key: &str) -> Option<()> {
        let new_element = self.arena.create(Monomorph::Undeterminate);
        self.arena.put_in_map(self.get_last()?, key, new_element)?;
        self.nodes.push(new_element);
        Some(())
    }

    fn on_list_item_only_key(&mut self, key: &str) -> Option<()> {
        self.close_list_item();
        let new_element = self.arena.create(Monomorph::Undeterminate);
        self.arena.add_in_list(self.get_last()?, new_element)?;
        self.nodes.push(new_element);
        self.on_only_key(key)
    }

    fn on_list_item_only_value(&mut self, value: &str) -> Option<()> {
        self.close_list_item();
        let scalar = self.arena.scalar(value);
        self.arena.add_in_list(self.get_last()?, scalar)
    }

    fn on_list_item_key_and_value(&mut self, key: &str, value: &str) -> Option<()> {
        self.close_list_item();
        let new_element = self.arena.create(Monomorph::Undeterminate);
        self.arena.add_in_list(self.get_last()?, new_element)?;
        self.nodes.push(new_element);
        self.on_key_and_value(key, value)
    }

    fn on_list_item_key_and_flow_sequence(&mut self, key: &str, values: &[String]) -> Option<()> {
        self.close_list_item();
        let new_element = self.arena.create(Monomorph::Undeterminate);
        self.arena.add_in_list(self.get_last()?, new_element)?;
        self.nodes.push(new_element);
        self.on_key_and_flow_sequence(key, values)
    }
}

/// The document's nodes, its root first, or `None` where PlantUML throws (`YamlParser.parse`).
fn parse_monomorphs(lines: &[&str]) -> Option<Monomorphs> {
    let mut builder = YamlBuilder::new();
    let mut position = 0;
    while let Some(&line) = lines.get(position) {
        let yaml_line = YamlLine::build(line);
        match yaml_line.kind {
            YamlLineType::EmptyLine => {}
            YamlLineType::NoKeyOnlyText => return None,
            YamlLineType::PlainDash => builder.on_list_item_plain_dash()?,
            _ => {
                builder.adjust_indentation(yaml_line.indent)?;
                on_line(&mut builder, &yaml_line, lines, &mut position)?;
            }
        }
        position += 1;
    }
    Some(builder.arena)
}

fn on_line(
    builder: &mut YamlBuilder,
    line: &YamlLine,
    lines: &[&str],
    position: &mut usize,
) -> Option<()> {
    if line.list_item {
        return match line.kind {
            YamlLineType::KeyOnly => builder.on_list_item_only_key(&line.key),
            YamlLineType::PlainElementList => builder.on_list_item_only_value(&line.value),
            YamlLineType::KeyAndValue => builder.on_list_item_key_and_value(&line.key, &line.value),
            YamlLineType::KeyAndFlowSequence => {
                builder.on_list_item_key_and_flow_sequence(&line.key, &line.values)
            }
            _ => None,
        };
    }
    match line.kind {
        YamlLineType::KeyOnly => match peek_next(lines, *position) {
            Some(next) if next.indent > line.indent => {
                if next.kind == YamlLineType::NoKeyOnlyText {
                    let text = peek_next_only_text(lines, position);
                    builder.on_key_and_value(&line.key, &text)
                } else {
                    builder.on_only_key(&line.key)
                }
            }
            _ => builder.on_key_and_value(&line.key, ""),
        },
        YamlLineType::KeyAndValue => builder.on_key_and_value(&line.key, &line.value),
        YamlLineType::KeyAndBlockStyle => {
            let text = get_block_style_string(line.indent, lines, position);
            builder.on_key_and_value(&line.key, &text)
        }
        YamlLineType::KeyAndFlowSequence => {
            builder.on_key_and_flow_sequence(&line.key, &line.values)
        }
        _ => None,
    }
}

/// The next line that is not empty.
fn peek_next(lines: &[&str], position: usize) -> Option<YamlLine> {
    lines[position + 1..]
        .iter()
        .map(|line| YamlLine::build(line))
        .find(|line| line.kind != YamlLineType::EmptyLine)
}

/// The text lines that follow, joined with spaces; empty lines between them are skipped.
fn peek_next_only_text(lines: &[&str], position: &mut usize) -> String {
    let mut result = String::new();
    while let Some(peek) = lines.get(*position + 1) {
        let next = YamlLine::build(peek);
        match next.kind {
            YamlLineType::EmptyLine => {}
            YamlLineType::NoKeyOnlyText => {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&next.value);
            }
            _ => return result,
        }
        *position += 1;
    }
    result
}

/// The lines of a `|` block, each trimmed and ended with a newline.
fn get_block_style_string(indent: usize, lines: &[&str], position: &mut usize) -> String {
    let mut result = String::new();
    while let Some(line) = lines.get(*position + 1) {
        let yaml_line = YamlLine::build(line);
        if yaml_line.kind != YamlLineType::NoKeyOnlyText
            && yaml_line.kind != YamlLineType::EmptyLine
            && yaml_line.indent <= indent
        {
            return result;
        }
        result.push_str(crate::java::trim(line));
        result.push('\n');
        *position += 1;
    }
    result
}

/// `None` for a node PlantUML cannot convert (`MonomorphToJson`).
fn monomorph_to_json(arena: &Monomorphs, id: MonomorphId) -> Option<JsonValue> {
    match &arena.nodes[id] {
        Monomorph::Scalar(value) => Some(JsonValue::String(value.clone())),
        Monomorph::List(_) => convert_to_array(arena, id),
        Monomorph::Map(_) => convert_to_object(arena, id),
        Monomorph::Undeterminate => None,
    }
}

fn convert_to_array(arena: &Monomorphs, id: MonomorphId) -> Option<JsonValue> {
    let Monomorph::List(items) = &arena.nodes[id] else {
        unreachable!("a list");
    };
    items
        .iter()
        .map(|&item| match &arena.nodes[item] {
            Monomorph::Scalar(value) => Some(JsonValue::String(value.clone())),
            Monomorph::Map(_) => convert_to_object(arena, item),
            Monomorph::List(_) | Monomorph::Undeterminate => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(JsonValue::Array)
}

fn convert_to_object(arena: &Monomorphs, id: MonomorphId) -> Option<JsonValue> {
    let Monomorph::Map(entries) = &arena.nodes[id] else {
        unreachable!("a map");
    };
    let mut result = JsonObject::new();
    for (key, element) in entries {
        let value = match &arena.nodes[*element] {
            Monomorph::Scalar(value) => JsonValue::String(value.clone()),
            Monomorph::Map(_) => convert_to_object(arena, *element)?,
            Monomorph::List(_) => convert_to_array(arena, *element)?,
            Monomorph::Undeterminate => return None,
        };
        result.add(key.clone(), value);
    }
    Some(JsonValue::Object(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(lines: &[&str]) -> Option<String> {
        parse(lines).map(|value| value.to_string())
    }

    #[test]
    fn maps_lists_and_flow_lists_become_json_text() {
        assert_eq!(
            json(&["a: 1", "b:", "  - x", "  - y", "c: [p, 'q r']"]).as_deref(),
            Some(r#"{"a":"1","b":["x","y"],"c":["p","q r"]}"#)
        );
    }

    #[test]
    fn what_plantuml_cannot_read_gives_nothing() {
        assert_eq!(json(&["just text"]), None);
        assert_eq!(json(&["folded: >", "  text"]), None);
        assert_eq!(json(&["list:", "  -", "    - nested"]), None);
        assert_eq!(json(&[]), None);
    }

    #[test]
    fn block_scalars_keep_their_lines() {
        assert_eq!(
            json(&["text: |", "  one", "  two", "next: x"]).as_deref(),
            Some(r#"{"text":"one\ntwo\n","next":"x"}"#)
        );
    }
}

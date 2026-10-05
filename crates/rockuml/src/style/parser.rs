//! Reads style sheets: `selector { Property value }` blocks that nest, `--variables`, `@media` for dark mode.

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use regex::Regex;

use super::names::{PName, SName};
use super::signature::StyleSignature;
use super::value::Value;
use super::{STEREOTYPE_PRIORITY, Style};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleParsingError {
    /// PlantUML reports "Error in style definition: " followed by this message.
    Invalid(&'static str),
    /// Input on which PlantUML fails with an unchecked exception.
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    OpenBracket,
    CloseBracket,
    Text(String),
    Comma,
    Star,
    Newline,
    Semicolon,
    Colon,
    /// `@media ...`: what follows is for dark mode.
    Media(String),
}

impl Token {
    /// The text PlantUML keeps for the token, from which selector lists are rebuilt.
    fn data(&self) -> &str {
        match self {
            Self::OpenBracket => "{",
            Self::CloseBracket => "}",
            Self::Text(text) | Self::Media(text) => text,
            Self::Comma => ",",
            Self::Star => "*",
            Self::Newline => "NEWLINE",
            Self::Semicolon => ";",
            Self::Colon => ":",
        }
    }
}

/// Reads characters line by line, each line trimmed, never looking past the end of the current line.
struct CharInspector {
    lines: Vec<Vec<char>>,
    line: Option<usize>,
    position: usize,
}

impl CharInspector {
    /// Every line ends with `\n`.
    fn new(lines: &[&str]) -> Self {
        let lines = lines
            .iter()
            .map(|line| {
                let mut chars: Vec<char> = crate::java::trim(line).chars().collect();
                chars.push('\n');
                chars
            })
            .collect();
        Self {
            lines,
            line: Some(0),
            position: 0,
        }
    }

    /// `'\0'` past the end of the line or of the text.
    fn peek(&self, ahead: usize) -> char {
        self.line
            .and_then(|line| self.lines.get(line))
            .and_then(|line| line.get(self.position + ahead))
            .copied()
            .unwrap_or('\0')
    }

    fn jump(&mut self) {
        let Some(mut line) = self.line else {
            return;
        };
        self.position += 1;
        if self.position >= self.lines[line].len() {
            line += 1;
            self.position = 0;
        }
        while line < self.lines.len() && self.lines[line].is_empty() {
            line += 1;
        }
        self.line = (line < self.lines.len()).then_some(line);
    }
}

fn tokenize(mut inspector: CharInspector) -> Vec<Token> {
    let mut tokens = Vec::new();
    loop {
        let single = match inspector.peek(0) {
            '\0' => break,
            ' ' | '\t' => None,
            '/' if matches!(inspector.peek(1), '/' | '\'' | '*') => {
                match inspector.peek(1) {
                    '/' => skip_past(&mut inspector, '\n', None),
                    '\'' => skip_past(&mut inspector, '\'', Some('/')),
                    _ => skip_past(&mut inspector, '*', Some('/')),
                }
                continue;
            }
            ',' => Some(Token::Comma),
            ';' => Some(Token::Semicolon),
            '\n' | '\r' => Some(Token::Newline),
            '*' => Some(Token::Star),
            ':' => Some(Token::Colon),
            '{' => Some(Token::OpenBracket),
            '}' => Some(Token::CloseBracket),
            '@' => {
                tokens.push(Token::Media(read_media(&mut inspector)));
                continue;
            }
            '"' => {
                tokens.push(Token::Text(read_quoted(&mut inspector)));
                continue;
            }
            _ => {
                tokens.push(Token::Text(read_word(&mut inspector)));
                continue;
            }
        };
        tokens.extend(single);
        inspector.jump();
    }
    tokens
}

fn skip_past(inspector: &mut CharInspector, first: char, second: Option<char>) {
    while inspector.peek(0) != '\0' {
        if inspector.peek(0) == first && second.is_none_or(|second| inspector.peek(1) == second) {
            inspector.jump();
            if second.is_some() {
                inspector.jump();
            }
            return;
        }
        inspector.jump();
    }
}

fn read_media(inspector: &mut CharInspector) -> String {
    inspector.jump();
    let mut text = String::new();
    while inspector.peek(0) != '\0' {
        let c = inspector.peek(0);
        inspector.jump();
        if matches!(c, '{' | '}' | ';') {
            break;
        }
        text.push(c);
    }
    text
}

fn read_quoted(inspector: &mut CharInspector) -> String {
    inspector.jump();
    let mut text = String::new();
    while !matches!(inspector.peek(0), '\0' | '"') {
        text.push(inspector.peek(0));
        inspector.jump();
    }
    if inspector.peek(0) == '"' {
        inspector.jump();
    }
    text
}

/// A word, or for stereotype selectors (starting with `.`) everything up to a delimiter, spaces included.
fn read_word(inspector: &mut CharInspector) -> String {
    let mut text = String::new();
    loop {
        let c = inspector.peek(0);
        let ends_word = matches!(c, '\0' | '\n' | '\r' | '{' | '}' | ';' | ',' | ':' | '\t')
            || (c == ' ' && !text.starts_with('.'));
        if ends_word {
            break;
        }
        inspector.jump();
        text.push(c);
    }
    if text.starts_with('.') {
        crate::java::trim(&text).to_owned()
    } else {
        text
    }
}

/// The selectors open around the current position, with the properties declared directly inside.
struct Context {
    signatures: Vec<StyleSignature>,
    properties: BTreeMap<PName, Value>,
    parent: Option<Box<Context>>,
}

impl Context {
    fn root() -> Self {
        Self {
            signatures: vec![StyleSignature::empty()],
            properties: BTreeMap::new(),
            parent: None,
        }
    }

    fn is_root(&self) -> bool {
        self.signatures[0].is_empty()
    }

    /// Opens `selectors` (comma-separated, maybe starred) inside every currently open selector.
    fn push(self, selectors: &str) -> Self {
        let selectors = selectors.strip_prefix(':').unwrap_or(selectors);
        let (selectors, starred) = match selectors.strip_suffix('*') {
            Some(unstarred) => (crate::java::trim(unstarred), true),
            None => (selectors, false),
        };
        let mut signatures = Vec::new();
        for selector in crate::java::split(selectors, ",") {
            for outer in &self.signatures {
                let signature = selector_within(outer, &selector);
                signatures.push(if starred {
                    signature.with_star()
                } else {
                    signature
                });
            }
        }
        Self {
            signatures,
            properties: BTreeMap::new(),
            parent: Some(Box::new(self)),
        }
    }

    fn pop(self) -> Self {
        *self.parent.expect("only nested contexts are closed")
    }

    fn styles(&self) -> impl Iterator<Item = Style> {
        let properties = (!self.properties.is_empty()).then_some(&self.properties);
        self.signatures.iter().filter_map(move |signature| {
            let properties = properties?;
            let properties = if signature.has_stereotypes() {
                properties
                    .iter()
                    .map(|(&name, value)| (name, value.with_added_priority(STEREOTYPE_PRIORITY)))
                    .collect()
            } else {
                properties.clone()
            };
            Some(Style::new(signature.clone(), properties))
        })
    }
}

fn selector_within(outer: &StyleSignature, selector: &str) -> StyleSignature {
    static NON_DIGITS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\D").unwrap());
    if selector.starts_with('.') {
        outer.with_stereotype(selector)
    } else if selector.starts_with("depth(") {
        outer.with_level(
            NON_DIGITS
                .replace_all(selector, "")
                .parse()
                .unwrap_or_default(),
        )
    } else {
        match SName::retrieve(selector) {
            Some(name) => outer.with_name(name),
            None => outer.with_stereotype(selector),
        }
    }
}

/// Parses style sheets, numbering declarations with a counter shared by everything parsed for one diagram.
pub struct StyleParser<'a> {
    counter: &'a mut i32,
    variables: HashMap<String, String>,
    dark: bool,
}

impl<'a> StyleParser<'a> {
    pub fn new(counter: &'a mut i32) -> Self {
        Self {
            counter,
            variables: HashMap::new(),
            dark: false,
        }
    }

    pub fn parse(&mut self, lines: &[&str]) -> Result<Vec<Style>, StyleParsingError> {
        if lines.is_empty() {
            return Ok(Vec::new());
        }
        self.parse_tokens(&tokenize(CharInspector::new(lines)))
    }

    /// The rules in the order their blocks close.
    fn parse_tokens(&mut self, tokens: &[Token]) -> Result<Vec<Style>, StyleParsingError> {
        let mut styles = Vec::new();
        let mut context = Context::root();
        let mut index = 0;
        let peek = |index: usize| tokens.get(index);
        while let Some(token) = peek(index) {
            index += 1;
            match token {
                Token::Newline | Token::Semicolon => continue,
                Token::Text(text)
                    if text.eq_ignore_ascii_case("<style>")
                        || text.eq_ignore_ascii_case("</style>") =>
                {
                    continue;
                }
                _ => {}
            }
            match (token, peek(index).ok_or(StyleParsingError::Unexpected)?) {
                (_, Token::Comma) => {
                    let mut selectors = token.data().to_owned();
                    while let Some(next @ (Token::Text(_) | Token::Comma)) = peek(index) {
                        selectors.push_str(next.data());
                        index += 1;
                    }
                    index = skip_newlines(tokens, index);
                    if peek(index) != Some(&Token::OpenBracket) {
                        return Err(StyleParsingError::Unexpected);
                    }
                    context = context.push(&selectors);
                    index += 1;
                }
                (Token::Text(text), next) => {
                    let mut selector = text.clone();
                    if next == &Token::Star {
                        index += 1;
                        selector.push('*');
                    }
                    index = skip_newlines(tokens, index);
                    if peek(index) == Some(&Token::OpenBracket) {
                        context = context.push(&selector);
                        index += 1;
                        continue;
                    }
                    while peek(index) == Some(&Token::Colon) {
                        index += 1;
                    }
                    if let Some(variable) = text.strip_prefix("--") {
                        let value = read_value(tokens, &mut index)?;
                        self.variables.insert(variable.to_owned(), value);
                    } else if matches!(peek(index), Some(Token::Text(_))) {
                        let value = read_value(tokens, &mut index)?;
                        if let Some(name) = PName::retrieve(text) {
                            let value = self.declare(&self.resolve_variable(value));
                            context.properties.insert(name, value);
                        }
                    } else {
                        return Err(StyleParsingError::Invalid("parsing"));
                    }
                }
                (Token::CloseBracket, _) => {
                    styles.extend(context.styles());
                    if !context.is_root() {
                        context = context.pop();
                    }
                }
                (Token::Media(_), _) => self.dark = true,
                (Token::Colon, Token::Text(selector)) => {
                    let starred = peek(index + 1) == Some(&Token::Star);
                    let bracket = index + 1 + usize::from(starred);
                    if peek(bracket) != Some(&Token::OpenBracket) {
                        return Err(StyleParsingError::Unexpected);
                    }
                    let star = if starred { "*" } else { "" };
                    context = context.push(&format!(":{selector}{star}"));
                    index = bracket + 1;
                }
                (Token::OpenBracket, _) => {
                    return Err(StyleParsingError::Invalid("Invalid open bracket"));
                }
                _ => return Err(StyleParsingError::Unexpected),
            }
        }
        Ok(styles)
    }

    fn declare(&mut self, text: &str) -> Value {
        *self.counter += 1;
        if self.dark {
            Value::dark(text, *self.counter)
        } else {
            Value::regular(text, *self.counter)
        }
    }

    fn resolve_variable(&self, value: String) -> String {
        static VARIABLE: LazyLock<Regex> =
            LazyLock::new(|| crate::pattern::java_regex(r"^var\(-*([_\w][-_\w]+)\)$", false));
        VARIABLE
            .captures(&value)
            .and_then(|captures| self.variables.get(&captures[1]))
            .cloned()
            .unwrap_or(value)
    }
}

fn skip_newlines(tokens: &[Token], mut index: usize) -> usize {
    while tokens.get(index) == Some(&Token::Newline) {
        index += 1;
    }
    index
}

/// A property value: words joined by single spaces up to the end of the declaration.
fn read_value(tokens: &[Token], index: &mut usize) -> Result<String, StyleParsingError> {
    let mut value = String::new();
    while let Some(token) = tokens.get(*index) {
        match token {
            Token::Newline | Token::Semicolon | Token::CloseBracket => break,
            Token::Text(text) => {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(text);
            }
            Token::Comma => value.push(','),
            Token::Colon => match tokens.get(*index + 1) {
                Some(Token::Text(text)) => {
                    value.push(':');
                    value.push_str(text);
                    *index += 1;
                }
                Some(_) => return Err(StyleParsingError::Invalid("bad definition")),
                None => return Err(StyleParsingError::Unexpected),
            },
            _ => return Err(StyleParsingError::Invalid("bad definition")),
        }
        *index += 1;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::super::ValueReading;
    use super::*;

    fn parse(text: &str) -> Result<Vec<Style>, StyleParsingError> {
        let lines: Vec<&str> = text.lines().collect();
        let mut counter = 0;
        StyleParser::new(&mut counter).parse(&lines)
    }

    #[test]
    fn nested_blocks_extend_the_outer_selector() {
        let styles =
            parse("document {\n  BackGroundColor white\n  title {\n    FontSize 14\n  }\n}")
                .unwrap();
        assert_eq!(styles.len(), 2);
        assert_eq!(
            styles[0].signature(),
            &StyleSignature::of(&[SName::Document, SName::Title])
        );
        assert_eq!(
            styles[1].signature(),
            &StyleSignature::of(&[SName::Document])
        );
        assert!(styles[1].has_value(PName::BackGroundColor));
    }

    #[test]
    fn variables_and_quoted_words_are_resolved() {
        let styles = parse(
            "root {\n --bg: #f1f1f1;\n BackGroundColor: var(--bg);\n FontName \"Courier New\"\n}",
        )
        .unwrap();
        assert_eq!(
            styles[0].value(PName::BackGroundColor).as_string(),
            "#f1f1f1"
        );
        assert_eq!(styles[0].value(PName::FontName).as_string(), "Courier New");
    }

    #[test]
    fn selector_lists_and_stereotypes_make_one_rule_each() {
        let styles = parse("node, .Big_One {\n LineColor red\n}").unwrap();
        assert_eq!(styles.len(), 2);
        assert_eq!(
            styles[1].signature(),
            &StyleSignature::empty().with_stereotype("bigone")
        );
        assert_eq!(
            styles[1].value(PName::LineColor).unwrap().priority(),
            1 + STEREOTYPE_PRIORITY
        );
    }

    #[test]
    fn comments_are_skipped() {
        let styles = parse("// line\nroot { /* block */ FontSize 9 /' quote '/ }").unwrap();
        assert_eq!(styles.len(), 1);
        assert!(styles[0].has_value(PName::FontSize));
    }

    #[test]
    fn malformed_sheets_are_reported_like_plantuml() {
        assert_eq!(
            parse("root {\n FontSize\n}"),
            Err(StyleParsingError::Invalid("parsing"))
        );
        assert_eq!(
            parse("{\n}"),
            Err(StyleParsingError::Invalid("Invalid open bracket"))
        );
        assert_eq!(
            parse("root {\n FontSize 1 ,{\n}"),
            Err(StyleParsingError::Invalid("bad definition"))
        );
    }
}

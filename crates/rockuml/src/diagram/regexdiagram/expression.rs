//! A regular expression read into tokens and put in postfix order (PlantUML's `RegexExpression`, `ReToken`,
//! `ReTokenType`, `ShuntingYard` and `GroupSplitter`).

use crate::command::{CharInspector, PastTheEnd};
use crate::java;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReTokenType {
    SimpleChar,
    EscapedChar,
    Class,
    Quantifier,
    Anchor,
    Group,
    NamedGroup,
    LookAhead,
    LookBehind,
    Alternative,
    ParenthesisOpen,
    ParenthesisClose,
    ConcatenationImplicit,
}

impl ReTokenType {
    fn is_named_group_or_look_ahead_or_look_behind(self) -> bool {
        matches!(self, Self::NamedGroup | Self::LookAhead | Self::LookBehind)
    }

    fn need_implicit_concatenation(token1: Self, token2: Self) -> bool {
        !(token1.is_named_group_or_look_ahead_or_look_behind()
            || token1 == Self::Alternative
            || token2 == Self::Alternative
            || token2 == Self::Quantifier
            || token1 == Self::ParenthesisOpen
            || token2 == Self::ParenthesisClose)
    }

    /// The name PlantUML's messages give the type.
    fn java_name(self) -> &'static str {
        match self {
            Self::SimpleChar => "SIMPLE_CHAR",
            Self::EscapedChar => "ESCAPED_CHAR",
            Self::Class => "CLASS",
            Self::Quantifier => "QUANTIFIER",
            Self::Anchor => "ANCHOR",
            Self::Group => "GROUP",
            Self::NamedGroup => "NAMED_GROUP",
            Self::LookAhead => "LOOK_AHEAD",
            Self::LookBehind => "LOOK_BEHIND",
            Self::Alternative => "ALTERNATIVE",
            Self::ParenthesisOpen => "PARENTHESIS_OPEN",
            Self::ParenthesisClose => "PARENTHESIS_CLOSE",
            Self::ConcatenationImplicit => "CONCATENATION_IMPLICIT",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReToken {
    pub token_type: ReTokenType,
    pub data: String,
}

impl ReToken {
    fn new(token_type: ReTokenType, data: impl Into<String>) -> Self {
        Self {
            token_type,
            data: data.into(),
        }
    }
}

impl std::fmt::Display for ReToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.token_type.java_name(), self.data)
    }
}

/// Why a regular expression is not drawn.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Failure {
    /// PlantUML's `RegexParsingException`, reported as an error.
    Parsing(String),
    /// Where PlantUML throws something else, this Java exception.
    Thrown(&'static str),
}

impl Failure {
    /// Taking from an empty stack.
    pub(super) const NO_SUCH_ELEMENT: Self = Self::Thrown("java.util.NoSuchElementException");
    /// Reading the first element of an empty list.
    pub(super) const INDEX_OUT_OF_BOUNDS: Self =
        Self::Thrown("java.lang.IndexOutOfBoundsException: Index 0 out of bounds for length 0");
}

impl From<PastTheEnd> for Failure {
    fn from(_: PastTheEnd) -> Self {
        Self::Thrown(PastTheEnd::EXCEPTION)
    }
}

/// The tokens of a regular expression (`RegexExpression.parse`).
pub(super) fn parse(it: &mut CharInspector) -> Result<Vec<ReToken>, Failure> {
    use ReTokenType as T;
    let mut result = Vec::new();
    loop {
        let current = it.peek(0);
        if current == '\0' {
            break;
        }
        if is_start_anchor(it) {
            result.push(ReToken::new(T::Anchor, read_anchor(it)?));
        } else if is_escaped_char(it) {
            result.push(ReToken::new(T::EscapedChar, it.peek(1)));
            it.jump()?;
            it.jump()?;
        } else if current == '|' {
            result.push(ReToken::new(T::Alternative, "|"));
            it.jump()?;
        } else if is_start_posix_group(it) {
            result.push(ReToken::new(T::Class, read_group_posix(it)?));
        } else if current == '[' {
            result.push(ReToken::new(T::Group, read_group(it)?));
        } else if is_start_comment(it) {
            skip_comment(it)?;
        } else if is_start_look_ahead(it) {
            result.push(read_look_ahead(it)?);
            result.push(ReToken::new(T::ParenthesisOpen, "("));
        } else if is_start_look_behind(it) {
            result.push(read_look_behind(it)?);
            result.push(ReToken::new(T::ParenthesisOpen, "("));
        } else if is_start_named_capturing_group(it) {
            result.push(read_named_group(it)?);
            result.push(ReToken::new(T::ParenthesisOpen, "("));
        } else if current == '(' {
            result.push(read_open_parenthesis(it)?);
        } else if current == ')' {
            result.push(ReToken::new(T::ParenthesisClose, ")"));
            it.jump()?;
        } else if is_start_quantifier(current) {
            result.push(ReToken::new(T::Quantifier, read_quantifier(it)?));
        } else if it.peek(0) == '\\' && it.peek(1) == '0' {
            result.push(ReToken::new(T::Class, read_unicode_or_octal_escape(it, 4)?));
        } else if it.peek(0) == '\\' && it.peek(1) == 'u' {
            result.push(ReToken::new(T::Class, read_unicode_or_octal_escape(it, 5)?));
        } else if is_start_unicode_class(it) {
            result.push(ReToken::new(T::Class, read_unicode_class(it)?));
        } else if current == '.' || current == '\\' {
            result.push(ReToken::new(T::Class, read_class(it)?));
        } else {
            result.push(ReToken::new(T::SimpleChar, current));
            it.jump()?;
        }
    }
    Ok(result)
}

fn is_start_look_ahead(it: &CharInspector) -> bool {
    it.peek(0) == '(' && it.peek(1) == '?' && matches!(it.peek(2), '=' | '!')
}

fn is_start_look_behind(it: &CharInspector) -> bool {
    it.peek(0) == '(' && it.peek(1) == '?' && it.peek(2) == '<' && matches!(it.peek(3), '=' | '!')
}

fn is_start_posix_group(it: &CharInspector) -> bool {
    it.peek(0) == '[' && it.peek(1) == '[' && it.peek(2) == ':'
}

/// `(?<name>`, the name of letters only, which a supplementary character is not to Java.
fn is_start_named_capturing_group(it: &CharInspector) -> bool {
    if it.peek(0) != '(' || it.peek(1) != '?' || it.peek(2) != '<' {
        return false;
    }
    let mut i = 3;
    while it.peek(i) != '\0' {
        if it.peek(i) == '>' {
            return i > 3;
        }
        let ch = it.peek(i);
        if !(java::is_one_char(ch) && java::is_letter(ch)) {
            return false;
        }
        i += 1;
    }
    false
}

fn is_start_comment(it: &CharInspector) -> bool {
    it.peek(0) == '(' && it.peek(1) == '?' && it.peek(2) == '#'
}

fn skip_comment(it: &mut CharInspector) -> Result<(), Failure> {
    jump(it, 3)?;
    loop {
        if it.peek(0) == '\0' {
            return Err(Failure::Parsing("Unclosed comment".to_owned()));
        }
        let closing = it.peek(0) == ')';
        it.jump()?;
        if closing {
            return Ok(());
        }
    }
}

fn jump(it: &mut CharInspector, count: usize) -> Result<(), PastTheEnd> {
    (0..count).try_for_each(|_| it.jump())
}

fn read_look_ahead(it: &mut CharInspector) -> Result<ReToken, Failure> {
    jump(it, 2)?;
    let ch = it.peek(0);
    it.jump()?;
    Ok(ReToken::new(ReTokenType::LookAhead, format!("?{ch}")))
}

fn read_look_behind(it: &mut CharInspector) -> Result<ReToken, Failure> {
    jump(it, 3)?;
    let ch = it.peek(0);
    it.jump()?;
    Ok(ReToken::new(ReTokenType::LookBehind, format!("?<{ch}")))
}

fn read_named_group(it: &mut CharInspector) -> Result<ReToken, Failure> {
    jump(it, 3)?;
    let mut named_group = String::new();
    loop {
        match it.peek(0) {
            '\0' => {
                return Err(Failure::Parsing(
                    "Unclosed named capturing group".to_owned(),
                ));
            }
            '>' => {
                it.jump()?;
                return Ok(ReToken::new(ReTokenType::NamedGroup, named_group));
            }
            ch => {
                named_group.push(ch);
                it.jump()?;
            }
        }
    }
}

/// `(`, `(?:` or `(?!`.
fn read_open_parenthesis(it: &mut CharInspector) -> Result<ReToken, Failure> {
    let mut result = String::from(it.peek(0));
    it.jump()?;
    for kind in [':', '!'] {
        if it.peek(0) == '?' && it.peek(1) == kind {
            jump(it, 2)?;
            result.push('?');
            result.push(kind);
        }
    }
    Ok(ReToken::new(ReTokenType::ParenthesisOpen, result))
}

fn is_start_quantifier(current: char) -> bool {
    matches!(current, '*' | '+' | '?' | '{')
}

/// `*`, `+`, `?` or `{…}`, then `?` if lazy.
fn read_quantifier(it: &mut CharInspector) -> Result<String, Failure> {
    let current0 = it.peek(0);
    it.jump()?;
    let mut tmp = String::from(current0);
    if current0 == '{' {
        while it.peek(0) != '\0' {
            let ch = it.peek(0);
            tmp.push(ch);
            it.jump()?;
            if ch == '}' {
                break;
            }
        }
    }
    if it.peek(0) == '?' {
        tmp.push('?');
        it.jump()?;
    }
    if tmp.starts_with('{') && !is_braced_count(&tmp) {
        return Err(Failure::Parsing(format!("Bad quantifier {tmp}")));
    }
    Ok(tmp)
}

/// `^\{[0-9,]+\}$`.
fn is_braced_count(quantifier: &str) -> bool {
    quantifier
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .is_some_and(|inside| {
            !inside.is_empty() && inside.chars().all(|c| c.is_ascii_digit() || c == ',')
        })
}

fn is_escaped_char(it: &CharInspector) -> bool {
    it.peek(0) == '\\' && ".*\\?^$|()[]{}<>".contains(it.peek(1))
}

/// `[[:name:]]`, read as `:name:`.
fn read_group_posix(it: &mut CharInspector) -> Result<String, Failure> {
    jump(it, 3)?;
    let mut result = String::from(":");
    while it.peek(0) != '\0' {
        let ch = it.peek(0);
        it.jump()?;
        result.push(ch);
        if ch == ':' {
            break;
        }
    }
    jump(it, 2)?;
    Ok(result)
}

/// What is between `[` and `]`, escapes kept.
fn read_group(it: &mut CharInspector) -> Result<String, Failure> {
    it.jump()?;
    let mut result = String::new();
    while it.peek(0) != '\0' {
        let ch = it.peek(0);
        it.jump()?;
        if ch == ']' {
            break;
        }
        result.push(ch);
        if ch == '\\' {
            result.push(it.peek(0));
            it.jump()?;
        }
    }
    Ok(result)
}

/// `\p{…}` or `\x{…}`.
fn read_unicode_class(it: &mut CharInspector) -> Result<String, Failure> {
    let mut result = String::from(it.peek(0));
    it.jump()?;
    while it.peek(0) != '\0' {
        let ch = it.peek(0);
        it.jump()?;
        result.push(ch);
        if ch == '}' {
            return Ok(result);
        }
    }
    Err(Failure::Parsing("Unexpected end of data".to_owned()))
}

/// The backslash and the `nb` characters after it.
fn read_unicode_or_octal_escape(it: &mut CharInspector, nb: usize) -> Result<String, Failure> {
    let mut result = String::from(it.peek(0));
    it.jump()?;
    for _ in 0..nb {
        let ch = it.peek(0);
        if ch == '\0' {
            return Err(Failure::Parsing("Unexpected end of data".to_owned()));
        }
        result.push(ch);
        it.jump()?;
    }
    Ok(result)
}

/// `.` or a backslash and the letter after it.
fn read_class(it: &mut CharInspector) -> Result<String, Failure> {
    let current0 = it.peek(0);
    it.jump()?;
    if current0 == '.' {
        return Ok(".".to_owned());
    }
    let result = format!("{current0}{}", it.peek(0));
    it.jump()?;
    Ok(result)
}

fn is_start_unicode_class(it: &CharInspector) -> bool {
    it.peek(0) == '\\' && matches!(it.peek(1), 'p' | 'x') && it.peek(2) == '{'
}

fn is_start_anchor(it: &CharInspector) -> bool {
    matches!(it.peek(0), '^' | '$')
        || it.peek(0) == '\\' && matches!(it.peek(1), 'A' | 'Z' | 'z' | 'G' | 'b' | 'B')
}

fn read_anchor(it: &mut CharInspector) -> Result<String, Failure> {
    let current0 = it.peek(0);
    it.jump()?;
    if current0 == '\\' {
        let result = format!("{current0}{}", it.peek(0));
        it.jump()?;
        return Ok(result);
    }
    Ok(current0.to_string())
}

/// The tokens with the concatenations they leave implicit between them.
pub(super) fn add_implicit_concatenation(list: Vec<ReToken>) -> Vec<ReToken> {
    let mut result: Vec<ReToken> = Vec::new();
    for token in list {
        if result.last().is_some_and(|last| {
            ReTokenType::need_implicit_concatenation(last.token_type, token.token_type)
        }) {
            result.push(ReToken::new(ReTokenType::ConcatenationImplicit, ""));
        }
        result.push(token);
    }
    result
}

/// The tokens in postfix order (`ShuntingYard`). Unclosed parentheses are left in.
pub(super) fn shunting_yard(tokens: Vec<ReToken>) -> Result<Vec<ReToken>, Failure> {
    use ReTokenType as T;
    let mut ouput_queue = Vec::new();
    let mut operator_stack: Vec<ReToken> = Vec::new();
    for token in tokens {
        match token.token_type {
            T::SimpleChar | T::EscapedChar | T::Group | T::Class | T::Anchor | T::Quantifier => {
                ouput_queue.push(token);
            }
            T::NamedGroup | T::LookAhead | T::LookBehind | T::ConcatenationImplicit => {
                operator_stack.push(token);
            }
            T::Alternative => {
                while let Some(top) =
                    operator_stack.pop_if(|top| top.token_type == T::ConcatenationImplicit)
                {
                    ouput_queue.push(top);
                }
                operator_stack.push(token);
            }
            T::ParenthesisOpen => operator_stack.push(token),
            T::ParenthesisClose => {
                while let Some(top) =
                    operator_stack.pop_if(|top| top.token_type != T::ParenthesisOpen)
                {
                    ouput_queue.push(top);
                }
                operator_stack.pop().ok_or(Failure::NO_SUCH_ELEMENT)?;
                if let Some(top) = operator_stack
                    .pop_if(|top| top.token_type.is_named_group_or_look_ahead_or_look_behind())
                {
                    ouput_queue.push(top);
                }
            }
        }
    }
    ouput_queue.extend(operator_stack.into_iter().rev());
    Ok(ouput_queue)
}

/// The characters and ranges of a class `[…]`, an escaped punctuation without its backslash
/// (`GroupSplitter`). PlantUML splits UTF-16 units, which parts a surrogate pair.
pub(super) fn split_group(group: &str) -> Vec<String> {
    let units: Vec<u16> = group.encode_utf16().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < units.len() {
        if i + 2 < units.len() && units[i + 1] == u16::from(b'-') {
            result.push(java::string_from_utf16(&units[i..i + 3]));
            i += 2;
        } else if i + 1 < units.len() && units[i] == u16::from(b'\\') {
            let escaped = &units[i + 1..i + 2];
            let is_letter = char::from_u32(u32::from(escaped[0])).is_some_and(java::is_letter);
            result.push(java::string_from_utf16(if is_letter {
                &units[i..i + 2]
            } else {
                escaped
            }));
            i += 1;
        } else {
            result.push(java::string_from_utf16(&units[i..=i]));
        }
        i += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::BlocLines;

    fn postfix(regex: &str) -> Result<String, Failure> {
        let tokens = parse(&mut BlocLines::from_texts(&[regex]).inspector())?;
        let queue = shunting_yard(add_implicit_concatenation(tokens))?;
        Ok(queue
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" "))
    }

    #[test]
    fn concatenation_is_implicit_and_binds_tighter_than_alternation() {
        assert_eq!(
            postfix("ab|c").unwrap(),
            "SIMPLE_CHAR[a] SIMPLE_CHAR[b] CONCATENATION_IMPLICIT[] SIMPLE_CHAR[c] ALTERNATIVE[|]"
        );
    }

    #[test]
    fn quantifiers_follow_what_they_repeat() {
        assert_eq!(
            postfix(r"\d{2,3}x+?").unwrap(),
            "CLASS[\\d] QUANTIFIER[{2,3}] SIMPLE_CHAR[x] QUANTIFIER[+?] CONCATENATION_IMPLICIT[]"
        );
        assert_eq!(
            postfix("a{2}?"),
            Err(Failure::Parsing("Bad quantifier {2}?".to_owned()))
        );
    }

    #[test]
    fn named_groups_and_lookarounds_apply_after_their_parenthesis() {
        assert_eq!(
            postfix("(?<id>a)(?=b)").unwrap(),
            "SIMPLE_CHAR[a] NAMED_GROUP[id] SIMPLE_CHAR[b] LOOK_AHEAD[?=] CONCATENATION_IMPLICIT[]"
        );
    }

    #[test]
    fn errors_are_reported_or_thrown() {
        assert_eq!(
            postfix("(ab").unwrap(),
            "SIMPLE_CHAR[a] SIMPLE_CHAR[b] CONCATENATION_IMPLICIT[] PARENTHESIS_OPEN[(]"
        );
        assert_eq!(
            postfix("a{x}"),
            Err(Failure::Parsing("Bad quantifier {x}".to_owned()))
        );
        assert_eq!(
            postfix("a(?#b"),
            Err(Failure::Parsing("Unclosed comment".to_owned()))
        );
        assert_eq!(postfix("a)"), Err(Failure::NO_SUCH_ELEMENT));
        assert_eq!(
            postfix("a\\"),
            Err(Failure::Thrown("java.lang.IllegalStateException"))
        );
    }

    #[test]
    fn groups_split_into_characters_and_ranges() {
        assert_eq!(split_group(r"a-z_\]\w"), ["a-z", "_", "]", "\\w"]);
    }

    #[test]
    fn a_supplementary_character_is_two_units_and_no_letter() {
        assert_eq!(split_group("\u{1F600}-a"), ["?", "?-a"]);
        assert!(
            postfix("(?<\u{1D49C}b>x)")
                .unwrap()
                .starts_with("QUANTIFIER[?] SIMPLE_CHAR[<]")
        );
    }
}

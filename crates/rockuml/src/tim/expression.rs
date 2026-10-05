//! Preprocessor expressions: tokenizing, shunting-yard ordering and reverse-Polish evaluation.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use super::context::TContext;
use super::eater::Eater;
use super::error::{TimError, TimResult, fail};
use super::function::FunctionSignature;
use super::line_type::is_quote;
use super::memory::Memory;
use super::value::TValue;
use crate::java;
use crate::json::JsonValue;
use crate::text::StringLocated;

/// Stands for a binary minus, so that `-` can also start negative numbers.
const COMMERCIAL_MINUS_SIGN: char = '\u{2052}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TokenType {
    QuotedString,
    JsonData,
    Operator,
    OpenParenMath,
    Comma,
    CloseParenMath,
    Number,
    PlainText,
    Spaces,
    FunctionName,
    OpenParenFunc,
    CloseParenFunc,
    Affectation,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Token {
    surface: String,
    kind: TokenType,
    json: Option<JsonValue>,
}

impl Token {
    fn new(surface: impl Into<String>, kind: TokenType) -> Self {
        Self {
            surface: surface.into(),
            kind,
            json: None,
        }
    }

    pub(super) fn is_spaces(&self) -> bool {
        self.kind == TokenType::Spaces
    }

    fn operator(&self) -> Option<Operator> {
        let mut chars = self.surface.chars();
        let first = chars.next()?;
        Operator::from_chars(first, chars.next().unwrap_or('\0'))
    }

    fn precedence(&self) -> i32 {
        if self.kind == TokenType::Affectation {
            return Operator::Equals.precedence();
        }
        self.operator().map_or(0, Operator::precedence)
    }

    fn is_operator_or_affectation(&self) -> bool {
        matches!(self.kind, TokenType::Operator | TokenType::Affectation)
    }
}

impl From<&TValue> for Token {
    fn from(value: &TValue) -> Self {
        match value {
            TValue::Int(_) => Token::new(value.to_string(), TokenType::Number),
            TValue::Json(json) => Token {
                surface: value.to_string(),
                kind: TokenType::JsonData,
                json: Some(json.clone()),
            },
            TValue::String(_) => Token::new(value.to_string(), TokenType::QuotedString),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    Multiplication,
    Division,
    Addition,
    Subtraction,
    LessThan,
    GreaterThan,
    LessThanOrEquals,
    GreaterThanOrEquals,
    Equals,
    NotEquals,
    LogicalAnd,
    LogicalOr,
}

impl Operator {
    fn from_chars(c: char, next: char) -> Option<Self> {
        Some(match (c, next) {
            ('*', _) => Self::Multiplication,
            ('/', _) => Self::Division,
            ('+', _) => Self::Addition,
            (COMMERCIAL_MINUS_SIGN, _) => Self::Subtraction,
            ('<', '=') => Self::LessThanOrEquals,
            ('<', _) => Self::LessThan,
            ('>', '=') => Self::GreaterThanOrEquals,
            ('>', _) => Self::GreaterThan,
            ('=', '=') => Self::Equals,
            ('!', '=') => Self::NotEquals,
            ('&', '&') => Self::LogicalAnd,
            ('|', '|') => Self::LogicalOr,
            _ => return None,
        })
    }

    fn precedence(self) -> i32 {
        match self {
            Self::Multiplication | Self::Division => 97,
            Self::Addition | Self::Subtraction => 96,
            Self::LessThan
            | Self::GreaterThan
            | Self::LessThanOrEquals
            | Self::GreaterThanOrEquals => 94,
            Self::Equals | Self::NotEquals => 93,
            Self::LogicalAnd => 89,
            Self::LogicalOr => 88,
        }
    }

    fn is_single_char(self) -> bool {
        matches!(
            self,
            Self::Multiplication
                | Self::Division
                | Self::Addition
                | Self::Subtraction
                | Self::LessThan
                | Self::GreaterThan
        )
    }

    fn operate(self, left: &TValue, right: &TValue) -> TimResult<TValue> {
        use std::cmp::Ordering::{Equal, Greater, Less};
        Ok(match self {
            Self::Multiplication => left.multiply(right),
            Self::Division => left.divided_by(right).ok_or(TimError::Fatal)?,
            Self::Addition => left.add(right),
            Self::Subtraction => left.minus(right),
            Self::LessThan => TValue::from_bool(left.compare(right) == Less),
            Self::GreaterThan => TValue::from_bool(left.compare(right) == Greater),
            Self::LessThanOrEquals => TValue::from_bool(left.compare(right) != Greater),
            Self::GreaterThanOrEquals => TValue::from_bool(left.compare(right) != Less),
            Self::Equals => TValue::from_bool(left.compare(right) == Equal),
            Self::NotEquals => TValue::from_bool(left.compare(right) != Equal),
            Self::LogicalAnd => TValue::from_bool(left.to_bool() && right.to_bool()),
            Self::LogicalOr => TValue::from_bool(left.to_bool() || right.to_bool()),
        })
    }
}

fn token_type_of(c: char, next: char) -> TokenType {
    if is_quote(c) {
        TokenType::QuotedString
    } else if c == '=' {
        TokenType::Affectation
    } else if c == '(' {
        TokenType::OpenParenMath
    } else if c == ')' {
        TokenType::CloseParenMath
    } else if c == ',' {
        TokenType::Comma
    } else if c.is_ascii_digit() {
        TokenType::Number
    } else if java::is_space_char(c) {
        TokenType::Spaces
    } else if c == '-' || Operator::from_chars(c, next).is_some() {
        TokenType::Operator
    } else {
        TokenType::PlainText
    }
}

fn is_plain_text_break(c: char, next: char) -> bool {
    matches!(
        token_type_of(c, next),
        TokenType::OpenParenMath
            | TokenType::Comma
            | TokenType::CloseParenMath
            | TokenType::Operator
            | TokenType::Spaces
            | TokenType::Affectation
    )
}

/// After a value, `-` subtracts; after an operator, `(`, `,` or `=`, it starts a negative number.
fn is_subtraction_operator(last_token: Option<&Token>) -> bool {
    last_token.is_some_and(|token| {
        !matches!(
            token.kind,
            TokenType::Operator
                | TokenType::OpenParenMath
                | TokenType::Comma
                | TokenType::Affectation
        )
    })
}

pub(super) fn eat_one_token(
    last_token: Option<&Token>,
    eater: &mut Eater,
    stop_at_colon: bool,
) -> TimResult<Option<Token>> {
    let mut c = eater.peek_char();
    if c == '\0' || (stop_at_colon && c == ':') {
        return Ok(None);
    }
    if c == '-' && is_subtraction_operator(last_token) {
        c = COMMERCIAL_MINUS_SIGN;
    }
    let token = if is_quote(c) {
        Token::new(eater.eat_and_get_quoted_string()?, TokenType::QuotedString)
    } else if let Some(operator) = Operator::from_chars(c, eater.peek_char_n2()) {
        if operator.is_single_char() {
            eater.eat_one_char()?;
            Token::new(c, TokenType::Operator)
        } else {
            let surface: String = [eater.eat_one_char()?, eater.eat_one_char()?]
                .iter()
                .collect();
            Token::new(surface, TokenType::Operator)
        }
    } else if c == '=' {
        Token::new(eater.eat_one_char()?, TokenType::Affectation)
    } else if c == '(' {
        Token::new(eater.eat_one_char()?, TokenType::OpenParenMath)
    } else if c == ')' {
        Token::new(eater.eat_one_char()?, TokenType::CloseParenMath)
    } else if c == ',' {
        Token::new(eater.eat_one_char()?, TokenType::Comma)
    } else if c.is_ascii_digit() || c == '-' {
        Token::new(eater.eat_and_get_number()?, TokenType::Number)
    } else if java::is_space_char(c) {
        Token::new(eater.eat_and_get_spaces()?, TokenType::Spaces)
    } else {
        let mut text = String::new();
        loop {
            let c = eater.peek_char();
            if c == '\0' || is_plain_text_break(c, eater.peek_char_n2()) {
                break;
            }
            text.push(eater.eat_one_char()?);
        }
        Token::new(text, TokenType::PlainText)
    };
    Ok(Some(token))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct TokenStack {
    tokens: Vec<Token>,
}

impl TokenStack {
    pub(super) fn push(&mut self, token: Token) {
        self.tokens.push(token);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    fn without_spaces(&self) -> TokenStack {
        TokenStack {
            tokens: self
                .tokens
                .iter()
                .filter(|token| !token.is_spaces())
                .cloned()
                .collect(),
        }
    }

    /// Reads a function argument: tokens up to the next top-level `,` or `)`.
    pub(super) fn eat_until_close_parenthesis_or_comma(eater: &mut Eater) -> TimResult<TokenStack> {
        let mut result = TokenStack::default();
        let mut level = 0;
        let mut last_significant: Option<Token> = None;
        loop {
            eater.skip_spaces();
            let c = eater.peek_char();
            if c == '\0' {
                return fail("until001", eater.line());
            }
            if level == 0 && (c == ',' || c == ')') {
                return Ok(result);
            }
            let token =
                eat_one_token(last_significant.as_ref(), eater, false)?.ok_or(TimError::Fatal)?;
            match token.kind {
                TokenType::OpenParenMath => level += 1,
                TokenType::CloseParenMath => level -= 1,
                _ => {}
            }
            if !token.is_spaces() {
                last_significant = Some(token.clone());
            }
            result.push(token);
        }
    }

    /// A plain word right before `(` is a function call: retype it and record its argument count.
    pub(super) fn guess_functions(&mut self, location: &StringLocated) -> TimResult<()> {
        let mut open = Vec::new();
        let mut pairs = BTreeMap::new();
        for (index, token) in self.tokens.iter().enumerate() {
            match token.kind {
                TokenType::OpenParenMath => open.push(index),
                TokenType::CloseParenMath => {
                    pairs.insert(open.pop().ok_or(TimError::Fatal)?, index);
                }
                _ => {}
            }
        }
        for (open_index, close_index) in pairs {
            if open_index > 0 && self.tokens[open_index - 1].kind == TokenType::PlainText {
                self.tokens[open_index - 1].kind = TokenType::FunctionName;
                let argument_count =
                    count_function_arguments(&self.tokens[open_index + 1..], location)?;
                self.tokens[open_index] =
                    Token::new(argument_count.to_string(), TokenType::OpenParenFunc);
                self.tokens[close_index] = Token::new(")", TokenType::CloseParenFunc);
            }
        }
        Ok(())
    }

    pub(super) fn get_result(
        &self,
        location: &StringLocated,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<TValue> {
        let mut tokens = self.without_spaces();
        tokens.guess_functions(location)?;
        let queue = shunting_yard(&tokens.tokens, context, memory, location)?;
        evaluate_reverse_polish(&queue, context, memory, location)
    }
}

fn count_function_arguments(tokens: &[Token], location: &StringLocated) -> TimResult<usize> {
    let first = tokens.first().ok_or(TimError::Fatal)?;
    if matches!(
        first.kind,
        TokenType::CloseParenMath | TokenType::CloseParenFunc
    ) {
        return Ok(0);
    }
    let mut count = 1;
    let mut index = 0;
    while index < tokens.len() {
        index = skip_argument(tokens, index, location)?;
        let token = &tokens[index];
        index += 1;
        match token.kind {
            TokenType::CloseParenMath | TokenType::CloseParenFunc => return Ok(count),
            TokenType::Comma => count += 1,
            _ => return fail("count13", location),
        }
    }
    fail("count12", location)
}

fn skip_argument(tokens: &[Token], mut index: usize, location: &StringLocated) -> TimResult<usize> {
    let mut level = 0;
    loop {
        let Some(token) = tokens.get(index) else {
            return fail("until002", location);
        };
        if (level == 0 && matches!(token.kind, TokenType::Comma | TokenType::CloseParenMath))
            || token.kind == TokenType::CloseParenFunc
        {
            return Ok(index);
        }
        match token.kind {
            TokenType::OpenParenMath | TokenType::OpenParenFunc => level += 1,
            TokenType::CloseParenMath | TokenType::CloseParenFunc => level -= 1,
            _ => {}
        }
        index += 1;
    }
}

fn shunting_yard(
    tokens: &[Token],
    context: &mut TContext,
    memory: &mut Memory,
    location: &StringLocated,
) -> TimResult<Vec<Token>> {
    static VARIABLE_NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9.$_]+$").unwrap());

    let mut output = Vec::new();
    let mut operators: Vec<Token> = Vec::new();
    for token in tokens {
        match token.kind {
            TokenType::Number | TokenType::QuotedString => output.push(token.clone()),
            TokenType::FunctionName | TokenType::OpenParenFunc | TokenType::OpenParenMath => {
                operators.push(token.clone());
            }
            TokenType::PlainText => {
                match context.variable_for_expression(memory, &token.surface, location)? {
                    Some(value) => output.push(Token::from(&value)),
                    None if VARIABLE_NAME.is_match(&token.surface) => {
                        output.push(Token::new(token.surface.clone(), TokenType::QuotedString));
                    }
                    None => {
                        return fail(
                            format!("Parsing syntax error about {}", token.surface),
                            location,
                        );
                    }
                }
            }
            TokenType::Operator | TokenType::Affectation => {
                while let Some(top) = operators.last() {
                    let pops = top.kind == TokenType::FunctionName
                        || (top.is_operator_or_affectation()
                            && top.precedence() >= token.precedence());
                    if !pops {
                        break;
                    }
                    output.push(operators.pop().unwrap());
                }
                operators.push(token.clone());
            }
            TokenType::CloseParenFunc => {
                while operators
                    .last()
                    .is_some_and(|top| top.kind != TokenType::OpenParenFunc)
                {
                    output.push(operators.pop().unwrap());
                }
                output.push(operators.pop().ok_or(TimError::Fatal)?);
            }
            TokenType::CloseParenMath => {
                loop {
                    let top = operators.last().ok_or(TimError::Fatal)?;
                    if top.kind == TokenType::OpenParenMath {
                        break;
                    }
                    output.push(operators.pop().unwrap());
                }
                operators.pop();
            }
            TokenType::Comma => {
                while operators
                    .last()
                    .is_some_and(|top| top.kind != TokenType::OpenParenFunc)
                {
                    output.push(operators.pop().unwrap());
                }
            }
            TokenType::JsonData | TokenType::Spaces => return Err(TimError::Fatal),
        }
    }
    output.extend(operators.into_iter().rev());
    Ok(output)
}

fn evaluate_reverse_polish(
    queue: &[Token],
    context: &mut TContext,
    memory: &mut Memory,
    location: &StringLocated,
) -> TimResult<TValue> {
    let mut named: HashMap<String, TValue> = HashMap::new();
    let mut stack: Vec<TValue> = Vec::new();
    let mut tokens = queue.iter();
    while let Some(token) = tokens.next() {
        match token.kind {
            TokenType::Number => {
                stack.push(TValue::Int(
                    token.surface.parse().map_err(|_| TimError::Fatal)?,
                ));
            }
            TokenType::QuotedString => stack.push(TValue::string(token.surface.clone())),
            TokenType::JsonData => {
                stack.push(TValue::Json(token.json.clone().ok_or(TimError::Fatal)?));
            }
            TokenType::Affectation => {
                let value = stack.pop().ok_or(TimError::Fatal)?;
                let name = stack.pop().ok_or(TimError::Fatal)?;
                named.insert(name.to_string(), value);
            }
            TokenType::Operator => {
                let right = stack.pop().ok_or(TimError::Fatal)?;
                let left = stack.pop().ok_or(TimError::Fatal)?;
                let Some(operator) = token.operator() else {
                    return fail("bad op", location);
                };
                stack.push(operator.operate(&left, &right)?);
            }
            TokenType::OpenParenFunc => {
                let declared: usize = token.surface.parse().map_err(|_| TimError::Fatal)?;
                let positional = declared.checked_sub(named.len()).ok_or(TimError::Fatal)?;
                let Some(name) = tokens
                    .next()
                    .filter(|next| next.kind == TokenType::FunctionName)
                else {
                    return fail("rpn43", location);
                };
                let signature = FunctionSignature::new(&name.surface, positional);
                let Some(function) = context.function_smart(&signature) else {
                    return fail(
                        format!("Unknown built-in function {}", name.surface),
                        location,
                    );
                };
                if !function.can_cover(positional, &HashSet::default()) {
                    return fail(
                        format!(
                            "Bad number of arguments for {}",
                            function.signature().name()
                        ),
                        location,
                    );
                }
                let split = stack.len().checked_sub(positional).ok_or(TimError::Fatal)?;
                let arguments = stack.split_off(split);
                let result = function
                    .execute_return_function(context, memory, location, &arguments, &named)?;
                named.clear();
                stack.push(result);
            }
            _ => return fail("rpn41", location),
        }
    }
    stack.pop().ok_or(TimError::Fatal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LineLocation;

    fn tokens(text: &str) -> Vec<(TokenType, String)> {
        let mut eater = Eater::new(StringLocated::new(text, LineLocation::new("t", None)));
        let stack = eater.eat_token_stack().unwrap();
        stack
            .tokens
            .into_iter()
            .map(|token| (token.kind, token.surface))
            .collect()
    }

    #[test]
    fn minus_is_subtraction_after_a_value_and_a_sign_after_an_operator() {
        use TokenType::{Number, Operator, Spaces};
        assert_eq!(
            tokens("3 -1"),
            [
                (Number, "3".into()),
                (Spaces, " ".into()),
                (Operator, COMMERCIAL_MINUS_SIGN.to_string()),
                (Number, "1".into())
            ]
        );
        assert_eq!(tokens("3*-1")[2], (Number, "-1".into()));
    }

    #[test]
    fn two_character_operators_are_one_token() {
        assert_eq!(tokens("a<=b")[1], (TokenType::Operator, "<=".into()));
        assert_eq!(tokens("a==b")[1], (TokenType::Operator, "==".into()));
        assert_eq!(tokens("a=b")[1], (TokenType::Affectation, "=".into()));
    }

    #[test]
    fn plain_text_stops_at_breaking_characters() {
        assert_eq!(
            tokens("$var.field[0]+1")[0],
            (TokenType::PlainText, "$var.field[0]".into())
        );
    }

    #[test]
    fn guessing_functions_counts_arguments() {
        let location = StringLocated::new("f(1, g(2), 3)", LineLocation::new("t", None));
        let mut eater = Eater::new(location.clone());
        let mut stack = eater.eat_token_stack().unwrap().without_spaces();
        stack.guess_functions(&location).unwrap();
        let surfaces: Vec<_> = stack
            .tokens
            .iter()
            .map(|token| (token.kind, token.surface.as_str()))
            .collect();
        assert_eq!(surfaces[0], (TokenType::FunctionName, "f"));
        assert_eq!(surfaces[1], (TokenType::OpenParenFunc, "3"));
        assert_eq!(surfaces[5], (TokenType::OpenParenFunc, "1"));
    }
}

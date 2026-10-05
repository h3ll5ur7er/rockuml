//! A cursor over one line, with the scanning primitives every directive parser shares.

use std::sync::LazyLock;

use regex::Regex;

use super::context::TContext;
use super::error::{TimResult, fail};
use super::expression::{TokenStack, eat_one_token};
use super::function::{FunctionArgument, FunctionType, UserFunction};
use super::line_type::{
    is_letter_or_emoji_or_underscore_or_digit, is_letter_or_emoji_or_underscore_or_dollar, is_quote,
};
use super::memory::Memory;
use super::value::TValue;
use crate::java;
use crate::json;
use crate::text::StringLocated;

pub struct Eater {
    line: StringLocated,
    chars: Vec<char>,
    position: usize,
}

impl Eater {
    pub fn new(line: StringLocated) -> Self {
        Self {
            chars: line.text().chars().collect(),
            line,
            position: 0,
        }
    }

    pub fn line(&self) -> &StringLocated {
        &self.line
    }

    pub fn position(&self) -> usize {
        self.position
    }

    /// `'\0'` past the end, as in PlantUML.
    pub fn peek_char(&self) -> char {
        self.chars.get(self.position).copied().unwrap_or('\0')
    }

    pub fn peek_char_n2(&self) -> char {
        self.chars.get(self.position + 1).copied().unwrap_or('\0')
    }

    pub fn has_next_char(&self) -> bool {
        self.position < self.chars.len()
    }

    /// Like Java's `charAt`, reading past the end is a runtime failure.
    pub fn eat_one_char(&mut self) -> TimResult<char> {
        let c = *self
            .chars
            .get(self.position)
            .ok_or(super::error::TimError::Fatal)?;
        self.position += 1;
        Ok(c)
    }

    pub fn eat_all_to_end(&mut self) -> String {
        let rest = self.chars[self.position..].iter().collect();
        self.position = self.chars.len();
        rest
    }

    pub fn skip_spaces(&mut self) {
        while self.position < self.chars.len() && java::is_whitespace(self.chars[self.position]) {
            self.position += 1;
        }
    }

    pub fn skip_until_char(&mut self, c: char) {
        while self.position < self.chars.len() && self.chars[self.position] != c {
            self.position += 1;
        }
    }

    pub fn check_and_eat_char(&mut self, expected: char) -> TimResult<()> {
        if self.peek_char() != expected || !self.has_next_char() {
            return fail("a001", &self.line);
        }
        self.position += 1;
        Ok(())
    }

    pub fn check_and_eat(&mut self, expected: &str) -> TimResult<()> {
        expected
            .chars()
            .try_for_each(|c| self.check_and_eat_char(c))
    }

    pub fn safe_check_and_eat_char(&mut self, expected: char) -> bool {
        if self.has_next_char() && self.peek_char() == expected {
            self.position += 1;
            true
        } else {
            false
        }
    }

    pub fn optionally_eat_char(&mut self, expected: char) {
        self.safe_check_and_eat_char(expected);
    }

    /// An optional `$`, then letters, digits, underscores or emoji.
    pub fn eat_and_get_varname(&mut self) -> TimResult<String> {
        let first = self.eat_one_char()?;
        if !is_letter_or_emoji_or_underscore_or_dollar(first) {
            return fail("a002", &self.line);
        }
        let mut name = first.to_string();
        self.add_up_to_last_letter_or_emoji_or_underscore_or_digit(&mut name);
        Ok(name)
    }

    pub fn eat_and_get_function_name(&mut self) -> TimResult<String> {
        let first = self.eat_one_char()?;
        if !is_letter_or_emoji_or_underscore_or_dollar(first) {
            return fail("a003", &self.line);
        }
        let mut name = first.to_string();
        self.add_up_to_last_letter_or_emoji_or_underscore_or_digit(&mut name);
        Ok(name)
    }

    fn add_up_to_last_letter_or_emoji_or_underscore_or_digit(&mut self, name: &mut String) {
        while let Some(&c) = self.chars.get(self.position) {
            if !is_letter_or_emoji_or_underscore_or_digit(c) {
                return;
            }
            name.push(c);
            self.position += 1;
        }
    }

    pub fn eat_and_get_number(&mut self) -> TimResult<String> {
        let mut number = String::new();
        loop {
            let c = self.peek_char();
            if number.is_empty() && c == '-' {
                number.push(self.eat_one_char()?);
                continue;
            }
            if c == '\0' || !c.is_ascii_digit() {
                return Ok(number);
            }
            number.push(self.eat_one_char()?);
        }
    }

    pub fn eat_and_get_spaces(&mut self) -> TimResult<String> {
        let mut spaces = String::new();
        loop {
            let c = self.peek_char();
            if c == '\0' || !java::is_space_char(c) {
                return Ok(spaces);
            }
            spaces.push(self.eat_one_char()?);
        }
    }

    pub fn eat_and_get_quoted_string(&mut self) -> TimResult<String> {
        let separator = self.peek_char();
        if !is_quote(separator) {
            return fail("quote10", &self.line);
        }
        self.check_and_eat_char(separator)?;
        let mut value = String::new();
        while self.has_next_char() && self.peek_char() != separator {
            value.push(self.chars[self.position]);
            self.position += 1;
        }
        self.check_and_eat_char(separator)?;
        Ok(value)
    }

    /// A quoted string, or raw text up to the next top-level `,` or `)`.
    pub fn eat_and_get_optional_quoted_string(&mut self) -> TimResult<String> {
        if is_quote(self.peek_char()) {
            return self.eat_and_get_quoted_string();
        }
        let mut value = String::new();
        let mut level = 0;
        loop {
            let c = self.peek_char();
            if c == '\0' {
                return fail("until001", &self.line);
            }
            if level == 0 && (c == ',' || c == ')') {
                return Ok(java::trim(&value).to_owned());
            }
            let c = self.eat_one_char()?;
            match c {
                '(' => level += 1,
                ')' => level -= 1,
                _ => {}
            }
            value.push(c);
        }
    }

    /// Whether `name = ...` follows: a named argument in a function call.
    pub fn match_affectation(&self) -> bool {
        static AFFECTATION: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\$?[_\p{L}][_\p{L}0-9]*[\t\n\x0B\x0C\r ]*=").unwrap());
        let rest: String = self.chars[self.position..].iter().collect();
        AFFECTATION.is_match(&rest)
    }

    pub fn eat_expression(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<TValue> {
        let c = self.peek_char();
        if c == '{' || c == '[' {
            let data = self.eat_all_to_end();
            return Ok(TValue::Json(json::parse(&data)?));
        }
        let tokens = self.eat_token_stack()?;
        tokens.get_result(&self.line, context, memory)
    }

    pub fn eat_token_stack(&mut self) -> TimResult<TokenStack> {
        let tokens = self.tokens_until(false)?;
        if tokens.is_empty() {
            return fail("Missing expression", &self.line);
        }
        Ok(tokens)
    }

    pub fn eat_expression_stop_at_colon(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<TValue> {
        let tokens = self.tokens_until(true)?;
        tokens.get_result(&self.line, context, memory)
    }

    fn tokens_until(&mut self, stop_at_colon: bool) -> TimResult<TokenStack> {
        let mut tokens = TokenStack::default();
        let mut last_significant = None;
        while let Some(token) = eat_one_token(last_significant.as_ref(), self, stop_at_colon)? {
            if !token.is_spaces() {
                last_significant = Some(token.clone());
            }
            tokens.push(token);
        }
        Ok(tokens)
    }

    /// `name(arg, arg = default, ...)` as written after `!function`, `!procedure` or `!define`.
    pub fn eat_declare_function(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
        unquoted: bool,
        allow_no_parenthesis: bool,
        function_type: FunctionType,
    ) -> TimResult<UserFunction> {
        let mut arguments = Vec::new();
        let name = self.eat_and_get_function_name()?;
        self.skip_spaces();
        if !self.safe_check_and_eat_char('(') {
            if allow_no_parenthesis {
                return Ok(UserFunction::new(name, arguments, unquoted, function_type));
            }
            return fail("Missing opening parenthesis", &self.line);
        }
        loop {
            self.skip_spaces();
            let c = self.peek_char();
            if is_letter_or_emoji_or_underscore_or_dollar(c) {
                let argument_name = self.eat_and_get_varname()?;
                self.skip_spaces();
                let default = if self.peek_char() == '=' {
                    self.eat_one_char()?;
                    let mut tokens = TokenStack::eat_until_close_parenthesis_or_comma(self)?;
                    tokens.guess_functions(&self.line)?;
                    Some(tokens.get_result(&self.line, context, memory)?)
                } else {
                    None
                };
                arguments.push(FunctionArgument::new(argument_name, default));
            } else if c == ',' {
                self.check_and_eat_char(',')?;
            } else if c == ')' {
                self.check_and_eat_char(')')?;
                break;
            } else {
                return fail("Error in function definition", &self.line);
            }
        }
        self.skip_spaces();
        Ok(UserFunction::new(name, arguments, unquoted, function_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LineLocation;

    fn eater(text: &str) -> Eater {
        Eater::new(StringLocated::new(text, LineLocation::new("t", None)))
    }

    #[test]
    fn varnames_may_start_with_a_dollar() {
        let mut eater = eater("$my_var2 = 1");
        assert_eq!(eater.eat_and_get_varname().unwrap(), "$my_var2");
        assert_eq!(eater.peek_char(), ' ');
    }

    #[test]
    fn unquoted_arguments_stop_at_top_level_comma() {
        let mut eater = eater("f(a, b), c");
        assert_eq!(
            eater.eat_and_get_optional_quoted_string().unwrap(),
            "f(a, b)"
        );
        assert_eq!(eater.peek_char(), ',');
    }

    #[test]
    fn quoted_strings_end_at_the_matching_quote() {
        let mut eater = eater("'it\"s' rest");
        assert_eq!(eater.eat_and_get_quoted_string().unwrap(), "it\"s");
    }

    #[test]
    fn numbers_accept_a_leading_minus() {
        let mut eater = eater("-42x");
        assert_eq!(eater.eat_and_get_number().unwrap(), "-42");
    }

    #[test]
    fn named_arguments_are_detected_even_before_a_comparison() {
        assert!(eater("$x = 1").match_affectation());
        assert!(eater("$x == 1").match_affectation());
        assert!(!eater("\"x\"").match_affectation());
    }
}

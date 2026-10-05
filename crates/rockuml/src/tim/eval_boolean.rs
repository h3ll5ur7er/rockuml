//! The `!ifdef` condition language: names combined with `!`, `&`/`&&`, `|`/`||` and parentheses.

use crate::java;

/// `None` when the expression does not parse (PlantUML throws there).
pub fn eval(expression: &str, mut is_defined: impl FnMut(&str) -> bool) -> Option<bool> {
    let mut parser = Parser {
        chars: expression.chars().collect(),
        position: 0,
        is_defined: &mut is_defined,
    };
    let value = parser.expression()?;
    parser.skip_spaces();
    (parser.position >= parser.chars.len()).then_some(value)
}

struct Parser<'a> {
    chars: Vec<char>,
    position: usize,
    is_defined: &'a mut dyn FnMut(&str) -> bool,
}

impl Parser<'_> {
    fn current(&self) -> char {
        self.chars.get(self.position).copied().unwrap_or('\0')
    }

    fn skip_spaces(&mut self) {
        while self.current() == ' ' {
            self.position += 1;
        }
    }

    fn eat(&mut self, expected: char) -> bool {
        self.skip_spaces();
        if self.current() == expected {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn expression(&mut self) -> Option<bool> {
        let mut value = self.term()?;
        while self.eat('|') {
            self.eat('|');
            value |= self.term()?;
        }
        Some(value)
    }

    fn term(&mut self) -> Option<bool> {
        let mut value = self.factor()?;
        while self.eat('&') {
            self.eat('&');
            value &= self.factor()?;
        }
        Some(value)
    }

    fn factor(&mut self) -> Option<bool> {
        if self.eat('!') {
            return self.factor().map(|value| !value);
        }
        if self.eat('(') {
            let value = self.expression()?;
            self.eat(')');
            return Some(value);
        }
        let start = self.position;
        while is_identifier(self.current()) {
            self.position += 1;
        }
        if start == self.position {
            return None;
        }
        let name: String = self.chars[start..self.position].iter().collect();
        Some((self.is_defined)(&name))
    }
}

fn is_identifier(c: char) -> bool {
    c == '_' || c == '$' || java::is_letter_or_digit(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_with(expression: &str, defined: &[&str]) -> Option<bool> {
        eval(expression, |name| defined.contains(&name))
    }

    #[test]
    fn combines_names_with_boolean_operators() {
        assert_eq!(eval_with("A", &["A"]), Some(true));
        assert_eq!(eval_with("!A", &["A"]), Some(false));
        assert_eq!(eval_with("A && B", &["A"]), Some(false));
        assert_eq!(eval_with("A || B", &["B"]), Some(true));
        assert_eq!(eval_with("!(A | B) & C", &["C"]), Some(true));
    }

    #[test]
    fn rejects_malformed_expressions() {
        assert_eq!(eval_with("A +", &[]), None);
        assert_eq!(eval_with("", &[]), None);
    }
}

//! An EBNF rule read into tokens and put in postfix order (PlantUML's `EbnfExpression`, `Token`, `Symbol` and
//! `ShuntingYard`).

use crate::command::{CharInspector, PastTheEnd};
use crate::java;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names, reason = "PlantUML's names")]
pub(super) enum Symbol {
    Literal,
    /// `=`
    Definition,
    /// `,`
    Concatenation,
    /// `|`
    Alternation,
    /// `[`
    OptionalOpen,
    /// `]`
    OptionalClose,
    Optional,
    /// `*`
    RepetitionSymbol,
    /// `{`
    RepetitionOpen,
    /// `}`
    RepetitionClose,
    /// `}-`
    RepetitionMinusClose,
    RepetitionZeroOrMore,
    RepetitionOneOrMore,
    /// `(`
    GroupingOpen,
    /// `)`
    GroupingClose,
    /// `"…"`
    TerminalString1,
    /// `'…'`
    TerminalString2,
    /// `(* … *)`, before it is known to belong above or below what it follows.
    CommentToken,
    CommentBelow,
    CommentAbove,
    /// `? … ?`
    SpecialSequence,
    /// `-`, the exception.
    Not,
}

impl Symbol {
    fn priority(self) -> u8 {
        match self {
            Self::RepetitionSymbol => 3,
            Self::Concatenation => 2,
            Self::Alternation => 1,
            _ => unreachable!("only operators have a priority"),
        }
    }

    fn is_operator(self) -> bool {
        matches!(
            self,
            Self::Concatenation | Self::Alternation | Self::RepetitionSymbol
        )
    }

    /// What the tile engine draws as a box.
    pub(super) fn is_terminal_or_literal(self) -> bool {
        matches!(
            self,
            Self::TerminalString1 | Self::TerminalString2 | Self::Literal | Self::SpecialSequence
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Token {
    pub symbol: Symbol,
    pub data: Option<String>,
}

impl Token {
    fn new(symbol: Symbol) -> Self {
        Self { symbol, data: None }
    }

    fn with_data(symbol: Symbol, data: String) -> Self {
        Self {
            symbol,
            data: Some(data),
        }
    }
}

/// The tokens of a rule, up to its `;`; none if it holds a character EBNF does not have.
pub(super) fn tokenize(it: &mut CharInspector) -> Result<Vec<Token>, PastTheEnd> {
    let mut tokens = Vec::new();
    loop {
        let ch = it.peek(0);
        if java::is_whitespace(ch) {
        } else if ch == '-' {
            tokens.push(Token::new(Symbol::Not));
        } else if is_letter_or_digit(ch) {
            tokens.push(Token::with_data(Symbol::Literal, read_litteral(it)?));
            continue;
        } else if ch == '*' {
            tokens.push(Token::new(Symbol::RepetitionSymbol));
        } else if ch == '(' && it.peek(1) == '*' {
            let comment = read_comment(it)?;
            if !java::trim(&comment).is_empty() {
                tokens.push(Token::with_data(Symbol::CommentToken, comment));
            }
            continue;
        } else if ch == ',' {
            tokens.push(Token::new(Symbol::Concatenation));
        } else if ch == '|' {
            tokens.push(Token::new(Symbol::Alternation));
        } else if ch == '=' {
            tokens.push(Token::new(Symbol::Definition));
        } else if ch == '(' {
            tokens.push(Token::new(Symbol::GroupingOpen));
        } else if ch == ')' {
            tokens.push(Token::new(Symbol::GroupingClose));
        } else if ch == '[' {
            tokens.push(Token::new(Symbol::OptionalOpen));
        } else if ch == ']' {
            tokens.push(Token::new(Symbol::OptionalClose));
        } else if ch == '{' {
            tokens.push(Token::new(Symbol::RepetitionOpen));
        } else if ch == '}' && it.peek(1) == '-' {
            tokens.push(Token::new(Symbol::RepetitionMinusClose));
            it.jump()?;
        } else if ch == '}' {
            tokens.push(Token::new(Symbol::RepetitionClose));
        } else if ch == ';' || ch == '\0' {
            break;
        } else if let Some(symbol) = string_symbol(ch) {
            tokens.push(Token::with_data(symbol, protect(read_string(it)?)));
        } else {
            return Ok(Vec::new());
        }
        it.jump()?;
    }
    Ok(tokens)
}

fn string_symbol(separator: char) -> Option<Symbol> {
    match separator {
        '"' => Some(Symbol::TerminalString1),
        '\'' => Some(Symbol::TerminalString2),
        '?' => Some(Symbol::SpecialSequence),
        _ => None,
    }
}

/// An empty string is drawn as a space.
fn protect(literal: String) -> String {
    if literal.is_empty() {
        " ".to_owned()
    } else {
        literal
    }
}

/// Up to the closing separator, which is left to read.
fn read_string(it: &mut CharInspector) -> Result<String, PastTheEnd> {
    let separator = it.peek(0);
    it.jump()?;
    let mut sb = String::new();
    loop {
        let ch = it.peek(0);
        if ch == separator {
            return Ok(sb);
        }
        sb.push(ch);
        it.jump()?;
    }
}

fn read_litteral(it: &mut CharInspector) -> Result<String, PastTheEnd> {
    let mut sb = String::new();
    loop {
        let ch = it.peek(0);
        if !is_letter_or_digit(ch) {
            return Ok(sb);
        }
        sb.push(ch);
        it.jump()?;
    }
}

/// Between `(*` and `*)`, both read.
fn read_comment(it: &mut CharInspector) -> Result<String, PastTheEnd> {
    let mut sb = String::new();
    it.jump()?;
    it.jump()?;
    loop {
        let ch = it.peek(0);
        if ch == '\0' {
            return Ok(sb);
        }
        if ch == '*' && it.peek(1) == ')' {
            it.jump()?;
            it.jump()?;
            return Ok(sb);
        }
        sb.push(ch);
        it.jump()?;
    }
}

/// A supplementary character is no letter to Java, and makes the rule unparsable.
fn is_letter_or_digit(ch: char) -> bool {
    ch == '-' || ch == '_' || java::is_one_char(ch) && java::is_letter_or_digit(ch)
}

/// Where PlantUML's `ShuntingYard` throws, which draws the rule as a syntax error.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct SyntaxError;

/// The tokens in postfix order, comments placed above or below what they follow; empty when an optional or
/// repeated part is left open (`ShuntingYard`).
pub(super) fn shunting_yard(tokens: &[Token]) -> Result<Vec<Token>, SyntaxError> {
    let mut yard = ShuntingYard::default();
    for token in tokens {
        yard.read(token)?;
    }
    while let Some(token) = yard.operator_stack.pop() {
        match token.symbol {
            Symbol::Optional | Symbol::RepetitionOneOrMore | Symbol::RepetitionZeroOrMore => {
                return Ok(Vec::new());
            }
            Symbol::CommentToken => yard.ouput_queue.push(Token {
                symbol: Symbol::CommentBelow,
                data: token.data,
            }),
            _ => yard.ouput_queue.push(token),
        }
    }
    Ok(yard.ouput_queue)
}

#[derive(Default)]
struct ShuntingYard {
    ouput_queue: Vec<Token>,
    /// Its top is last.
    operator_stack: Vec<Token>,
}

impl ShuntingYard {
    fn read(&mut self, token: &Token) -> Result<(), SyntaxError> {
        let symbol = token.symbol;
        if symbol.is_terminal_or_literal() {
            self.ouput_queue.push(token.clone());
            self.move_comments_above();
        } else if symbol == Symbol::CommentToken {
            self.operator_stack.push(token.clone());
        } else if symbol == Symbol::Not {
            self.operator_stack.push(Token::new(Symbol::Concatenation));
            self.operator_stack.push(token.clone());
        } else if symbol.is_operator() {
            while self.operator_with_greater_precedence_on_top(symbol) || self.comment_on_top() {
                self.move_top_below()?;
            }
            self.operator_stack.push(token.clone());
        } else if symbol == Symbol::GroupingOpen {
            self.operator_stack.push(token.clone());
        } else if symbol == Symbol::GroupingClose {
            while self.top()?.symbol != Symbol::GroupingOpen {
                self.move_top_below()?;
            }
            self.operator_stack.pop();
        } else if symbol == Symbol::OptionalOpen {
            self.operator_stack.push(Token::new(Symbol::Optional));
        } else if symbol == Symbol::RepetitionOpen {
            self.operator_stack
                .push(Token::new(Symbol::RepetitionZeroOrMore));
        } else if let Some((opening, output)) = closing(symbol) {
            while self.operator_on_top() || self.comment_on_top() {
                self.move_top_below()?;
            }
            let first = self.operator_stack.pop().ok_or(SyntaxError)?;
            if first.symbol != opening {
                return Err(SyntaxError);
            }
            self.ouput_queue.push(Token::new(output));
            self.move_comments_above();
        } else {
            return Err(SyntaxError);
        }
        Ok(())
    }

    fn top(&self) -> Result<&Token, SyntaxError> {
        self.operator_stack.last().ok_or(SyntaxError)
    }

    fn move_comments_above(&mut self) {
        while self.comment_on_top() {
            let comment = self.operator_stack.pop().expect("a comment is on top");
            self.ouput_queue.push(Token {
                symbol: Symbol::CommentAbove,
                data: comment.data,
            });
        }
    }

    fn move_top_below(&mut self) -> Result<(), SyntaxError> {
        let mut top = self.operator_stack.pop().ok_or(SyntaxError)?;
        if top.symbol == Symbol::CommentToken {
            top.symbol = Symbol::CommentBelow;
        }
        self.ouput_queue.push(top);
        Ok(())
    }

    fn comment_on_top(&self) -> bool {
        self.operator_stack
            .last()
            .is_some_and(|top| top.symbol == Symbol::CommentToken)
    }

    fn operator_on_top(&self) -> bool {
        self.operator_stack
            .last()
            .is_some_and(|top| top.symbol.is_operator())
    }

    fn operator_with_greater_precedence_on_top(&self, symbol: Symbol) -> bool {
        self.operator_stack.last().is_some_and(|top| {
            top.symbol.is_operator() && top.symbol.priority() > symbol.priority()
        })
    }
}

/// What a closing symbol closes, and what it puts out.
fn closing(symbol: Symbol) -> Option<(Symbol, Symbol)> {
    match symbol {
        Symbol::OptionalClose => Some((Symbol::Optional, Symbol::Optional)),
        Symbol::RepetitionClose => {
            Some((Symbol::RepetitionZeroOrMore, Symbol::RepetitionZeroOrMore))
        }
        Symbol::RepetitionMinusClose => {
            Some((Symbol::RepetitionZeroOrMore, Symbol::RepetitionOneOrMore))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::BlocLines;

    fn tokens(lines: &[&str]) -> Vec<Token> {
        tokenize(&mut BlocLines::from_texts(lines).inspector()).expect("the rule ends")
    }

    fn postfix(rule: &str) -> String {
        let tokens = tokens(&[rule]);
        let queue = shunting_yard(&tokens[2..]).expect("the rule is balanced");
        queue
            .iter()
            .map(|token| match &token.data {
                Some(data) => format!("{:?}:{data}", token.symbol),
                None => format!("{:?}", token.symbol),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn literals_run_across_lines_and_strings_keep_their_spaces() {
        let read = tokens(&["rule = ab", "cd, \"x y\", '', ? s ?;"]);
        let data: Vec<_> = read.iter().map(|token| token.data.as_deref()).collect();
        assert_eq!(
            data,
            [
                Some("rule"),
                None,
                Some("abcd"),
                None,
                Some("x y"),
                None,
                Some(" "),
                None,
                Some(" s ")
            ]
        );
    }

    #[test]
    fn an_unknown_character_leaves_no_token() {
        assert_eq!(tokens(&["a = b # c;"]), []);
    }

    #[test]
    fn a_supplementary_character_is_no_letter() {
        assert_eq!(tokens(&["r = x\u{1D49C}y ;"]), []);
    }

    #[test]
    fn an_unclosed_string_reads_past_the_end() {
        let mut it = BlocLines::from_texts(&["a = \"b"]).inspector();
        assert_eq!(tokenize(&mut it), Err(PastTheEnd));
    }

    #[test]
    fn concatenation_binds_tighter_than_alternation() {
        assert_eq!(
            postfix("r = a | b, c;"),
            "Literal:a Literal:b Literal:c Concatenation Alternation"
        );
    }

    #[test]
    fn brackets_become_functions_of_what_they_hold_and_concatenations_group_to_the_right() {
        assert_eq!(
            postfix("r = [a], {b}, {c}-;"),
            "Literal:a Optional Literal:b RepetitionZeroOrMore Literal:c RepetitionOneOrMore \
             Concatenation Concatenation"
        );
    }

    #[test]
    fn comments_go_above_what_follows_or_below_what_precedes() {
        assert_eq!(
            postfix("r = (* x *) a, b (* y *);"),
            "Literal:a CommentAbove: x  Literal:b CommentBelow: y  Concatenation"
        );
    }

    #[test]
    fn an_open_bracket_leaves_nothing_and_a_stray_one_is_unbalanced() {
        assert_eq!(postfix("r = [ a;"), "");
        let tokens = tokens(&["r = a ) ;"]);
        assert_eq!(shunting_yard(&tokens[2..]), Err(SyntaxError));
    }
}

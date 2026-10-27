//! Builds a rule's tiles from its tokens in postfix order (PlantUML's `EbnfEngine`).

use super::expression::{Symbol, Token};
use super::tile::{
    BoxKind, ETile, ETileBox, ETileConcatenation, ETileNot, ETileWithCircles, TileStyle,
};
use crate::skin::SkinParam;

/// Where PlantUML's engine throws, which draws the rule as a syntax error: an operator finds too few tiles, or
/// a symbol is left that no tile draws, such as an unclosed `(`.
#[derive(Debug)]
pub(super) struct Thrown;

pub(super) struct EbnfEngine<'a> {
    /// Its top is last.
    stack: Vec<ETile>,
    style: TileStyle,
    skin_param: &'a SkinParam,
}

impl<'a> EbnfEngine<'a> {
    pub(super) fn new(style: TileStyle, skin_param: &'a SkinParam) -> Self {
        Self {
            stack: Vec::new(),
            style,
            skin_param,
        }
    }

    /// The tiles `tokens` make, or `None` when they do not make one rule.
    pub(super) fn run(mut self, tokens: &[Token]) -> Result<Option<ETileWithCircles>, Thrown> {
        for token in tokens {
            self.apply(token)?;
        }
        Ok(self.get_text_block())
    }

    fn apply(&mut self, token: &Token) -> Result<(), Thrown> {
        let data = token.data.as_deref().unwrap_or_default();
        let skin_param = self.skin_param;
        match token.symbol {
            Symbol::TerminalString1 | Symbol::TerminalString2 => self.push(data, BoxKind::Terminal),
            Symbol::SpecialSequence => self.push(data, BoxKind::SpecialSequence),
            Symbol::Literal => self.push(data, BoxKind::Literal),
            Symbol::CommentAbove => self.top()?.add_comment_above(data, skin_param),
            Symbol::CommentBelow => self.top()?.add_comment_below(data, skin_param),
            Symbol::Alternation => self.alternation()?,
            Symbol::Concatenation => self.concatenation()?,
            Symbol::Optional => {
                let arg1 = self.pop()?;
                self.stack.push(ETile::optional(arg1));
            }
            Symbol::Not => {
                let arg1 = self.pop()?;
                self.stack
                    .push(ETile::Not(ETileNot::new(arg1, self.style.font.clone())));
            }
            Symbol::RepetitionZeroOrMore => {
                let arg1 = self.pop()?;
                self.stack.push(ETile::zero_or_more(arg1));
            }
            Symbol::RepetitionOneOrMore => {
                let arg1 = self.pop()?;
                self.stack.push(ETile::one_or_more(arg1, None));
            }
            Symbol::RepetitionSymbol => self.repetition_symbol()?,
            _ => return Err(Thrown),
        }
        Ok(())
    }

    fn push(&mut self, value: &str, kind: BoxKind) {
        self.stack.push(ETile::Box(ETileBox::new(
            value.to_owned(),
            kind,
            self.style.clone(),
        )));
    }

    fn pop(&mut self) -> Result<ETile, Thrown> {
        self.stack.pop().ok_or(Thrown)
    }

    fn top(&mut self) -> Result<&mut ETile, Thrown> {
        self.stack.last_mut().ok_or(Thrown)
    }

    /// `3 * x`: the count above a loop, which may be skipped when the count is 0.
    fn repetition_symbol(&mut self) -> Result<(), Thrown> {
        let arg1 = self.pop()?;
        let repetition_label = self.pop()?.repetition_label().to_owned();
        let label = format!("{repetition_label}\u{00D7}");
        let mut tile = ETile::one_or_more(arg1, Some((label, self.style.font.bigger(-2.0))));
        if repetition_label == "0" {
            tile = ETile::optional(tile);
        }
        self.stack.push(tile);
        Ok(())
    }

    fn alternation(&mut self) -> Result<(), Thrown> {
        let arg1 = self.pop()?;
        let arg2 = self.pop()?;
        self.stack.push(ETile::alternation(arg1, arg2));
        Ok(())
    }

    fn concatenation(&mut self) -> Result<(), Thrown> {
        let mut arg1 = self.pop()?;
        let arg2 = self.pop()?;
        if matches!(arg1, ETile::Concatenation(_)) {
            arg1.push(arg2);
            self.stack.push(arg1);
            return Ok(());
        }
        let concatenation = match arg2 {
            ETile::Concatenation(mut concatenation) => {
                concatenation.push_last(arg1);
                concatenation
            }
            arg2 => ETileConcatenation::of(arg2, arg1),
        };
        self.stack.push(ETile::Concatenation(concatenation));
        Ok(())
    }

    fn get_text_block(mut self) -> Option<ETileWithCircles> {
        if self.stack.len() != 1 {
            return None;
        }
        let line_color = self.style.line_color.clone();
        self.stack
            .pop()
            .map(|tile| ETileWithCircles::new(tile, line_color))
    }
}

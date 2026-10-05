//! `com.plantuml.ubrex.builder`, which diagram commands use to assemble expressions from parts:
//! `UBrexLeaf` is [`UnicodeBracketedExpression::build`], the other classes are the constructors below.

use super::UnicodeBracketedExpression;
use super::challenge::Challenge;

impl UnicodeBracketedExpression {
    /// `UBrexConcat`.
    pub fn concat(parts: impl IntoIterator<Item = Self>) -> Self {
        Self::from(Challenge::List(challenges(parts)))
    }

    /// `UBrexNamed`.
    pub fn named(name: &str, part: Self) -> Self {
        Self::from(Challenge::Named {
            name: name.to_owned(),
            challenges: vec![part.challenge],
        })
    }

    /// `UBrexOr`.
    pub fn or(parts: impl IntoIterator<Item = Self>) -> Self {
        Self::from(Challenge::Alternative(challenges(parts)))
    }

    /// `UBrexOptional`.
    pub fn optional(part: Self) -> Self {
        Self::from(Challenge::Optional(Box::new(part.challenge)))
    }

    /// `UBrexZeroOrMore`.
    pub fn zero_or_more(part: Self) -> Self {
        Self::from(Challenge::ZeroOrMore(Box::new(part.challenge)))
    }

    /// `UBrexOneOrMore`.
    pub fn one_or_more(part: Self) -> Self {
        Self::from(Challenge::OneOrMore(Box::new(part.challenge)))
    }

    /// `UBrexUpto`: one or more `what` until `stop` matches, then `stop`.
    pub fn upto(what: Self, stop: Self) -> Self {
        Self::from(Challenge::List(vec![
            Challenge::OneOrMoreUpTo {
                origin: Box::new(what.challenge),
                stop_condition: Box::new(stop.challenge.clone()),
            },
            stop.challenge,
        ]))
    }

    pub fn end() -> Self {
        Self::build("〒$")
    }

    pub fn space_one_or_more() -> Self {
        Self::build("〇+〴s")
    }

    pub fn space_zero_or_more() -> Self {
        Self::build("〇*〴s")
    }
}

impl From<Challenge> for UnicodeBracketedExpression {
    fn from(challenge: Challenge) -> Self {
        Self { challenge }
    }
}

fn challenges(parts: impl IntoIterator<Item = UnicodeBracketedExpression>) -> Vec<Challenge> {
    parts.into_iter().map(|part| part.challenge).collect()
}

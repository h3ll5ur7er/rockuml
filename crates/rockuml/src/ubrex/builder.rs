//! Expressions composed from parts instead of written as one string (`com.plantuml.ubrex.builder`).

use super::UnicodeBracketedExpression;
use super::challenge::Challenge;
use super::parser::parse_and_build;

/// A part of an expression (`UBrexPart` and its subclasses).
pub(crate) struct UBrexPart(Challenge);

impl UBrexPart {
    /// `UBrexLeaf`: a part written in the string form.
    pub(crate) fn leaf(definition: &str) -> Self {
        let definition: Vec<u16> = definition.encode_utf16().collect();
        Self(Challenge::List(parse_and_build(&definition)))
    }

    pub(crate) fn end() -> Self {
        Self::leaf("〒$")
    }

    pub(crate) fn space_one_or_more() -> Self {
        Self::leaf("〇+〴s")
    }

    pub(crate) fn space_zero_or_more() -> Self {
        Self::leaf("〇*〴s")
    }

    /// `UBrexConcat.build`.
    pub(crate) fn concat(parts: Vec<UBrexPart>) -> Self {
        Self(Challenge::List(
            parts.into_iter().map(|part| part.0).collect(),
        ))
    }

    /// `UBrexNamed`: what `part` matches is captured under `name`.
    pub(crate) fn named(name: &str, part: UBrexPart) -> Self {
        Self(Challenge::Named {
            name: name.to_owned(),
            challenges: vec![part.0],
        })
    }

    /// `UBrexOr`: the first alternative that matches.
    pub(crate) fn or(alternatives: Vec<UBrexPart>) -> Self {
        Self(Challenge::Alternative(
            alternatives.into_iter().map(|part| part.0).collect(),
        ))
    }

    /// `UBrexOptional`.
    pub(crate) fn optional(part: UBrexPart) -> Self {
        Self(Challenge::Optional(Box::new(part.0)))
    }

    /// `UBrexZeroOrMore`.
    pub(crate) fn zero_or_more(part: UBrexPart) -> Self {
        Self(Challenge::ZeroOrMore(Box::new(part.0)))
    }

    /// `UBrexOneOrMore`.
    pub(crate) fn one_or_more(part: UBrexPart) -> Self {
        Self(Challenge::OneOrMore(Box::new(part.0)))
    }

    /// `UBrexUpto`: `what` repeated up to where `stop` matches, then `stop`.
    pub(crate) fn upto(what: UBrexPart, stop: UBrexPart) -> Self {
        Self(Challenge::List(vec![
            Challenge::OneOrMoreUpTo {
                origin: Box::new(what.0),
                stop_condition: Box::new(stop.0.clone()),
            },
            stop.0,
        ]))
    }

    /// `UnicodeBracketedExpression.from`.
    pub(crate) fn build(self) -> UnicodeBracketedExpression {
        UnicodeBracketedExpression { challenge: self.0 }
    }
}

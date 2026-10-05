//! PlantUML's Unicode Bracketed Expressions (`com.plantuml.ubrex`): a regex dialect written with CJK
//! brackets (`〶$NAME=〇+「〤>」`, `【a┇b】`...) that matches without backtracking.
//!
//! Java runs it over UTF-16 units, and so does this port: a pattern step such as `〴.` consumes one unit,
//! and letter classes never match a surrogate half. Callers work with `&str`. A Java match can end
//! between the two halves of a surrogate pair, which no `&str` can express; there the accepted text and
//! the captured values widen to the whole character. No PlantUML pattern stops inside a pair.
//!
//! Malformed patterns panic, as Java throws: PlantUML only builds them from constants.

mod challenge;
mod char_set;
mod parser;

use challenge::{Challenge, TextNavigator};

#[derive(Clone, Debug)]
pub(crate) struct UnicodeBracketedExpression {
    challenge: Challenge,
}

impl UnicodeBracketedExpression {
    pub(crate) fn build(ubrex: &str) -> Self {
        let definition: Vec<u16> = ubrex.encode_utf16().collect();
        Self {
            challenge: Challenge::List(parser::parse_and_build(&definition)),
        }
    }

    /// Java's `match(text, 0)`, `None` where its `startMatch()` is false.
    ///
    /// # Panics
    /// Where Java throws: a repetition of something that matched empty text ("infinite loop").
    pub(crate) fn match_at<'a>(&self, text: &'a str) -> Option<UMatcher<'a>> {
        let content: Vec<u16> = text.encode_utf16().collect();
        let result = self
            .challenge
            .run_challenge(TextNavigator::build(&content), 0)?;
        Some(UMatcher {
            accepted_match: utf16_slice(text, 0..result.full_capture_length),
            values: result
                .capture
                .entries()
                .iter()
                .map(|entry| (entry.key.clone(), utf16_slice(text, entry.value.clone())))
                .collect(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct UMatcher<'a> {
    accepted_match: &'a str,
    values: Vec<(String, &'a str)>,
}

impl<'a> UMatcher<'a> {
    pub(crate) fn accepted_match(&self) -> &'a str {
        self.accepted_match
    }

    /// Every value captured under `key`, in match order; nested names are keyed `OUTER/INNER`.
    pub(crate) fn find_values_by_key(&self, key: &str) -> Vec<&'a str> {
        self.values
            .iter()
            .filter(|(entry_key, _)| entry_key == key)
            .map(|&(_, value)| value)
            .collect()
    }
}

/// The part of `text` between two UTF-16 indexes, widened to whole characters.
fn utf16_slice(text: &str, units: std::ops::Range<usize>) -> &str {
    &text[byte_offset(text, units.start, false)..byte_offset(text, units.end, true)]
}

fn byte_offset(text: &str, unit: usize, round_up_inside_pair: bool) -> usize {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units == unit {
            return byte;
        }
        units += ch.len_utf16();
        if units > unit {
            return if round_up_inside_pair {
                byte + ch.len_utf8()
            } else {
                byte
            };
        }
    }
    text.len()
}

#[cfg(test)]
mod tests;

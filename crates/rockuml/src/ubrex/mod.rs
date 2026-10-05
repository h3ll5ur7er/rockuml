//! PlantUML's Unicode Bracketed Expressions (`com.plantuml.ubrex`): a regex dialect written with CJK
//! brackets (`〶$NAME=〇+「〤>」`, `【a┇b】`...) that matches without backtracking.
//!
//! Java runs it over UTF-16 units, and so does this port: a pattern step such as `〴.` consumes one unit,
//! letter classes never match a surrogate half, and a look-behind sees a surrogate pair reversed. Callers
//! work with `&str`, so positions and results are UTF-8 byte offsets into the text they pass in. A Java
//! match can end between the two halves of a surrogate pair, which no `&str` can express; there the
//! accepted text and the captured values widen to the whole character, while `start_match` and
//! `exact_match` still give Java's answers. No PlantUML pattern stops inside a pair.
//!
//! Malformed patterns panic, as Java throws: PlantUML only builds them from constants.

mod builder;
mod challenge;
mod char_set;
mod parser;

use challenge::{Challenge, TextNavigator};

#[derive(Clone, Debug)]
pub struct UnicodeBracketedExpression {
    challenge: Challenge,
}

impl UnicodeBracketedExpression {
    pub fn build(ubrex: &str) -> Self {
        let definition: Vec<u16> = ubrex.encode_utf16().collect();
        Self {
            challenge: Challenge::List(parser::parse_and_build(&definition)),
        }
    }

    /// Java's `match(String, int)`; `position` is a byte offset into `text`.
    ///
    /// # Panics
    /// Where Java throws: a repetition of something that matched empty text ("infinite loop"), or a
    /// look-behind inside a look-behind.
    pub fn match_at<'a>(&self, text: &'a str, position: usize) -> UMatcher<'a> {
        let content: Vec<u16> = text.encode_utf16().collect();
        let start = text[..position].encode_utf16().count();
        let Some(result) = self
            .challenge
            .run_challenge(TextNavigator::build(&content), start)
        else {
            return UMatcher::default();
        };
        let end = start + result.full_capture_length;
        UMatcher {
            accepted_match: utf16_slice(text, start..end),
            start_match: true,
            exact_match: end == content.len(),
            values: result
                .capture
                .entries()
                .iter()
                .map(|entry| (entry.key.clone(), utf16_slice(text, entry.value.clone())))
                .collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct UMatcher<'a> {
    accepted_match: &'a str,
    start_match: bool,
    exact_match: bool,
    values: Vec<(String, &'a str)>,
}

impl<'a> UMatcher<'a> {
    /// Whether the expression matched at the position, possibly leaving text after it.
    pub fn start_match(&self) -> bool {
        self.start_match
    }

    /// Whether the expression matched everything from the position to the end of the text.
    pub fn exact_match(&self) -> bool {
        self.exact_match
    }

    /// Empty when nothing matched.
    pub fn accepted_match(&self) -> &'a str {
        self.accepted_match
    }

    /// Every value captured under `key`, in match order; nested names are keyed `OUTER/INNER`.
    pub fn find_values_by_key(&self, key: &str) -> Vec<&'a str> {
        self.values
            .iter()
            .filter(|(entry_key, _)| entry_key == key)
            .map(|&(_, value)| value)
            .collect()
    }

    /// The values of the first captured key starting with `key_prefix`; empty where Java returns null.
    pub fn find_first_values_by_key_prefix(&self, key_prefix: &str) -> Vec<&'a str> {
        self.values
            .iter()
            .find(|(key, _)| key.starts_with(key_prefix))
            .map(|(key, _)| self.find_values_by_key(key))
            .unwrap_or_default()
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

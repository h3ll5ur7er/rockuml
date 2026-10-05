//! The compiled expression tree (`Challenge` and its implementations) and how it runs over a text.

use std::ops::{Range, RangeInclusive};

use super::char_set::{ChallengeCharSet, CharClass, ensure_lowercase};

#[derive(Clone, Debug)]
pub(super) enum Challenge {
    /// Holds the lowercase form: every comparison ignores ASCII case.
    SingleChar(u16),
    CharClass(CharClass),
    CharSet(ChallengeCharSet),
    EndOfText,
    /// `CompositeList`.
    List(Vec<Challenge>),
    /// `CompositeNamed`.
    Named {
        name: String,
        challenges: Vec<Challenge>,
    },
    Alternative(Vec<Challenge>),
    Optional(Box<Challenge>),
    ZeroOrMore(Box<Challenge>),
    OneOrMore(Box<Challenge>),
    Repetition(Repetition, Box<Challenge>),
    /// `ChallengeOneOrMoreUpToOldVersion`: the stop condition is only peeked at, the parser places it after
    /// this challenge as well.
    OneOrMoreUpTo {
        origin: Box<Challenge>,
        stop_condition: Box<Challenge>,
    },
    UpTo(Box<Challenge>),
}

impl Challenge {
    /// `None` is Java's `NO_MATCH`.
    pub(super) fn run_challenge(
        &self,
        text: TextNavigator,
        position: usize,
    ) -> Option<ChallengeResult> {
        match self {
            Self::SingleChar(ch) => {
                single_unit(text, position, |unit| ensure_lowercase(unit) == *ch)
            }
            Self::CharClass(class) => single_unit(text, position, |unit| class.matches(unit)),
            Self::CharSet(set) => single_unit(text, position, |unit| set.matches(unit)),
            Self::EndOfText => (position >= text.length()).then(ChallengeResult::default),
            Self::List(challenges) => run_all(challenges, text, position),
            Self::Named { name, challenges } => {
                let mut result = run_all(challenges, text, position)?;
                let value = position..position + result.full_capture_length;
                result.capture.prefix_keys(name);
                result.capture.add(name.clone(), value);
                Some(result)
            }
            Self::Alternative(alternatives) => alternatives
                .iter()
                .find_map(|alternative| alternative.run_challenge(text, position)),
            Self::Optional(origin) => {
                Some(origin.run_challenge(text, position).unwrap_or_default())
            }
            Self::ZeroOrMore(origin) => Some(repeat_greedily(origin, text, position).0),
            Self::OneOrMore(origin) => {
                let (result, count) = repeat_greedily(origin, text, position);
                (count > 0).then_some(result)
            }
            Self::Repetition(repetition, origin) => {
                let (result, count) = repeat_greedily(origin, text, position);
                (count > 0 && repetition.matches(count)).then_some(result)
            }
            Self::OneOrMoreUpTo {
                origin,
                stop_condition,
            } => repeat_until(origin, stop_condition, text, position),
            Self::UpTo(origin) => (position..=text.length())
                .find(|&current| origin.run_challenge(text, current).is_some())
                .map(|current| ChallengeResult::of_length(current - position)),
        }
    }
}

fn single_unit(
    text: TextNavigator,
    position: usize,
    accepts: impl Fn(u16) -> bool,
) -> Option<ChallengeResult> {
    if position == text.length() {
        return None;
    }
    accepts(text.char_at(position)).then(|| ChallengeResult::of_length(1))
}

fn run_all(
    challenges: &[Challenge],
    text: TextNavigator,
    position: usize,
) -> Option<ChallengeResult> {
    let mut result = ChallengeResult::default();
    for challenge in challenges {
        result.append(challenge.run_challenge(text, position + result.full_capture_length)?);
    }
    Some(result)
}

/// The longest run of `origin`, without backtracking, and how many times it matched.
fn repeat_greedily(
    origin: &Challenge,
    text: TextNavigator,
    position: usize,
) -> (ChallengeResult, usize) {
    let mut result = ChallengeResult::default();
    let mut count = 0;
    while let Some(next) = origin.run_challenge(text, position + result.full_capture_length) {
        result.append_progress(next);
        count += 1;
    }
    (result, count)
}

fn repeat_until(
    origin: &Challenge,
    stop_condition: &Challenge,
    text: TextNavigator,
    position: usize,
) -> Option<ChallengeResult> {
    let mut result = ChallengeResult::default();
    loop {
        result.append_progress(origin.run_challenge(text, position + result.full_capture_length)?);
        let current = position + result.full_capture_length;
        if stop_condition.run_challenge(text, current).is_some() {
            return Some(result);
        }
    }
}

/// The counts allowed by `〇{...}`.
#[derive(Clone, Debug)]
pub(super) struct Repetition {
    values: Vec<RangeInclusive<i64>>,
    min_inclusive: i64,
}

impl Default for Repetition {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            min_inclusive: i64::from(i32::MAX),
        }
    }
}

impl Repetition {
    pub(super) fn add_range(&mut self, values: RangeInclusive<i64>) {
        self.values.push(values);
    }

    pub(super) fn set_min_inclusive(&mut self, min: i64) {
        self.min_inclusive = min;
    }

    fn matches(&self, count: usize) -> bool {
        let count = count as i64;
        count >= self.min_inclusive || self.values.iter().any(|values| values.contains(&count))
    }
}

#[derive(Debug, Default)]
pub(super) struct ChallengeResult {
    pub full_capture_length: usize,
    pub capture: Capture,
}

impl ChallengeResult {
    fn of_length(full_capture_length: usize) -> Self {
        Self {
            full_capture_length,
            capture: Capture::default(),
        }
    }

    fn append(&mut self, next: Self) {
        self.full_capture_length += next.full_capture_length;
        self.capture.0.extend(next.capture.0);
    }

    /// For loops that only end when their challenge fails.
    fn append_progress(&mut self, next: Self) {
        assert!(next.full_capture_length > 0, "infinite loop");
        self.append(next);
    }
}

/// The named values of a match, in the order PlantUML lists them: inner names before the name holding them.
#[derive(Debug, Default)]
pub(super) struct Capture(Vec<CaptureEntry>);

#[derive(Debug)]
pub(super) struct CaptureEntry {
    pub key: String,
    /// UTF-16 units of the matched text.
    pub value: Range<usize>,
}

impl Capture {
    fn add(&mut self, key: String, value: Range<usize>) {
        self.0.push(CaptureEntry { key, value });
    }

    fn prefix_keys(&mut self, prefix: &str) {
        for entry in &mut self.0 {
            entry.key = format!("{prefix}/{}", entry.key);
        }
    }

    pub(super) fn entries(&self) -> &[CaptureEntry] {
        &self.0
    }
}

/// The text a challenge runs over, as UTF-16 units.
#[derive(Clone, Copy, Debug)]
pub(super) struct TextNavigator<'a> {
    content: &'a [u16],
}

impl<'a> TextNavigator<'a> {
    pub(super) fn build(content: &'a [u16]) -> Self {
        Self { content }
    }

    fn length(self) -> usize {
        self.content.len()
    }

    fn char_at(self, index: usize) -> u16 {
        self.content[index]
    }
}

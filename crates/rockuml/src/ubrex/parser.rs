//! Turns the string form of an expression into challenges (`AtomicParser` and the parsing halves of
//! `CompositeList`, `Repetition` and `ChallengeCharSet`).
//!
//! The input is a slice of UTF-16 units that each step cuts from the front, as `TextNavigator.jump` does.

use super::challenge::{Challenge, Repetition};
use super::char_set::{ChallengeCharSet, CharClass, CharClassRaw, ensure_lowercase};
use crate::java;

/// `CompositeList.parseAndBuild`.
pub fn parse_and_build(definition: &[u16]) -> Vec<Challenge> {
    assert!(!definition.is_empty(), "empty ubrex");
    let mut input = definition;
    parse_and_consume_now(&mut input)
}

fn parse_and_consume_now(input: &mut &[u16]) -> Vec<Challenge> {
    let mut challenges = Vec::new();
    while !input.is_empty() {
        if char_at(input, 0) == ' ' {
            jump(input, 1);
        } else {
            challenges.extend(parse(input));
        }
    }
    challenges
}

/// Surrogate halves are never syntax, so they can stand in as U+FFFD while the syntax is inspected.
fn char_at(input: &[u16], index: usize) -> char {
    char::from_u32(u32::from(input[index])).unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// A two-letter class can step past the end, which Java's navigator tolerates as an empty text.
fn jump(input: &mut &[u16], step: usize) {
    *input = input.get(step..).unwrap_or_default();
}

/// One atom; a few atoms bring siblings that must sit next to them in the enclosing list.
fn parse(input: &mut &[u16]) -> Vec<Challenge> {
    match char_at(input, 0) {
        '┇' => panic!("┇ outside an alternative"),
        '〒' => vec![manage_look_around(input)],
        '【' => vec![manage_alternative(input)],
        '〇' => manage_quantifier(input),
        '〄' => manage_up_to(input),
        '〘' => vec![manage_group(input)],
        '「' => vec![manage_character_set(input)],
        '〴' => vec![manage_class(input)],
        '〶' => manage_named(input),
        '〃' => {
            jump(input, 1);
            vec![single_char(u16::from(b'"'))]
        }
        _ => vec![manage_regular_character(input)],
    }
}

fn parse_single(input: &mut &[u16]) -> Challenge {
    let [challenge] = <[Challenge; 1]>::try_from(parse(input))
        .unwrap_or_else(|_| panic!("a quantified ubrex must be a single atom"));
    challenge
}

fn manage_look_around(input: &mut &[u16]) -> Challenge {
    jump(input, 1);
    let (behind, positive) = match char_at(input, 0) {
        '$' => {
            jump(input, 1);
            return Challenge::EndOfText;
        }
        '=' => (false, true),
        '!' => (false, false),
        '<' => match char_at(input, 1) {
            '=' => (true, true),
            '!' => (true, false),
            _ => panic!("syntax error after 〒<"),
        },
        _ => panic!("syntax error after 〒"),
    };
    jump(input, if behind { 2 } else { 1 });
    let origin = Box::new(parse_single(input));
    if behind {
        Challenge::LookBehind { origin, positive }
    } else {
        Challenge::LookAhead { origin, positive }
    }
}

fn manage_class(input: &mut &[u16]) -> Challenge {
    jump(input, 1);
    let class = CharClass::from_definition(char_at(input, 0));
    jump(input, class.definition_length());
    Challenge::CharClass(class)
}

fn manage_quantifier(input: &mut &[u16]) -> Vec<Challenge> {
    let operator = char_at(input, 1);
    jump(input, 2);
    let challenge = match operator {
        '{' => {
            let repetition = parse_repetition(input);
            Challenge::Repetition(repetition, Box::new(parse_single(input)))
        }
        'l' => return manage_quantifier_lazzy(input),
        '+' => Challenge::OneOrMore(Box::new(parse_single(input))),
        '*' => Challenge::ZeroOrMore(Box::new(parse_single(input))),
        '?' => Challenge::Optional(Box::new(parse_single(input))),
        _ => panic!("unknown quantifier 〇{operator}"),
    };
    vec![challenge]
}

/// The rest of the enclosing list is only peeked at as a stop condition, then matched again as siblings.
fn manage_quantifier_lazzy(input: &mut &[u16]) -> Vec<Challenge> {
    jump(input, 1);
    let origin = Box::new(parse_single(input));
    let remaining = parse_and_consume_now(input);
    let stop_condition = Box::new(Challenge::List(remaining.clone()));
    let mut result = vec![Challenge::OneOrMoreUpTo {
        origin,
        stop_condition,
    }];
    result.extend(remaining);
    result
}

/// `Repetition.parse`: `;`-separated counts (`3`), ranges (`2-4`) and minimums (`5+`) up to `}`.
fn parse_repetition(input: &mut &[u16]) -> Repetition {
    let mut repetition = Repetition::default();
    let mut token = String::new();
    loop {
        let ch = char_at(input, 0);
        jump(input, 1);
        match ch {
            '}' => {
                add_repetition_token(&mut repetition, &token);
                return repetition;
            }
            ';' => add_repetition_token(&mut repetition, &std::mem::take(&mut token)),
            _ => token.push(ch),
        }
    }
}

fn add_repetition_token(repetition: &mut Repetition, token: &str) {
    assert!(!token.is_empty(), "empty repetition token");
    if let Some(min) = token.strip_suffix('+') {
        repetition.set_min_inclusive(parse_int(min));
    } else if let Some((min, max)) = token.split_once('-') {
        repetition.add_range(parse_int(min)..=parse_int(max));
    } else {
        let exact = parse_int(token);
        repetition.add_range(exact..=exact);
    }
}

/// `Integer.parseInt`, which like Rust accepts a leading sign.
fn parse_int(token: &str) -> i64 {
    token
        .parse::<i32>()
        .map_or_else(|_| panic!("bad repetition count {token:?}"), i64::from)
}

/// The two forms keep their stop pattern as a sibling so that a name around them leaves it out.
fn manage_up_to(input: &mut &[u16]) -> Vec<Challenge> {
    match char_at(input, 1) {
        '>' => {
            jump(input, 2);
            let stop = parse_single(input);
            vec![Challenge::UpTo(Box::new(stop.clone())), stop]
        }
        '+' => {
            jump(input, 2);
            skip_spaces(input);
            let origin = Box::new(parse_single(input));
            skip_spaces(input);
            assert!(
                char_at(input, 0) == '-' && char_at(input, 1) == '>',
                "〄+ needs ->"
            );
            jump(input, 2);
            skip_spaces(input);
            let stop = parse_single(input);
            vec![
                Challenge::OneOrMoreUpTo {
                    origin,
                    stop_condition: Box::new(stop.clone()),
                },
                stop,
            ]
        }
        _ => panic!("unknown 〄 operator"),
    }
}

fn skip_spaces(input: &mut &[u16]) {
    while char_at(input, 0) == ' ' {
        jump(input, 1);
    }
}

fn manage_group(input: &mut &[u16]) -> Challenge {
    let end = closing_bracket(input, '〘', '〙').unwrap_or_else(|| panic!("unclosed 〘"));
    let group = parse_and_build(&input[1..end]);
    jump(input, end + 1);
    Challenge::List(group)
}

fn closing_bracket(input: &[u16], open: char, close: char) -> Option<usize> {
    let mut level = 0;
    for index in 1..input.len() {
        let ch = char_at(input, index);
        if ch == open {
            level += 1;
        } else if ch == close {
            if level == 0 {
                return Some(index);
            }
            level -= 1;
        }
    }
    None
}

/// Only nested alternatives are skipped over: a `┇` inside a group or a set still splits.
fn manage_alternative(input: &mut &[u16]) -> Challenge {
    let text = *input;
    let mut alternatives = Vec::new();
    let mut start = 1;
    let mut level = 0;
    for index in 1..text.len() {
        match char_at(text, index) {
            '【' => level += 1,
            '┇' if level == 0 => {
                alternatives.push(Challenge::List(parse_and_build(&text[start..index])));
                start = index + 1;
            }
            '】' if level == 0 => {
                alternatives.push(Challenge::List(parse_and_build(&text[start..index])));
                jump(input, index + 1);
                return Challenge::Alternative(alternatives);
            }
            '】' => level -= 1,
            _ => {}
        }
    }
    panic!("unclosed 【")
}

/// The name covers the first atom only; the siblings it brings stay outside.
fn manage_named(input: &mut &[u16]) -> Vec<Challenge> {
    assert!(char_at(input, 1) == '$', "a ubrex name must start with $");
    jump(input, 2);
    let mut name = String::new();
    loop {
        let ch = char_at(input, 0);
        jump(input, 1);
        if ch == '=' {
            assert!(!name.is_empty(), "no name before =");
            let mut parsed = parse(input);
            let first = parsed.remove(0);
            parsed.insert(
                0,
                Challenge::Named {
                    name,
                    challenges: vec![first],
                },
            );
            return parsed;
        }
        assert!(
            is_java_identifier_part(ch),
            "unsupported name character {ch:?}"
        );
        name.push(ch);
    }
}

/// Narrower than `Character.isJavaIdentifierPart` (no marks or currency signs), which no PlantUML name needs.
fn is_java_identifier_part(ch: char) -> bool {
    ch == '_' || ch == '$' || java::is_letter_or_digit(ch)
}

fn manage_character_set(input: &mut &[u16]) -> Challenge {
    let end = (0..input.len())
        .find(|&index| char_at(input, index) == '」')
        .unwrap_or_else(|| panic!("unclosed 「"));
    let set = build_char_set(&input[1..end]);
    jump(input, end + 1);
    Challenge::CharSet(set)
}

/// `ChallengeCharSet.build`.
fn build_char_set(pattern: &[u16]) -> ChallengeCharSet {
    assert!(!pattern.is_empty(), "empty char set");
    assert!(
        char_at(pattern, pattern.len() - 1) != '〜',
        "range operator 〜 must be followed by a character"
    );
    let mut result = ChallengeCharSet::default();
    let mut index = 0;
    while index < pattern.len() {
        match char_at(pattern, index) {
            '〤' => result.reverse(),
            ' ' => {}
            '〴' => {
                index += 1;
                let class = CharClassRaw::from_definition(char_at(pattern, index));
                result.add_class(class);
                index += class.definition_length() - 1;
            }
            _ if pattern.len() - index > 2 && char_at(pattern, index + 1) == '〜' => {
                result.add_range(pattern[index], pattern[index + 2]);
                index += 2;
            }
            '〃' => result.add_char(u16::from(b'"')),
            '∙' => result.add_char(u16::from(b' ')),
            _ => result.add_char(pattern[index]),
        }
        index += 1;
    }
    result
}

fn manage_regular_character(input: &mut &[u16]) -> Challenge {
    let unit = match char_at(input, 0) {
        ' ' => panic!("no space allowed"),
        '∙' => u16::from(b' '),
        _ => input[0],
    };
    jump(input, 1);
    single_char(unit)
}

/// `ChallengeSingleChar`'s constructor.
fn single_char(unit: u16) -> Challenge {
    let reserved = "┇〴「」〤〜〇〄〶〘〙【】";
    assert!(
        !char::from_u32(u32::from(unit)).is_some_and(|ch| reserved.contains(ch)),
        "reserved character {unit:#x} used as a literal"
    );
    Challenge::SingleChar(ensure_lowercase(unit))
}

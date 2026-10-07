//! What an entity holds in its body: the members of a class or object, the free lines of other elements,
//! the entries of a map or the data of a JSON element (PlantUML's `Bodier` and its implementations).

use std::sync::LazyLock;

use regex::Regex;

use super::Member;
use crate::abel::LeafType;
use crate::creole::Display;
use crate::java;
use crate::json::JsonValue;
use crate::klimt::url::Url;
use crate::pattern::java_regex;
use crate::skin::visibility_modifier::VisibilityModifier;

#[derive(Clone)]
pub(crate) enum Bodier {
    /// `BodierLikeClassOrObject`: members, read as fields or methods.
    LikeClassOrObject {
        leaf_type: LeafType,
        raw_body: Vec<String>,
    },
    /// `BodierSimple`: lines, each line's `\n` breaking it further.
    Simple { raw_body: Vec<String> },
    /// `BodierMap`: keys and values in the order first written; a value linked to an entity is `\0`.
    Map { map: Vec<(String, String)> },
    /// `BodierJSon`.
    Json { json: Option<JsonValue> },
}

impl Bodier {
    /// The body of a new leaf (`CucaDiagram.createLeaf`, `BodyFactory.createLeaf`).
    pub(crate) fn for_leaf(leaf_type: LeafType) -> Self {
        match leaf_type {
            LeafType::Map => Self::Map { map: Vec::new() },
            LeafType::Json => Self::Json { json: None },
            _ if leaf_type.is_like_class() || leaf_type == LeafType::Object => {
                Self::LikeClassOrObject {
                    leaf_type,
                    raw_body: Vec::new(),
                }
            }
            _ => Self::for_group(),
        }
    }

    /// `BodyFactory.createGroup`.
    pub(crate) fn for_group() -> Self {
        Self::Simple {
            raw_body: Vec::new(),
        }
    }

    /// Whether the line could be added: a map line needs `=>` or a link like `*->`.
    ///
    /// # Panics
    ///
    /// For JSON bodies, which take their data whole.
    pub(crate) fn add_field_or_method(&mut self, s: &str) -> bool {
        match self {
            Self::LikeClassOrObject { raw_body, .. } => raw_body.push(s.to_owned()),
            Self::Simple { raw_body } => {
                raw_body.extend(Display::with_newlines(s).lines().iter().cloned());
            }
            Self::Map { map } => {
                if let Some(x) = s.find("=>") {
                    put(map, java::trim(&s[..x]), java::trim(&s[x + 2..]));
                } else if let Some(link) = get_linked_entry(s) {
                    let pos = s.find(link).expect("the link was found in the line");
                    put(map, java::trim(&s[..pos]), "\0");
                } else {
                    return false;
                }
            }
            Self::Json { .. } => panic!("JSON bodies take their data whole"),
        }
        true
    }

    /// A class that turns out to be an object reads its members again, all as fields.
    pub(crate) fn mute_class_to_object(&mut self) {
        if let Self::LikeClassOrObject { leaf_type, .. } = self {
            *leaf_type = LeafType::Object;
        }
    }

    pub(crate) fn set_json(&mut self, value: JsonValue) {
        if let Self::Json { json } = self {
            *json = Some(value);
        }
    }

    /// The lines as written.
    pub(crate) fn get_raw_body(&self) -> &[String] {
        match self {
            Self::LikeClassOrObject { raw_body, .. } | Self::Simple { raw_body } => raw_body,
            Self::Map { .. } | Self::Json { .. } => &[],
        }
    }

    /// The methods of a class, without those of a hidden visibility and without empty lines at either end.
    pub(crate) fn get_methods_to_display(&self, hidden: &[VisibilityModifier]) -> Vec<Member> {
        let raw_body = self.get_raw_body();
        let mut result = Vec::new();
        for (i, s) in raw_body.iter().enumerate() {
            if !is_method_at(i, raw_body) || (s.is_empty() && result.is_empty()) {
                continue;
            }
            let member = Member::method(s);
            if !is_hidden(&member, hidden) {
                result.push(member);
            }
        }
        remove_final_empty_members(&mut result);
        result
    }

    /// The fields of a class, or every member of an object, without those of a hidden visibility and
    /// without empty lines at either end.
    pub(crate) fn get_fields_to_display(&self, hidden: &[VisibilityModifier]) -> Vec<Member> {
        let is_object = matches!(
            self,
            Self::LikeClassOrObject {
                leaf_type: LeafType::Object,
                ..
            }
        );
        let mut result = Vec::new();
        for s in self.get_raw_body() {
            if (!is_object && is_method(s)) || (s.is_empty() && result.is_empty()) {
                continue;
            }
            let member = Member::field(s);
            if !is_hidden(&member, hidden) {
                result.push(member);
            }
        }
        remove_final_empty_members(&mut result);
        result
    }

    /// Every member as written, each read as a field or a method, without those of a hidden visibility.
    pub(crate) fn raw_body_without_hidden(&self, hidden: &[VisibilityModifier]) -> Vec<Member> {
        self.get_raw_body()
            .iter()
            .map(|s| {
                if is_method(s) {
                    Member::method(s)
                } else {
                    Member::field(s)
                }
            })
            .filter(|member| !is_hidden(member, hidden))
            .collect()
    }

    /// The line `candidate` most likely names: the earliest match after the fewest letters, followed by the
    /// fewest letters of the same word (`BodierAbstract.getBestMatch`).
    pub(crate) fn get_best_match(&self, candidate: &str) -> Option<&str> {
        let mut best = None;
        let mut best_score = u64::MAX;
        for line in self.get_raw_body() {
            let score = match_score(line, candidate);
            if score < best_score {
                best = Some(line.as_str());
                best_score = score;
                if best_score == 0 {
                    break;
                }
            }
        }
        best
    }
}

fn put(map: &mut Vec<(String, String)>, key: &str, value: &str) {
    match map.iter_mut().find(|(existing, _)| existing == key) {
        Some(entry) => value.clone_into(&mut entry.1),
        None => map.push((key.to_owned(), value.to_owned())),
    }
}

/// The link in a map line like `key *--> Entity` (`BodierMap.getLinkedEntry`).
pub(crate) fn get_linked_entry(s: &str) -> Option<&str> {
    static LINK: LazyLock<Regex> = LazyLock::new(|| java_regex(r"(\*-+_?\>)", false));
    LINK.find(s).map(|found| found.as_str())
}

fn is_hidden(member: &Member, hidden: &[VisibilityModifier]) -> bool {
    member
        .get_visibility_modifier()
        .is_some_and(|modifier| hidden.contains(&modifier))
}

/// Whether the line is a method: marked `{method}`, or with parentheses outside its link.
fn is_method(s: &str) -> bool {
    // PlantUML compiles the link pattern without expanding its `%s`-style classes.
    static URL: LazyLock<Regex> = LazyLock::new(|| java_regex(&Url::regexp(), false));
    let purged = URL.replace_all(s, "");
    if purged.contains("{method}") {
        return true;
    }
    if purged.contains("{field}") {
        return false;
    }
    purged.contains('(') || purged.contains(')')
}

/// An empty line between two methods belongs to the methods.
fn is_method_at(i: usize, raw_body: &[String]) -> bool {
    if i > 0
        && i + 1 < raw_body.len()
        && raw_body[i].is_empty()
        && is_method(&raw_body[i - 1])
        && is_method(&raw_body[i + 1])
    {
        return true;
    }
    is_method(&raw_body[i])
}

fn remove_final_empty_members(result: &mut Vec<Member>) {
    while result
        .last()
        .is_some_and(|member| java::trim(&member.get_display(false)).is_empty())
    {
        result.pop();
    }
}

const WEIGHT_BEFORE_MATCH_STEP: u64 = 1;
const WEIGHT_AFTER_SEPARATOR: u64 = 1_000;
const WEIGHT_TRAILING_LETTERS: u64 = 1_000_000;
const WEIGHT_BEFORE_MATCH_LETTER_STEP: u64 = 1_000_000_000;

/// How far `candidate` is from naming `full_string`, compared in UTF-16 units as PlantUML does.
fn match_score(full_string: &str, candidate: &str) -> u64 {
    let full: Vec<u16> = full_string.encode_utf16().collect();
    let candidate: Vec<u16> = candidate.encode_utf16().collect();
    if candidate.len() > full.len() {
        return u64::MAX;
    }
    let char_at = |i: usize| char::from_u32(u32::from(full[i])).unwrap_or('\u{FFFD}');
    let mut score = 0;
    for i in 0..=full.len() - candidate.len() {
        if full[i..].starts_with(&candidate) {
            let mut separator_seen = false;
            for j in i + candidate.len()..full.len() {
                let ch = char_at(j);
                if !separator_seen && (ch.is_alphabetic() || ch.is_numeric() || ch == '_') {
                    score += WEIGHT_TRAILING_LETTERS;
                } else {
                    separator_seen = true;
                    score += WEIGHT_AFTER_SEPARATOR;
                }
            }
            return score;
        }
        score += if char_at(i).is_alphabetic() {
            WEIGHT_BEFORE_MATCH_LETTER_STEP
        } else {
            WEIGHT_BEFORE_MATCH_STEP
        };
    }
    u64::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(lines: &[&str]) -> Bodier {
        let mut bodier = Bodier::for_leaf(LeafType::Class);
        for line in lines {
            bodier.add_field_or_method(line);
        }
        bodier
    }

    fn displays(members: &[Member]) -> Vec<String> {
        members
            .iter()
            .map(|member| member.get_display(true))
            .collect()
    }

    #[test]
    fn parentheses_make_methods() {
        let bodier = class(&[
            "- a : int",
            "+ b()",
            "",
            "c()",
            "{method} d",
            "{field} e()",
            "",
        ]);
        assert_eq!(
            displays(&bodier.get_fields_to_display(&[])),
            ["-a : int", " ", "e()"]
        );
        assert_eq!(
            displays(&bodier.get_methods_to_display(&[])),
            ["+b()", " ", "c()", "d"]
        );
        let hidden = [VisibilityModifier::PrivateField];
        assert_eq!(displays(&bodier.get_fields_to_display(&hidden)), ["e()"]);
    }

    #[test]
    fn objects_show_every_member_as_a_field() {
        let mut bodier = class(&["a = 1", "f()"]);
        bodier.mute_class_to_object();
        assert_eq!(
            displays(&bodier.get_fields_to_display(&[])),
            ["a = 1", "f()"]
        );
    }

    #[test]
    fn maps_need_arrows() {
        let mut bodier = Bodier::for_leaf(LeafType::Map);
        assert!(bodier.add_field_or_method("UK => London"));
        assert!(bodier.add_field_or_method("USA *--> Washington"));
        assert!(!bodier.add_field_or_method("nothing"));
        let Bodier::Map { map } = bodier else {
            unreachable!()
        };
        assert_eq!(
            map,
            [
                ("UK".to_owned(), "London".to_owned()),
                ("USA".to_owned(), "\0".to_owned())
            ]
        );
    }

    #[test]
    fn the_best_match_names_the_member_at_its_start() {
        let bodier = class(&["priority : int", "start()", "run()"]);
        assert_eq!(bodier.get_best_match("start"), Some("start()"));
        assert_eq!(bodier.get_best_match("priority"), Some("priority : int"));
        assert_eq!(bodier.get_best_match("zzz"), None);
    }
}

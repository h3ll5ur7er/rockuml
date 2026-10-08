//! `#highlight "a" / "b" <<style>>`: a path to the entries of a JSON or YAML document drawn highlighted
//! (PlantUML's `Highlighted`).

use std::sync::LazyLock;

use regex::Regex;

use crate::stereo::Stereotype;

const HIGHLIGHTED: &str = "#highlight ";

#[derive(Clone, Debug)]
pub(super) struct Highlighted {
    paths: Vec<String>,
    stereotype: Option<Stereotype>,
}

impl Highlighted {
    pub(super) fn matches_definition(line: &str) -> bool {
        line.starts_with(HIGHLIGHTED)
    }

    /// The highlight a definition line asks for; `None` for one PlantUML cannot read either.
    pub(super) fn build(line: &str) -> Option<Self> {
        static PATTERN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^([^<>]+)(<<.*>>)?$").unwrap());
        let line = crate::java::trim(line.strip_prefix(HIGHLIGHTED)?);
        let captures = PATTERN.captures(line)?;
        let paths = crate::java::trim(&captures[1])
            .split('/')
            .map(|path| remove_starting_and_ending_quote(crate::java::trim(path)).to_owned())
            .collect();
        Some(Self {
            paths,
            stereotype: captures
                .get(2)
                .map(|stereotype| Stereotype::new(stereotype.as_str())),
        })
    }

    /// The highlight seen from the child `key`, if it reaches below it (`upOneLevel`).
    pub(super) fn up_one_level(&self, key: &str) -> Option<Self> {
        if self.paths.len() <= 1 {
            return None;
        }
        let first = &self.paths[0];
        if first == "**" {
            return Some(self.clone());
        }
        (first == "*" || first == key).then(|| Self {
            paths: self.paths[1..].to_vec(),
            stereotype: self.stereotype.clone(),
        })
    }

    pub(super) fn is_key_highlight(&self, key: &str) -> bool {
        match self.paths.as_slice() {
            [any, last] if any == "**" => last == key,
            [only] => only == key,
            _ => false,
        }
    }

    pub(super) fn stereotype(&self) -> Option<&Stereotype> {
        self.stereotype.as_ref()
    }
}

/// `StringUtils.eventuallyRemoveStartingAndEndingDoubleQuote(s, "\"")`.
fn remove_starting_and_ending_quote(text: &str) -> &str {
    if text.len() > 1
        && let Some(inner) = text
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
    {
        return inner;
    }
    text
}

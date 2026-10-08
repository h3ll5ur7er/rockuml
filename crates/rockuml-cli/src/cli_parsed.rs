//! The command line split into flags and their values, and the remaining arguments (`CliParsed`).

use std::collections::BTreeMap;

use crate::cli_flag::{Arity, CliFlag};

#[derive(Debug, Default)]
pub(crate) struct CliParsed {
    /// The values of each flag given, in order; `None` for flags without a value.
    lists: BTreeMap<CliFlag, Vec<Option<String>>>,
    /// The `KEY` or `KEY=VALUE` of `-D`, `-I`, `-P` and `-S`; a repeated key keeps its first position.
    maps: BTreeMap<CliFlag, Vec<(String, Option<String>)>>,
    args: Vec<String>,
}

/// An argument PlantUML cannot parse (`CliParsingException`).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CliParsingException(pub String);

impl CliParsed {
    pub(crate) fn parse(
        arguments: impl IntoIterator<Item = String>,
    ) -> Result<Self, CliParsingException> {
        let mut result = Self::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            let Some(flag) = CliFlag::of(&argument) else {
                result.args.push(argument);
                continue;
            };
            match flag.arity() {
                Arity::UnaryBoolean | Arity::UnaryImmediateAction => result.put_value(flag, None),
                Arity::BinaryNextArgumentValue => {
                    let value = arguments.next().ok_or_else(|| missing_value(flag))?;
                    result.put_value(flag, Some(value));
                }
                Arity::UnaryInlineKeyOrKeyValue => {
                    let inline = &argument[flag.flag().len()..];
                    let key_value = if inline.is_empty() {
                        arguments.next().ok_or_else(|| missing_value(flag))?
                    } else {
                        inline.to_owned()
                    };
                    let (key, value) = match key_value.split_once('=') {
                        Some((key, value)) => (key.to_owned(), Some(value.to_owned())),
                        None => (key_value, None),
                    };
                    result.put_map_value(flag, key, value);
                }
                Arity::UnaryOptionalColon => {
                    let values = result.lists.entry(flag).or_default();
                    if let Some(colon) = argument.find(':') {
                        values.extend(
                            argument[colon + 1..]
                                .split(':')
                                .map(|part| Some(part.to_owned())),
                        );
                    }
                }
            }
        }
        Ok(result)
    }

    fn put_value(&mut self, flag: CliFlag, value: Option<String>) {
        self.lists.entry(flag).or_default().push(value);
    }

    fn put_map_value(&mut self, flag: CliFlag, key: String, value: Option<String>) {
        let entries = self.maps.entry(flag).or_default();
        match entries.iter_mut().find(|(existing, _)| *existing == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key, value)),
        }
    }

    /// The flags given, in PlantUML's order.
    pub(crate) fn flags(&self) -> impl Iterator<Item = CliFlag> + '_ {
        let mut flags: Vec<CliFlag> = self.lists.keys().chain(self.maps.keys()).copied().collect();
        flags.sort();
        flags.dedup();
        flags.into_iter()
    }

    pub(crate) fn is_true(&self, flag: CliFlag) -> bool {
        self.maps.contains_key(&flag) || self.lists.contains_key(&flag)
    }

    /// The flag's first value, or its default.
    pub(crate) fn get_string(&self, flag: CliFlag) -> Option<&str> {
        match self.lists.get(&flag).and_then(|values| values.first()) {
            Some(value) => value.as_deref(),
            None => flag.default_value(),
        }
    }

    /// Every value of the flag, in order.
    pub(crate) fn get_list(&self, flag: CliFlag) -> impl Iterator<Item = &str> {
        self.lists
            .get(&flag)
            .into_iter()
            .flatten()
            .filter_map(Option::as_deref)
    }

    /// How often the flag was given.
    pub(crate) fn count(&self, flag: CliFlag) -> usize {
        self.lists.get(&flag).map_or(0, Vec::len)
    }

    /// The keys of a short flag (`-DKEY=VALUE`) followed by those of its long form (`--define KEY=VALUE`); a
    /// key given again replaces the value but keeps its place. A long form with several `=` is ignored, as
    /// PlantUML splits it into too many parts.
    pub(crate) fn get_map(
        &self,
        map_flag: CliFlag,
        list_flag: CliFlag,
    ) -> Vec<(String, Option<String>)> {
        let mut result = self.maps.get(&map_flag).cloned().unwrap_or_default();
        for key_value in self.get_list(list_flag) {
            let parts: Vec<&str> = java_split(key_value, '=');
            let (key, value) = match parts.as_slice() {
                [key] => (*key, None),
                [key, value] => (*key, Some((*value).to_owned())),
                _ => continue,
            };
            match result.iter_mut().find(|(existing, _)| existing == key) {
                Some(entry) => entry.1 = value,
                None => result.push((key.to_owned(), value)),
            }
        }
        result
    }

    pub(crate) fn remaining_args(&self) -> &[String] {
        &self.args
    }
}

/// `String.split`: trailing empty parts are dropped.
fn java_split(text: &str, separator: char) -> Vec<&str> {
    let mut parts: Vec<&str> = text.split(separator).collect();
    while parts.len() > 1 && parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

fn missing_value(flag: CliFlag) -> CliParsingException {
    CliParsingException(format!("{}: missing value", flag.flag()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(words: &str) -> CliParsed {
        CliParsed::parse(words.split_whitespace().map(str::to_owned)).unwrap()
    }

    #[test]
    fn flags_take_their_values_and_the_rest_are_files() {
        let parsed = parse("-tsvg -o out a.puml --exclude x b.puml");
        assert!(parsed.is_true(CliFlag::TSvg));
        assert_eq!(parsed.get_string(CliFlag::OutputDir), Some("out"));
        assert_eq!(parsed.get_list(CliFlag::Exclude).collect::<Vec<_>>(), ["x"]);
        assert_eq!(parsed.remaining_args(), ["a.puml", "b.puml"]);
    }

    #[test]
    fn defines_come_inline_or_as_the_next_argument() {
        let parsed = parse("-DA=1 -D B=2 -DFLAG --define C=3 --define A=4 --define X=1=2");
        assert_eq!(
            parsed.get_map(CliFlag::Define, CliFlag::DefineLong),
            [
                ("A".to_owned(), Some("4".to_owned())),
                ("B".to_owned(), Some("2".to_owned())),
                ("FLAG".to_owned(), None),
                ("C".to_owned(), Some("3".to_owned())),
            ]
        );
    }

    #[test]
    fn optional_colon_values_split_at_colons() {
        let parsed = parse("-stdrpt:1:2");
        assert_eq!(
            parsed.get_list(CliFlag::Stdrpt).collect::<Vec<_>>(),
            ["1", "2"]
        );
        assert!(parse("-stdrpt").is_true(CliFlag::Stdrpt));
    }

    #[test]
    fn defaults_stand_in_for_flags_not_given() {
        assert_eq!(parse("").get_string(CliFlag::Charset), Some("UTF-8"));
        assert_eq!(parse("").get_string(CliFlag::OutputDir), None);
    }

    #[test]
    fn values_cannot_be_missing() {
        let error = CliParsed::parse(["-D".to_owned()]).unwrap_err();
        assert_eq!(error, CliParsingException("-D: missing value".to_owned()));
        assert!(CliParsed::parse(["-o".to_owned()]).is_err());
    }
}

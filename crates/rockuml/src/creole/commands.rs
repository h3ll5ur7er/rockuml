//! Inline creole markup: commands found at some position of a line that restyle the text they enclose.

use std::sync::LazyLock;

use regex::{Captures, Regex};

use super::parser::StripeBuilder;
use crate::color::HColor;
use crate::klimt::font::FontConfiguration;
use crate::pattern::plantuml_regex;

pub trait CreoleCommand: Send + Sync {
    /// The two characters a line must continue with for the command to be tried.
    fn starters(&self) -> &[&'static str];

    /// Whether the command applies at the start of `rest`.
    fn matches(&self, rest: &str) -> bool;

    /// Applies the command at the start of `rest`; returns how many bytes it consumed.
    fn execute(&self, rest: &str, stripe: &mut StripeBuilder) -> usize;
}

/// The commands of PlantUML's `CommandCreoleBuilder.FULL`, in its order: the first that matches wins.
pub fn full_creole_commands() -> &'static [Box<dyn CreoleCommand>] {
    static COMMANDS: LazyLock<Vec<Box<dyn CreoleCommand>>> = LazyLock::new(|| {
        vec![
            RegexCommand::boxed(
                "<s",
                r"^(\<size[\s:]+(\d+)[%s]*\>(.*?)\</size\>)",
                2,
                change_size,
            ),
            RegexCommand::boxed("<s", r"^(\<size[\s:]+(\d+)[%s]*\>(.*)$)", 2, change_size),
            RegexCommand::boxed(
                "<c",
                &format!(r"^({COLOR}(.*?)\</color\>)"),
                2,
                change_color,
            ),
            RegexCommand::boxed("<c", &format!("^({COLOR}(.*)$)"), 2, change_color),
            RegexCommand::boxed(
                "<f",
                &format!(r"^({FONT}(.*?)\</font\>)"),
                1,
                change_color_and_size,
            ),
            RegexCommand::boxed("<f", &format!("^({FONT}(.*))$"), 1, change_color_and_size),
            RegexCommand::boxed(
                "<f",
                &format!(r"^({FAMILY}(.*?)\</font\>)"),
                1,
                change_family,
            ),
            RegexCommand::boxed("<f", &format!("^({FAMILY}(.*)$)"), 1, change_family),
            RegexCommand::boxed("\"\"", r#"^(""(.*?)"")"#, 1, monospaced),
        ]
    });
    &COMMANDS
}

const COLOR: &str = r"\<color[\s:]+(#[0-9a-fA-F]{1,6}|#?\w+)[%s]*\>";
const FONT: &str = r"\<font(?:[%s]+size[%s]*=[%s]*[%g]?(\d+)[%g]?|[%s]+color[%s]*=[%s]*[%g]?(#[0-9a-fA-F]{6}|\w+)[%g]?)+[%s]*\>";
const FAMILY: &str = r"\<font[\s:]+([^>]+)/?\>";

/// A command described by a regex whose group 1 is everything the command consumes.
struct RegexCommand {
    starters: [&'static str; 1],
    regex: Regex,
    /// PlantUML ignores a match whose group of this number is empty.
    group_required: usize,
    apply: fn(&Captures, &mut StripeBuilder),
}

impl RegexCommand {
    fn boxed(
        starter: &'static str,
        pattern: &str,
        group_required: usize,
        apply: fn(&Captures, &mut StripeBuilder),
    ) -> Box<dyn CreoleCommand> {
        Box::new(Self {
            starters: [starter],
            regex: plantuml_regex(pattern),
            group_required,
            apply,
        })
    }
}

impl CreoleCommand for RegexCommand {
    fn starters(&self) -> &[&'static str] {
        &self.starters
    }

    fn matches(&self, rest: &str) -> bool {
        self.regex
            .captures(rest)
            .and_then(|captures| captures.get(self.group_required))
            .is_some_and(|group| !group.is_empty())
    }

    fn execute(&self, rest: &str, stripe: &mut StripeBuilder) -> usize {
        let captures = self
            .regex
            .captures(rest)
            .expect("only executed where it matches");
        (self.apply)(&captures, stripe);
        captures[1].len()
    }
}

fn group<'a>(captures: &'a Captures, number: usize) -> Option<&'a str> {
    captures.get(number).map(|group| group.as_str())
}

/// The text the command encloses: its last group.
fn enclosed<'a>(captures: &'a Captures) -> &'a str {
    group(captures, captures.len() - 1).unwrap_or_default()
}

fn change_size(captures: &Captures, stripe: &mut StripeBuilder) {
    let size = font_size(&captures[2]);
    stripe.with_font(|font| font.with_size(size), enclosed(captures));
}

/// An unknown colour leaves the text's colour as it is.
fn change_color(captures: &Captures, stripe: &mut StripeBuilder) {
    let color = parse_color(&captures[2]);
    stripe.with_font(
        |font| color.map_or_else(|| font.clone(), |color| font.with_color(color)),
        enclosed(captures),
    );
}

fn change_color_and_size(captures: &Captures, stripe: &mut StripeBuilder) {
    let size = group(captures, 2).map(font_size);
    let color = group(captures, 3).and_then(parse_color);
    stripe.with_font(
        |font| {
            let font = size.map_or_else(|| font.clone(), |size| font.with_size(size));
            color.map_or(font.clone(), |color| font.with_color(color))
        },
        enclosed(captures),
    );
}

fn change_family(captures: &Captures, stripe: &mut StripeBuilder) {
    let family = captures[2].to_owned();
    stripe.with_font(|font| font.with_family(&family), enclosed(captures));
}

fn monospaced(captures: &Captures, stripe: &mut StripeBuilder) {
    stripe.with_font(|font| font.with_family("monospaced"), enclosed(captures));
}

/// The patterns let only digits through, and any run of digits parses as a float.
fn font_size(digits: &str) -> f32 {
    digits.parse().unwrap_or_default()
}

fn parse_color(name: &str) -> Option<HColor> {
    HColor::parse(name).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matching(rest: &str) -> Vec<usize> {
        full_creole_commands()
            .iter()
            .enumerate()
            .filter(|(_, command)| command.matches(rest))
            .map(|(index, _)| index)
            .collect()
    }

    #[test]
    fn closed_and_open_ended_forms_both_match() {
        assert_eq!(matching("<size:9>a</size>b"), [0, 1]);
        assert_eq!(matching("<size:9>a"), [1]);
        assert_eq!(matching("<color:red>a</color>"), [2, 3]);
        assert_eq!(matching("<font color=red size=3>a</font>"), [4, 5, 6, 7]);
        assert_eq!(matching("\"\"a\"\""), [8]);
    }

    #[test]
    fn empty_enclosed_text_still_counts_where_plantuml_checks_the_whole_match() {
        assert_eq!(matching("\"\"\"\""), [8]);
        assert_eq!(matching("<size:>a"), Vec::<usize>::new());
    }
}

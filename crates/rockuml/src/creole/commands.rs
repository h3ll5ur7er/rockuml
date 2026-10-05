//! Inline creole markup: commands found at some position of a line that restyle the text they enclose.

use std::sync::LazyLock;

use regex::{Captures, Regex};

use super::CreoleMode;
use super::parser::StripeBuilder;
use crate::color::HColor;
use crate::klimt::font::{FontPosition, FontStyle};
use crate::klimt::url::Url;
use crate::pattern::{java_regex, plantuml_regex};
use crate::ubrex::{UMatcher, UnicodeBracketedExpression};

pub trait CreoleCommand: Send + Sync {
    /// The two characters a line must continue with for the command to be tried.
    fn starters(&self) -> &[&'static str];

    /// Whether the command applies at the start of `rest`.
    fn matches(&self, rest: &str) -> bool;

    /// Applies the command at the start of `rest`; returns how many bytes it consumed.
    fn execute(&self, rest: &str, stripe: &mut StripeBuilder) -> usize;
}

/// The inline commands of a creole mode, in `CommandCreoleBuilder`'s order: the first that matches wins.
/// Only full creole reads `__underline__`.
pub fn creole_commands(mode: CreoleMode) -> &'static [Box<dyn CreoleCommand>] {
    static FULL: LazyLock<Vec<Box<dyn CreoleCommand>>> = LazyLock::new(|| build_commands(true));
    static OTHER: LazyLock<Vec<Box<dyn CreoleCommand>>> = LazyLock::new(|| build_commands(false));
    if mode == CreoleMode::Full {
        &FULL
    } else {
        &OTHER
    }
}

fn build_commands(creole_underline: bool) -> Vec<Box<dyn CreoleCommand>> {
    let mut commands: Vec<Box<dyn CreoleCommand>> = Vec::new();
    for style in [
        FontStyle::Bold,
        FontStyle::Italic,
        FontStyle::Plain,
        FontStyle::Underline,
        FontStyle::Strike,
        FontStyle::Wave,
        FontStyle::Backcolor,
    ] {
        let with_creole = style != FontStyle::Underline || creole_underline;
        commands.extend(StyleCommand::all_for(style, with_creole));
    }
    commands.extend(vec![
        RegexCommand::boxed(
            &["<s"],
            r"^(\<size[\s:]+(\d+)[%s]*\>(.*?)\</size\>)",
            2,
            change_size,
        ),
        RegexCommand::boxed(&["<s"], r"^(\<size[\s:]+(\d+)[%s]*\>(.*)$)", 2, change_size),
        RegexCommand::boxed(
            &["<c"],
            &format!(r"^({COLOR}(.*?)\</color\>)"),
            2,
            change_color,
        ),
        RegexCommand::boxed(&["<c"], &format!("^({COLOR}(.*)$)"), 2, change_color),
        RegexCommand::boxed(
            &["<f"],
            &format!(r"^({FONT}(.*?)\</font\>)"),
            1,
            change_color_and_size,
        ),
        RegexCommand::boxed(
            &["<f"],
            &format!("^({FONT}(.*))$"),
            1,
            change_color_and_size,
        ),
        PositionCommand::boxed(FontPosition::Exposant, "sup"),
        PositionCommand::boxed(FontPosition::Indice, "sub"),
        RegexCommand::boxed(
            &["<#", "<&"],
            &format!(r"^(\<(#\w+)?&([-\w]+){SCALE_OR_COLOR}\>)"),
            1,
            open_icon,
        ),
        RegexCommand::boxed(
            &["<f"],
            &format!(r"^({FAMILY}(.*?)\</font\>)"),
            1,
            change_family,
        ),
        RegexCommand::boxed(&["<f"], &format!("^({FAMILY}(.*)$)"), 1, change_family),
        RegexCommand::boxed(&["\"\""], r#"^(""(.*?)"")"#, 1, monospaced),
        Box::new(LinkCommand),
    ]);
    commands
}

const COLOR: &str = r"\<color[\s:]+(#[0-9a-fA-F]{1,6}|#?\w+)[%s]*\>";
const FONT: &str = r"\<font(?:[%s]+size[%s]*=[%s]*[%g]?(\d+)[%g]?|[%s]+color[%s]*=[%s]*[%g]?(#[0-9a-fA-F]{6}|\w+)[%g]?)+[%s]*\>";
const FAMILY: &str = r"\<font[\s:]+([^>]+)/?\>";
const SCALE_OR_COLOR: &str =
    r"([\{,]?(?:(?:scale=|\*)[0-9.]+)?(?:,?color[= :](?:#[0-9a-fA-F]{1,8}|\w+))?\}?)?";

/// Turns on a font style for the text it encloses: `**bold**` (creole), `<b>bold</b>` (legacy) or `<b>`
/// up to the end of the line.
struct StyleCommand {
    starters: Vec<&'static str>,
    pattern: UnicodeBracketedExpression,
    style: FontStyle,
    /// Whether `<u:red>`-like markup may give the style a colour.
    takes_color: bool,
}

impl StyleCommand {
    /// PlantUML's `createCreole`, `createLegacy` and `createLegacyEol`, in that order.
    fn all_for(style: FontStyle, with_creole: bool) -> Vec<Box<dyn CreoleCommand>> {
        let mut commands: Vec<Box<dyn CreoleCommand>> = Vec::new();
        if let Some(creole) = creole_markup(style).filter(|_| with_creole) {
            commands.push(Box::new(Self {
                starters: vec![creole],
                pattern: UnicodeBracketedExpression::build(&format!(
                    "{creole}〶$V=〄+〴.->〘{creole}〙"
                )),
                style,
                takes_color: false,
            }));
        }
        let (activation, deactivation) = legacy_markup(style);
        let takes_color = matches!(
            style,
            FontStyle::Underline | FontStyle::Wave | FontStyle::Backcolor | FontStyle::Strike
        );
        for pattern in [
            format!("{activation}〶$V=〄>〘{deactivation}〙"),
            format!("{activation}〶$V=〇+〴."),
        ] {
            commands.push(Box::new(Self {
                starters: legacy_starters(style).to_vec(),
                pattern: UnicodeBracketedExpression::build(&pattern),
                style,
                takes_color,
            }));
        }
        commands
    }
}

impl CreoleCommand for StyleCommand {
    fn starters(&self) -> &[&'static str] {
        &self.starters
    }

    fn matches(&self, rest: &str) -> bool {
        captures_value(&self.pattern, rest)
    }

    fn execute(&self, rest: &str, stripe: &mut StripeBuilder) -> usize {
        let matcher = matched(&self.pattern, rest);
        let value = matcher.find_values_by_key("V")[0];
        let color = self
            .takes_color
            .then(|| matcher.find_values_by_key("XC").first().copied())
            .flatten()
            .map(HColor::parse_or_white);
        stripe.with_font(
            |font| {
                let styled = font.with_style(self.style);
                color.map_or(styled.clone(), |color| styled.with_extended_color(color))
            },
            value,
        );
        matcher.accepted_match().len()
    }
}

/// Whether `pattern` matches at the start of `rest` with a non-empty `V`.
fn captures_value(pattern: &UnicodeBracketedExpression, rest: &str) -> bool {
    pattern.match_at(rest, 0).is_some_and(|matcher| {
        matcher
            .find_values_by_key("V")
            .first()
            .is_some_and(|value| !value.is_empty())
    })
}

fn matched<'a>(pattern: &UnicodeBracketedExpression, rest: &'a str) -> UMatcher<'a> {
    pattern
        .match_at(rest, 0)
        .expect("a creole command executes only where it matches")
}

fn creole_markup(style: FontStyle) -> Option<&'static str> {
    match style {
        FontStyle::Italic => Some("//"),
        FontStyle::Bold => Some("**"),
        FontStyle::Underline => Some("__"),
        FontStyle::Wave => Some("~~"),
        FontStyle::Strike => Some("--"),
        FontStyle::Plain | FontStyle::Backcolor => None,
    }
}

fn legacy_starters(style: FontStyle) -> &'static [&'static str] {
    match style {
        FontStyle::Plain => &["<p", "<P"],
        FontStyle::Italic => &["<i", "<I"],
        FontStyle::Bold | FontStyle::Backcolor => &["<b", "<B"],
        FontStyle::Underline => &["<u", "<U"],
        FontStyle::Strike => &["<s", "<S", "<d", "<D"],
        FontStyle::Wave => &["<w"],
    }
}

/// The `UBrex` patterns of the HTML-like tags that open and close a style.
fn legacy_markup(style: FontStyle) -> (String, &'static str) {
    const COLOR: &str = "〇?〘:〶$XC=【#〇{6}「0〜9a〜fA〜F」┇〇+〴w】〙>";
    match style {
        FontStyle::Plain => ("<「pP」「lL」「aA」「iI」「nN」>".to_owned(), "</「pP」「lL」「aA」「iI」「nN」>"),
        FontStyle::Italic => ("<「iI」>".to_owned(), "</「iI」>"),
        FontStyle::Bold => ("<「bB」>".to_owned(), "</「bB」>"),
        FontStyle::Underline => (format!("<「uU」{COLOR}"), "</「uU」>"),
        FontStyle::Wave => (format!("<「wW」{COLOR}"), "</「wW」>"),
        FontStyle::Backcolor => (
            "<「bB」「aA」「cC」「kK」〇?〘:〶$XC=〘【#〇{6}「0〜9a〜fA〜F」┇〇+〴w 】 〇?〘「-\\|/」【〇{6}「0〜9a〜fA〜F」┇〇+〴w】 〙〙 〙>"
                .to_owned(),
            "</「bB」「aA」「cC」「kK」>",
        ),
        FontStyle::Strike => (
            format!("<【strike┇STRIKE┇s┇S┇del┇DEL】{COLOR}"),
            "</【strike┇STRIKE┇s┇S┇del┇DEL】>",
        ),
    }
}

/// `<sup>raised</sup>` and `<sub>lowered</sub>`.
struct PositionCommand {
    pattern: UnicodeBracketedExpression,
    position: FontPosition,
}

impl PositionCommand {
    fn boxed(position: FontPosition, tag: &str) -> Box<dyn CreoleCommand> {
        Box::new(Self {
            pattern: UnicodeBracketedExpression::build(&format!("<{tag}>〶$V=〄>〘</{tag}>〙")),
            position,
        })
    }
}

impl CreoleCommand for PositionCommand {
    fn starters(&self) -> &[&'static str] {
        &["<s"]
    }

    fn matches(&self, rest: &str) -> bool {
        captures_value(&self.pattern, rest)
    }

    fn execute(&self, rest: &str, stripe: &mut StripeBuilder) -> usize {
        let matcher = matched(&self.pattern, rest);
        let value = matcher.find_values_by_key("V")[0];
        stripe.with_font(|font| font.with_position(self.position), value);
        matcher.accepted_match().len()
    }
}

/// A command described by a regex whose group 1 is everything the command consumes.
struct RegexCommand {
    starters: &'static [&'static str],
    regex: Regex,
    /// PlantUML ignores a match whose group of this number is empty.
    group_required: usize,
    apply: fn(&Captures, &mut StripeBuilder),
}

impl RegexCommand {
    fn boxed(
        starters: &'static [&'static str],
        pattern: &str,
        group_required: usize,
        apply: fn(&Captures, &mut StripeBuilder),
    ) -> Box<dyn CreoleCommand> {
        Box::new(Self {
            starters,
            regex: plantuml_regex(pattern),
            group_required,
            apply,
        })
    }
}

impl CreoleCommand for RegexCommand {
    fn starters(&self) -> &[&'static str] {
        self.starters
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

/// `<&name>`, `<#color&name>`, `<&name*2>` or `<&name{scale=2,color=red}>`.
fn open_icon(captures: &Captures, stripe: &mut StripeBuilder) {
    let scale_or_color = group(captures, 4);
    let color = group(captures, 2)
        .or_else(|| get_color(scale_or_color))
        .map(HColor::parse_or_white);
    stripe.add_open_icon(&captures[3], get_scale(scale_or_color, 1.0), color);
}

fn get_scale(scale_or_color: Option<&str>, default: f64) -> f64 {
    static SCALE: LazyLock<Regex> = LazyLock::new(|| java_regex(r"(?:scale=|\*)([0-9.]+)", false));
    scale_or_color
        .and_then(|text| SCALE.captures(text))
        .and_then(|captures| captures[1].parse().ok())
        .unwrap_or(default)
}

fn get_color(scale_or_color: Option<&str>) -> Option<&str> {
    static COLOR: LazyLock<Regex> =
        LazyLock::new(|| java_regex(r"color[= :](#[0-9a-fA-F]{1,6}|\w+)", false));
    COLOR
        .captures(scale_or_color?)
        .and_then(|captures| captures.get(1))
        .map(|color| color.as_str())
}

/// The patterns let only digits through, and any run of digits parses as a float.
fn font_size(digits: &str) -> f32 {
    digits.parse().unwrap_or_default()
}

fn parse_color(name: &str) -> Option<HColor> {
    HColor::parse(name).ok().flatten()
}

/// `[[url label]]` and the other link forms.
struct LinkCommand;

impl CreoleCommand for LinkCommand {
    fn starters(&self) -> &[&'static str] {
        &["[["]
    }

    fn matches(&self, rest: &str) -> bool {
        Url::markup_length(rest).is_some()
    }

    fn execute(&self, rest: &str, stripe: &mut StripeBuilder) -> usize {
        let length =
            Url::markup_length(rest).expect("a creole command executes only where it matches");
        if let Some(url) = Url::parse(&rest[..length]) {
            stripe.add_url(url);
        }
        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many commands apply at the start of `rest`.
    fn applying(rest: &str) -> usize {
        creole_commands(CreoleMode::Full)
            .iter()
            .filter(|command| command.matches(rest))
            .count()
    }

    #[test]
    fn closed_and_open_ended_forms_both_match() {
        assert_eq!(applying("<size:9>a</size>b"), 2);
        assert_eq!(applying("<size:9>a"), 1);
        assert_eq!(applying("<b>a</b>"), 2);
        assert_eq!(applying("<b>a"), 1);
        assert_eq!(applying("<font color=red size=3>a</font>"), 4);
    }

    #[test]
    fn plantuml_ignores_commands_around_nothing_except_monospace() {
        assert_eq!(applying("****"), 0);
        assert_eq!(applying("<sup></sup>"), 0);
        assert_eq!(applying("<size:>a"), 0);
        assert_eq!(applying("\"\"\"\""), 1);
    }
}

use std::sync::LazyLock;

use regex::Regex;

use super::atom_text::AtomText;
use super::atoms::{Bullet, HorizontalLine};
use super::{Atom, Sheet, Stripe, char_hidder};
use crate::java;
use crate::jaws::BLOCK_E1_NEWLINE;
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::{FontConfiguration, FontStyle};
use crate::pattern::{java_regex, plantuml_regex};

/// Turns the lines of a label into a [`Sheet`] (PlantUML's legacy `CreoleParser`).
pub struct CreoleParser {
    font: FontConfiguration,
    horizontal_alignment: HorizontalAlignment,
}

impl CreoleParser {
    pub fn new(font: FontConfiguration, horizontal_alignment: HorizontalAlignment) -> Self {
        Self {
            font,
            horizontal_alignment,
        }
    }

    pub fn create_sheet(&self, lines: &[impl AsRef<str>]) -> Sheet {
        let mut list_numbers = ListNumbers::default();
        let mut stripes: Vec<Stripe> = Vec::new();
        for line in lines {
            let alignment = stripes
                .last()
                .map_or(self.horizontal_alignment, |stripe| stripe.cell_alignment);
            let line = manage_guillemet(line.as_ref());
            stripes.extend(self.create_stripes(&line, alignment, &mut list_numbers));
        }
        Sheet { stripes }
    }

    fn create_stripes(
        &self,
        line: &str,
        alignment: HorizontalAlignment,
        list_numbers: &mut ListNumbers,
    ) -> Vec<Stripe> {
        let (text, style) = StripeStyle::parse(line);
        java::split(&text, &BLOCK_E1_NEWLINE.to_string())
            .iter()
            .map(|single_line| {
                let header = self.header(style, list_numbers);
                let mut stripe = StripeBuilder::new(self.font.clone(), style, alignment, header);
                stripe.analyze_and_add(single_line);
                stripe.build()
            })
            .collect()
    }

    /// What a list item starts with.
    fn header(&self, style: StripeStyle, list_numbers: &mut ListNumbers) -> Option<Box<dyn Atom>> {
        match style.kind {
            StripeStyleType::ListWithoutNumber => {
                Some(Box::new(Bullet::new(self.font.clone(), style.order)))
            }
            StripeStyleType::ListWithNumber => {
                let number = list_numbers.next(style.order);
                Some(Box::new(AtomText::list_number(
                    self.font.clone(),
                    style.order,
                    number,
                )))
            }
            _ => None,
        }
    }
}

/// Counts `#` list items per nesting level; going back up a level restarts the deeper counts.
#[derive(Default)]
struct ListNumbers {
    per_level: Vec<usize>,
}

impl ListNumbers {
    fn next(&mut self, order: usize) -> usize {
        self.per_level.resize(order + 1, 0);
        let number = self.per_level[order];
        self.per_level[order] += 1;
        number
    }
}

/// `<<stereotype>>` reads as `«stereotype»`.
fn manage_guillemet(line: &str) -> String {
    static GUILLEMET: LazyLock<Regex> =
        LazyLock::new(|| java_regex(r"\<\<\s?((?:\<&\w+\>|[^<>])+?)\s?\>\>", false));
    if !line.contains('<') {
        return line.to_owned();
    }
    GUILLEMET.replace_all(line, "\u{AB}${1}\u{BB}").into_owned()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StripeStyleType {
    Normal,
    Heading,
    ListWithoutNumber,
    ListWithNumber,
    /// Drawn with `-`, `=` or `.`.
    HorizontalLine(char),
}

/// How a whole line is styled, from markup at its start (PlantUML's `CreoleStripeSimpleParser`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StripeStyle {
    kind: StripeStyleType,
    order: usize,
}

struct LinePattern {
    regex: LazyLock<Regex>,
    /// Hash lists are matched with escaped characters hidden, so that `~#` is no list.
    on_hidden_text: bool,
    style: fn(&regex::Captures) -> (String, StripeStyle),
}

impl StripeStyle {
    const NORMAL: Self = Self::of(StripeStyleType::Normal, 0);

    const fn of(kind: StripeStyleType, order: usize) -> Self {
        Self { kind, order }
    }

    /// The line's text without the markup, and its style.
    fn parse(line: &str) -> (String, Self) {
        static PATTERNS: [LinePattern; 8] = [
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex("^--([^-]*)--$")),
                on_hidden_text: false,
                style: |m| line_with_title(m, '-'),
            },
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex("^==([^=]*)==$")),
                on_hidden_text: false,
                style: |m| line_with_title(m, '='),
            },
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex("^===*==$")),
                on_hidden_text: false,
                style: |_| {
                    (
                        String::new(),
                        StripeStyle::of(StripeStyleType::HorizontalLine('='), 0),
                    )
                },
            },
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex(r"^\.\.([^\.]*)\.\.$")),
                on_hidden_text: false,
                style: |m| line_with_title(m, '.'),
            },
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex(r"^(\*+)([^*]+(?:[^*]|\*\*[^*]+\*\*)*)$")),
                on_hidden_text: false,
                style: |m| item(m, StripeStyleType::ListWithoutNumber),
            },
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex(r"^(\*+)([%s].+)$")),
                on_hidden_text: false,
                style: |m| item(m, StripeStyleType::ListWithoutNumber),
            },
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex("^(#+)(.+)$")),
                on_hidden_text: true,
                style: |m| item(m, StripeStyleType::ListWithNumber),
            },
            LinePattern {
                regex: LazyLock::new(|| plantuml_regex("^(=+)(.+)$")),
                on_hidden_text: false,
                style: |m| item(m, StripeStyleType::Heading),
            },
        ];

        let hidden = char_hidder::hide(line);
        for pattern in &PATTERNS {
            let text = if pattern.on_hidden_text {
                &hidden
            } else {
                line
            };
            if let Some(captures) = pattern.regex.captures(text) {
                let (text, style) = (pattern.style)(&captures);
                let text = if pattern.on_hidden_text {
                    char_hidder::unhide(&text)
                } else {
                    text
                };
                return (text, style);
            }
        }
        (line.to_owned(), Self::NORMAL)
    }
}

fn line_with_title(captures: &regex::Captures, drawn_with: char) -> (String, StripeStyle) {
    let style = StripeStyle::of(StripeStyleType::HorizontalLine(drawn_with), 0);
    (captures[1].to_owned(), style)
}

/// A list item or heading: the markup's length gives the nesting depth.
fn item(captures: &regex::Captures, kind: StripeStyleType) -> (String, StripeStyle) {
    let order = captures[1].chars().count() - 1;
    (
        java::trim(&captures[2]).to_owned(),
        StripeStyle::of(kind, order),
    )
}

/// Collects the atoms of one stripe (PlantUML's `StripeSimple`).
struct StripeBuilder {
    font: FontConfiguration,
    style: StripeStyle,
    alignment: HorizontalAlignment,
    atoms: Vec<Box<dyn Atom>>,
}

impl StripeBuilder {
    fn new(
        font: FontConfiguration,
        style: StripeStyle,
        alignment: HorizontalAlignment,
        header: Option<Box<dyn Atom>>,
    ) -> Self {
        Self {
            font,
            style,
            alignment,
            atoms: header.into_iter().collect(),
        }
    }

    fn analyze_and_add(&mut self, line: &str) {
        let line = self.manage_cell_alignment(line);
        let line = char_hidder::hide(line);
        match self.style.kind {
            StripeStyleType::Heading => {
                self.font = heading_font(&self.font, self.style.order);
                self.modify_stripe(&line);
            }
            StripeStyleType::HorizontalLine(_) => {
                let title = (!line.is_empty()).then(|| {
                    CreoleParser::new(self.font.clone(), HorizontalAlignment::Left)
                        .create_sheet(&[line])
                });
                self.atoms.push(Box::new(HorizontalLine::new(title)));
            }
            _ => self.modify_stripe(&line),
        }
    }

    /// A leading `<left>`, `<center>` or `<right>` (or `<l>`, `<c>`, `<r>`) aligns the line.
    fn manage_cell_alignment<'a>(&mut self, line: &'a str) -> &'a str {
        const MARKERS: [(&str, HorizontalAlignment); 6] = [
            ("<l>", HorizontalAlignment::Left),
            ("<left>", HorizontalAlignment::Left),
            ("<center>", HorizontalAlignment::Center),
            ("<c>", HorizontalAlignment::Center),
            ("<right>", HorizontalAlignment::Right),
            ("<r>", HorizontalAlignment::Right),
        ];
        for (marker, alignment) in MARKERS {
            if let Some(rest) = line.strip_prefix(marker) {
                self.alignment = alignment;
                return rest;
            }
        }
        line
    }

    fn modify_stripe(&mut self, line: &str) {
        if !line.is_empty() {
            self.atoms
                .push(Box::new(AtomText::legacy(line, self.font.clone())));
        }
    }

    fn build(mut self) -> Stripe {
        if self.atoms.is_empty() {
            self.atoms
                .push(Box::new(AtomText::legacy(" ", self.font.clone())));
        }
        Stripe {
            atoms: self.atoms,
            cell_alignment: self.alignment,
        }
    }
}

fn heading_font(font: &FontConfiguration, order: usize) -> FontConfiguration {
    match order {
        0 => font.bigger(4.0).with_style(FontStyle::Bold),
        1 => font.bigger(2.0).with_style(FontStyle::Bold),
        2 => font.bigger(1.0).with_style(FontStyle::Bold),
        _ => font.with_style(FontStyle::Italic),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_angle_brackets_become_guillemets() {
        assert_eq!(
            manage_guillemet("a <<stereo>> b << x >>"),
            "a «stereo» b «x»"
        );
        assert_eq!(manage_guillemet("a << b"), "a << b");
    }

    #[test]
    fn line_markup_sets_the_style() {
        let parsed = |line| StripeStyle::parse(line);
        let style = StripeStyle::of;
        assert_eq!(
            parsed("== Title "),
            ("Title".to_owned(), style(StripeStyleType::Heading, 1))
        );
        assert_eq!(
            parsed("== Title =="),
            (
                " Title ".to_owned(),
                style(StripeStyleType::HorizontalLine('='), 0)
            )
        );
        assert_eq!(
            parsed("===="),
            (
                String::new(),
                style(StripeStyleType::HorizontalLine('='), 0)
            )
        );
        assert_eq!(
            parsed("----"),
            (
                String::new(),
                style(StripeStyleType::HorizontalLine('-'), 0)
            )
        );
        assert_eq!(
            parsed("..x.."),
            (
                "x".to_owned(),
                style(StripeStyleType::HorizontalLine('.'), 0)
            )
        );
        assert_eq!(
            parsed("** item"),
            (
                "item".to_owned(),
                style(StripeStyleType::ListWithoutNumber, 1)
            )
        );
        assert_eq!(
            parsed("**bold**"),
            ("**bold**".to_owned(), StripeStyle::NORMAL)
        );
        assert_eq!(
            parsed("## two"),
            ("two".to_owned(), style(StripeStyleType::ListWithNumber, 1))
        );
        assert_eq!(
            parsed("~# no list"),
            ("~# no list".to_owned(), StripeStyle::NORMAL)
        );
        assert_eq!(parsed("plain").1, StripeStyle::NORMAL);
    }

    #[test]
    fn list_numbers_restart_below_a_shallower_item() {
        let mut numbers = ListNumbers::default();
        let sequence: Vec<usize> = [0, 0, 1, 1, 0, 1]
            .iter()
            .map(|&order| numbers.next(order))
            .collect();
        assert_eq!(sequence, [0, 1, 0, 1, 2, 0]);
    }
}

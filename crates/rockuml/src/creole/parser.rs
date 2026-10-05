use std::sync::LazyLock;

use regex::Regex;

use super::atom_text::AtomText;
use super::{Atom, Sheet, Stripe, char_hidder};
use crate::java;
use crate::jaws::BLOCK_E1_NEWLINE;
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::{FontConfiguration, FontStyle};
use crate::pattern::java_regex;

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
        let mut stripes: Vec<Stripe> = Vec::new();
        for line in lines {
            let alignment = stripes
                .last()
                .map_or(self.horizontal_alignment, |stripe| stripe.cell_alignment);
            stripes.extend(self.create_stripes(&manage_guillemet(line.as_ref()), alignment));
        }
        Sheet { stripes }
    }

    fn create_stripes(&self, line: &str, alignment: HorizontalAlignment) -> Vec<Stripe> {
        let (text, style) = StripeStyle::parse(line);
        java::split(&text, &BLOCK_E1_NEWLINE.to_string())
            .iter()
            .map(|single_line| {
                let mut stripe = StripeBuilder::new(self.font.clone(), style, alignment);
                stripe.analyze_and_add(single_line);
                stripe.build()
            })
            .collect()
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
}

/// How a whole line is styled, from markup at its start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StripeStyle {
    kind: StripeStyleType,
    order: usize,
}

impl StripeStyle {
    const NORMAL: Self = Self {
        kind: StripeStyleType::Normal,
        order: 0,
    };

    /// The line's text without the markup, and its style.
    fn parse(line: &str) -> (String, Self) {
        static HEADING: LazyLock<Regex> = LazyLock::new(|| java_regex("^(=+)(.+)$", true));
        if let Some(captures) = HEADING.captures(line) {
            let style = Self {
                kind: StripeStyleType::Heading,
                order: captures[1].len() - 1,
            };
            return (java::trim(&captures[2]).to_owned(), style);
        }
        (line.to_owned(), Self::NORMAL)
    }
}

/// Collects the atoms of one stripe (PlantUML's `StripeSimple`).
struct StripeBuilder {
    font: FontConfiguration,
    style: StripeStyle,
    alignment: HorizontalAlignment,
    atoms: Vec<Box<dyn Atom>>,
}

impl StripeBuilder {
    fn new(font: FontConfiguration, style: StripeStyle, alignment: HorizontalAlignment) -> Self {
        Self {
            font,
            style,
            alignment,
            atoms: Vec::new(),
        }
    }

    fn analyze_and_add(&mut self, line: &str) {
        let line = self.manage_cell_alignment(line);
        let line = char_hidder::hide(line);
        if self.style.kind == StripeStyleType::Heading {
            self.font = heading_font(&self.font, self.style.order);
        }
        self.modify_stripe(&line);
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
    fn equals_signs_start_headings() {
        assert_eq!(
            StripeStyle::parse("== Title "),
            (
                "Title".to_owned(),
                StripeStyle {
                    kind: StripeStyleType::Heading,
                    order: 1
                }
            )
        );
        assert_eq!(StripeStyle::parse("plain").1, StripeStyle::NORMAL);
    }
}

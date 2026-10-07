use std::sync::LazyLock;

use regex::Regex;

use super::atom_text::AtomText;
use super::atoms::{AtomOpenIconic, AtomWithMargin, Bullet, HorizontalLine};
use super::code::{self, AtomCode};
use super::commands::{CreoleCommand, creole_commands};
use super::display::Display;
use super::table::{self, AtomTable};
use super::tree::{self, AtomTree};
use super::{Atom, CreoleMode, Sheet, Stripe, char_hidder};
use crate::color::HColor;
use crate::java;
use crate::jaws::BLOCK_E1_NEWLINE;
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::{FontConfiguration, FontStyle};
use crate::klimt::url::Url;
use crate::openiconic::OpenIconic;
use crate::pattern::{java_regex, plantuml_regex};
use crate::stereo::Stereotype;

/// Turns the lines of a label into a [`Sheet`] (PlantUML's legacy `CreoleParser`).
pub(crate) struct CreoleParser {
    font: FontConfiguration,
    horizontal_alignment: HorizontalAlignment,
    mode: CreoleMode,
}

impl CreoleParser {
    pub(crate) fn new(font: FontConfiguration, horizontal_alignment: HorizontalAlignment) -> Self {
        Self::with_mode(font, horizontal_alignment, CreoleMode::Full)
    }

    pub(crate) fn with_mode(
        font: FontConfiguration,
        horizontal_alignment: HorizontalAlignment,
        mode: CreoleMode,
    ) -> Self {
        Self {
            font,
            horizontal_alignment,
            mode,
        }
    }

    pub(crate) fn create_sheet(&self, lines: &[impl AsRef<str>]) -> Sheet {
        self.create_sheet_of(
            lines
                .iter()
                .map(|line| (manage_guillemet(line.as_ref()), &self.font)),
        )
    }

    /// The sheet of a display: its lines, and its stereotype's labels in `stereotype_font`.
    pub(crate) fn create_display_sheet(
        &self,
        display: &Display,
        stereotype_font: &FontConfiguration,
    ) -> Sheet {
        let lines = display
            .lines()
            .iter()
            .map(|line| (manage_guillemet(line), &self.font));
        let labels = display
            .stereotype()
            .map(Stereotype::labels)
            .unwrap_or_default()
            .into_iter()
            .map(|label| (label, stereotype_font));
        if display.is_stereotype_first() {
            self.create_sheet_of(labels.chain(lines))
        } else {
            self.create_sheet_of(lines.chain(labels))
        }
    }

    /// Each line comes in the font it is written in.
    fn create_sheet_of<'a>(
        &self,
        lines: impl Iterator<Item = (String, &'a FontConfiguration)>,
    ) -> Sheet {
        let mut list_numbers = ListNumbers::default();
        let mut stripes: Vec<Stripe> = Vec::new();
        let mut open_block: Option<MultilineBlock> = None;
        for (line, font) in lines {
            if let Some(block) = &mut open_block
                && block.continues_with(&line)
            {
                block.add_line(&line, font);
                continue;
            }
            if let Some(block) = open_block.take() {
                stripes.push(block.into_stripe(self.horizontal_alignment));
            }
            if table::is_table_line(&line) {
                open_block = Some(MultilineBlock::Table(AtomTable::new(&line, font)));
            } else if tree::is_tree_start(&line) {
                open_block = Some(MultilineBlock::Tree(AtomTree::new(&line, font)));
            } else if code::is_code_start(&line) {
                open_block = Some(MultilineBlock::Code(AtomCode::new(font)));
            } else {
                let alignment = stripes
                    .last()
                    .map_or(self.horizontal_alignment, |stripe| stripe.cell_alignment);
                stripes.extend(self.create_stripes(&line, font, alignment, &mut list_numbers));
            }
        }
        stripes.extend(open_block.map(|block| block.into_stripe(self.horizontal_alignment)));
        Sheet { stripes }
    }

    fn create_stripes(
        &self,
        line: &str,
        font: &FontConfiguration,
        alignment: HorizontalAlignment,
        list_numbers: &mut ListNumbers,
    ) -> Vec<Stripe> {
        let (text, style) = StripeStyle::parse(line, self.mode);
        java::split(&text, &BLOCK_E1_NEWLINE.to_string())
            .iter()
            .map(|single_line| {
                let header = header(font, style, list_numbers);
                let mut stripe =
                    StripeBuilder::new(font.clone(), style, alignment, header, self.mode);
                stripe.analyze_and_add(single_line);
                stripe.build()
            })
            .collect()
    }
}

/// What a list item starts with.
fn header(
    font: &FontConfiguration,
    style: StripeStyle,
    list_numbers: &mut ListNumbers,
) -> Option<Box<dyn Atom>> {
    match style.kind {
        StripeStyleType::ListWithoutNumber => {
            Some(Box::new(Bullet::new(font.clone(), style.order)))
        }
        StripeStyleType::ListWithNumber => {
            let number = list_numbers.next(style.order);
            Some(Box::new(AtomText::list_number(
                font.clone(),
                style.order,
                number,
            )))
        }
        _ => None,
    }
}

/// A table, tree or code block, which following lines extend.
enum MultilineBlock {
    Table(AtomTable),
    Tree(AtomTree),
    Code(AtomCode),
}

impl MultilineBlock {
    /// Tables and trees keep this much space above and below them; code blocks none.
    const MARGIN: f64 = 2.0;

    fn continues_with(&self, line: &str) -> bool {
        match self {
            Self::Table(_) => table::is_table_line(line),
            Self::Tree(_) => tree::is_tree_start(java::trim(line)),
            Self::Code(code) => !code.is_terminated(),
        }
    }

    fn add_line(&mut self, line: &str, font: &FontConfiguration) {
        match self {
            Self::Table(table) => table.add_line(line, font),
            Self::Tree(tree) => tree.add_line(line, font),
            Self::Code(code) => code.add_line(line),
        }
    }

    /// Aligned like the sheet, as not being a line of text.
    fn into_stripe(self, alignment: HorizontalAlignment) -> Stripe {
        let atom: Box<dyn Atom> = match self {
            Self::Table(table) => Box::new(AtomWithMargin::new(table, Self::MARGIN, Self::MARGIN)),
            Self::Tree(tree) => Box::new(AtomWithMargin::new(tree, Self::MARGIN, Self::MARGIN)),
            Self::Code(code) => Box::new(code),
        };
        Stripe {
            atoms: vec![atom],
            cell_alignment: alignment,
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

    /// The line's text without the markup, and its style. Lists need full creole.
    fn parse(line: &str, mode: CreoleMode) -> (String, Self) {
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
                let is_list = matches!(
                    style.kind,
                    StripeStyleType::ListWithoutNumber | StripeStyleType::ListWithNumber
                );
                if is_list && mode != CreoleMode::Full {
                    continue;
                }
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

/// The first command that applies at the start of `rest`. Commands are only looked for where at least
/// three UTF-16 units remain, as in PlantUML.
fn command_at(rest: &str, mode: CreoleMode) -> Option<&'static dyn CreoleCommand> {
    let units: usize = rest.chars().take(3).map(char::len_utf16).sum();
    if units <= 2 {
        return None;
    }
    let prefix_length: usize = rest.chars().take(2).map(char::len_utf8).sum();
    let prefix = &rest[..prefix_length];
    creole_commands(mode)
        .iter()
        .find(|command| command.starters().contains(&prefix) && command.matches(rest))
        .map(AsRef::as_ref)
}

/// Collects the atoms of one stripe (PlantUML's `StripeSimple`).
pub(super) struct StripeBuilder {
    font: FontConfiguration,
    style: StripeStyle,
    alignment: HorizontalAlignment,
    atoms: Vec<Box<dyn Atom>>,
    mode: CreoleMode,
}

impl StripeBuilder {
    fn new(
        font: FontConfiguration,
        style: StripeStyle,
        alignment: HorizontalAlignment,
        header: Option<Box<dyn Atom>>,
        mode: CreoleMode,
    ) -> Self {
        Self {
            font,
            style,
            alignment,
            atoms: header.into_iter().collect(),
            mode,
        }
    }

    /// A stripe of plain text, as in table cells and tree items.
    pub(super) fn plain(font: FontConfiguration, mode: CreoleMode) -> Self {
        Self::new(
            font,
            StripeStyle::NORMAL,
            HorizontalAlignment::Left,
            None,
            mode,
        )
    }

    pub(super) fn analyze_and_add(&mut self, line: &str) {
        let line = self.manage_cell_alignment(line);
        let line = char_hidder::hide(line);
        match self.style.kind {
            StripeStyleType::Heading => {
                self.font = heading_font(&self.font, self.style.order);
                self.modify_stripe(&line);
            }
            StripeStyleType::HorizontalLine(style) => {
                let title = (!line.is_empty()).then(|| {
                    CreoleParser::new(self.font.clone(), HorizontalAlignment::Left)
                        .create_sheet(Display::with_newlines(&line).lines())
                });
                self.atoms.push(Box::new(HorizontalLine::new(style, title)));
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

    /// Splits the line into runs of plain text and inline commands, which add their own atoms.
    fn modify_stripe(&mut self, line: &str) {
        let mut pending = String::new();
        let mut rest = line;
        while let Some(c) = rest.chars().next() {
            if let Some(command) = command_at(rest, self.mode) {
                self.add_text(&mut pending);
                let consumed = command.execute(rest, self);
                rest = &rest[consumed..];
            } else {
                pending.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        self.add_text(&mut pending);
    }

    fn add_text(&mut self, pending: &mut String) {
        if !pending.is_empty() {
            let text = std::mem::take(pending);
            self.atoms
                .push(Box::new(AtomText::legacy(&text, self.font.clone())));
        }
    }

    pub(super) fn add_url(&mut self, url: Url) {
        self.atoms
            .push(Box::new(AtomText::link(url, self.font.hyperlink())));
    }

    /// An unknown icon is left out.
    pub(super) fn add_open_icon(&mut self, src: &str, scale: f64, color: Option<HColor>) {
        if let Some(open_iconic) = OpenIconic::retrieve(src) {
            self.atoms.push(Box::new(AtomOpenIconic::new(
                color,
                scale,
                open_iconic,
                &self.font,
            )));
        }
    }

    /// Adds `text` in a changed font, then goes back to the current one.
    pub(super) fn with_font(
        &mut self,
        change: impl FnOnce(&FontConfiguration) -> FontConfiguration,
        text: &str,
    ) {
        let current = self.font.clone();
        self.font = change(&current);
        self.analyze_and_add(text);
        self.font = current;
    }

    pub(super) fn build(mut self) -> Stripe {
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
        let parsed = |line| StripeStyle::parse(line, CreoleMode::Full);
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

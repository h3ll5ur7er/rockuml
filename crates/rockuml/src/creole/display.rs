use super::{CreoleMode, CreoleParser, SheetBlock1, SheetBlock2};
use crate::java;
use crate::jaws::{
    BLOCK_E1_BREAKLINE, BLOCK_E1_NEWLINE, BLOCK_E1_NEWLINE_LEFT_ALIGN,
    BLOCK_E1_NEWLINE_RIGHT_ALIGN, BLOCK_E1_REAL_BACKSLASH,
};
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::FontConfiguration;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::sprite::SpriteContainer;
use crate::skin::visibility_modifier::VisibilityModifier;
use crate::stereo::Stereotype;

/// Marks a quote PlantUML keeps out of the text.
const BLOCK_E1_INVISIBLE_QUOTE: char = '\u{E121}';

/// The lines of a label, with the alignment its line breaks asked for, and the stereotype shown with them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Display {
    lines: Vec<String>,
    natural_alignment: Option<HorizontalAlignment>,
    stereotype: Option<DisplayedStereotype>,
}

/// PlantUML keeps a shown stereotype as the first or the last line of the display.
#[derive(Clone, Debug, PartialEq)]
struct DisplayedStereotype {
    stereotype: Stereotype,
    first: bool,
}

impl Display {
    pub(crate) fn create(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            lines: lines.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// The display with a stereotype before (`first`) or after its lines (`addFirst` and `add`).
    #[must_use]
    pub(crate) fn with_stereotype(&self, stereotype: Stereotype, first: bool) -> Self {
        Self {
            stereotype: Some(DisplayedStereotype { stereotype, first }),
            ..self.clone()
        }
    }

    pub(crate) fn stereotype(&self) -> Option<&Stereotype> {
        self.stereotype.as_ref().map(|shown| &shown.stereotype)
    }

    /// Whether the stereotype comes before the lines.
    pub(crate) fn is_stereotype_first(&self) -> bool {
        self.stereotype.as_ref().is_some_and(|shown| shown.first)
    }

    /// The first line, as a tooltip shows it.
    pub(crate) fn tooltip_text(&self) -> String {
        match (self.lines.first(), &self.stereotype) {
            (Some(_), Some(shown)) if shown.first => shown.stereotype.to_string(),
            (Some(line), _) => line.clone(),
            (None, Some(shown)) => shown.stereotype.to_string(),
            (None, None) => String::new(),
        }
    }

    /// A label written on one line: `\n`, `\l` and `\r` break it (left- or right-aligning it for the last
    /// two), except inside `<math>`, `<latex>` and `[[links]]`.
    pub(crate) fn with_newlines(text: &str) -> Self {
        /// A line break, and the alignment it asks for.
        enum Break {
            Plain,
            Aligned(HorizontalAlignment),
        }
        let mut lines = vec![String::new()];
        let mut natural_alignment = None;
        let mut raw = false;
        let mut chars = text.char_indices().peekable();
        while let Some((at, c)) = chars.next() {
            let rest = &text[at..];
            if ["<math>", "<latex>", "[["]
                .iter()
                .any(|start| rest.starts_with(start))
            {
                raw = true;
            } else if ["</math>", "</latex>", "]]"]
                .iter()
                .any(|end| rest.starts_with(end))
            {
                raw = false;
            }
            let current = lines.last_mut().expect("there is always a current line");
            let line_break = match c {
                '\\' if !raw && chars.peek().is_some() => match chars.next().expect("peeked").1 {
                    'n' => Some(Break::Plain),
                    'r' => Some(Break::Aligned(HorizontalAlignment::Right)),
                    'l' => Some(Break::Aligned(HorizontalAlignment::Left)),
                    escaped => {
                        match escaped {
                            't' => current.push('\t'),
                            '\\' => current.push('\\'),
                            other => {
                                current.push('\\');
                                current.push(other);
                            }
                        }
                        None
                    }
                },
                BLOCK_E1_NEWLINE_LEFT_ALIGN => Some(Break::Aligned(HorizontalAlignment::Left)),
                BLOCK_E1_NEWLINE_RIGHT_ALIGN => Some(Break::Aligned(HorizontalAlignment::Right)),
                BLOCK_E1_NEWLINE if !raw => Some(Break::Plain),
                BLOCK_E1_BREAKLINE => Some(Break::Plain),
                BLOCK_E1_REAL_BACKSLASH => {
                    current.push('\\');
                    None
                }
                BLOCK_E1_INVISIBLE_QUOTE => None,
                _ => {
                    current.push(c);
                    None
                }
            };
            match line_break {
                Some(Break::Aligned(alignment)) => {
                    natural_alignment = Some(alignment);
                    lines.push(String::new());
                }
                Some(Break::Plain) => lines.push(String::new()),
                None => {}
            }
        }
        Self {
            lines,
            natural_alignment,
            stereotype: None,
        }
    }

    /// Written `\t` becomes a tabulation.
    #[must_use]
    pub(crate) fn replace_backslash_t(&self) -> Self {
        self.map_lines(|line| line.replace("\\t", "\t"))
    }

    /// Every `from` in every line becomes `to`.
    #[must_use]
    pub(crate) fn replace(&self, from: &str, to: &str) -> Self {
        self.map_lines(|line| line.replace(from, to))
    }

    /// `<<stereotypes>>` in the lines read as `«stereotypes»`; the visibility character starting the first
    /// line goes when `manage_visibility_modifier`, as it shows as an icon.
    #[must_use]
    pub(crate) fn manage_guillemet(&self, manage_visibility_modifier: bool) -> Self {
        let mut result = self.map_lines(super::parser::manage_guillemet);
        if manage_visibility_modifier
            && let Some(first) = self.lines.first()
            && VisibilityModifier::is_visibility_character(first)
        {
            let rest: String = first.chars().skip(1).collect();
            result.lines[0] = super::parser::manage_guillemet(java::trim(&rest));
        }
        result
    }

    /// `appended` goes before the first line.
    ///
    /// # Panics
    ///
    /// If there is no first line.
    #[must_use]
    pub(crate) fn append_first_line(&self, appended: &str) -> Self {
        let mut result = self.clone();
        result.lines[0].insert_str(0, appended);
        result
    }

    /// `<generic>` after the last line.
    #[must_use]
    pub(crate) fn add_generic(&self, generic: &str) -> Self {
        let mut result = self.clone();
        let generic = format!("<{generic}>");
        match result.lines.last_mut() {
            Some(last) => last.push_str(&generic),
            None => result.lines.push(generic),
        }
        result
    }

    #[must_use]
    pub(crate) fn underlined(&self) -> Self {
        self.map_lines(|line| format!("<u>{line}"))
    }

    fn map_lines(&self, change: impl Fn(&str) -> String) -> Self {
        Self {
            lines: self.lines.iter().map(|line| change(line)).collect(),
            natural_alignment: self.natural_alignment,
            stereotype: self.stereotype.clone(),
        }
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn natural_alignment(&self) -> Option<HorizontalAlignment> {
        self.natural_alignment
    }

    /// The display drawn in `font`, its lines wrapping beyond `max_width` unless it is 0 (`Display.create0`;
    /// `create`, `create7` and `create8` call it).
    pub(crate) fn create0(
        &self,
        font: &FontConfiguration,
        alignment: HorizontalAlignment,
        sprites: &dyn SpriteContainer,
        max_width: f64,
        mode: CreoleMode,
    ) -> SheetBlock2 {
        let alignment = self.natural_alignment.unwrap_or(alignment);
        let sheet = CreoleParser::with_mode(font.clone(), alignment, mode, sprites)
            .create_display_sheet(self, font);
        SheetBlock2::new(
            SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none()).wrapped_at(max_width),
        )
    }

    pub(crate) fn is_single_empty_line(&self) -> bool {
        matches!(self.lines.as_slice(), [only] if only.is_empty())
    }

    /// No lines, or a single one of only (ASCII) whitespace.
    pub(crate) fn is_white(&self) -> bool {
        match self.lines.as_slice() {
            [] => true,
            [only] => only.chars().all(crate::java::is_regex_whitespace),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backslash_escapes_break_lines_and_align() {
        let display = Display::with_newlines(r"a\nb\rc\td\\e\x");
        assert_eq!(display.lines(), ["a", "b", "c\td\\e\\x"]);
        assert_eq!(
            display.natural_alignment(),
            Some(HorizontalAlignment::Right)
        );
    }

    #[test]
    fn package_names_join_on_the_first_line() {
        let display = Display::create(["b", "second"]).append_first_line("a.");
        assert_eq!(display.lines(), ["a.b", "second"]);
    }

    #[test]
    fn stereotypes_in_labels_get_guillemets() {
        let display = Display::create(["uses <<friend>>"]).manage_guillemet(false);
        assert_eq!(display.lines(), ["uses \u{AB}friend\u{BB}"]);
        let display = Display::create(["+ uses <<friend>>"]).manage_guillemet(true);
        assert_eq!(display.lines(), ["uses \u{AB}friend\u{BB}"]);
    }

    #[test]
    fn links_keep_their_escapes() {
        assert_eq!(
            Display::with_newlines(r"[[a\nb]]\nc").lines(),
            [r"[[a\nb]]", "c"]
        );
    }

    #[test]
    fn tooltips_show_the_first_line_which_may_be_the_stereotype() {
        let display = Display::create(["Bob"]);
        let stereotype = Stereotype::with_spot("<< (C,red) Testable >>").unwrap();
        assert_eq!(display.tooltip_text(), "Bob");
        let first = display.with_stereotype(stereotype.clone(), true);
        assert_eq!(first.tooltip_text(), "C << Testable >>");
        assert_eq!(
            display.with_stereotype(stereotype, false).tooltip_text(),
            "Bob"
        );
    }

    #[test]
    fn underlining_keeps_the_alignment() {
        let display = Display::with_newlines(r"a\rb").underlined();
        assert_eq!(display.lines(), ["<u>a", "<u>b"]);
        assert_eq!(
            display.natural_alignment(),
            Some(HorizontalAlignment::Right)
        );
    }

    #[test]
    fn only_one_empty_line_is_a_single_empty_line() {
        assert!(Display::create([""]).is_single_empty_line());
        assert!(!Display::create(["", ""]).is_single_empty_line());
        assert!(!Display::create([" "]).is_single_empty_line());
    }

    #[test]
    fn whitespace_only_labels_are_white() {
        assert!(Display::with_newlines("  ").is_white());
        assert!(!Display::with_newlines("x").is_white());
        assert!(!Display::with_newlines(r" \n ").is_white());
    }
}

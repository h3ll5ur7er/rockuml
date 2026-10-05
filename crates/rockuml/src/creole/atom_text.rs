use super::{Atom, char_hidder};
use crate::jaws::BLOCK_E1_REAL_TABULATION;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UShape, UText};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::url::Url;
use crate::klimt::{TextBlock, layout_tabulated};

/// A run of text in one font.
#[derive(Clone, Debug)]
pub(super) struct AtomText {
    text: String,
    font: FontConfiguration,
    margins: Margins,
    url: Option<Url>,
}

/// Space kept free beside the text, measured in the text's own font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Margins {
    None,
    /// A list number is indented by its nesting depth and followed by the width of a dot.
    ListNumber {
        order: usize,
    },
}

impl AtomText {
    /// Text from creole markup, which may still hold escapes and character references.
    pub(super) fn legacy(text: &str, font: FontConfiguration) -> Self {
        Self {
            text: manage_special_chars(&char_hidder::unhide(text)),
            font,
            margins: Margins::None,
            url: None,
        }
    }

    /// The label of a link, which clicking on follows it.
    pub(super) fn link(url: Url, font: FontConfiguration) -> Self {
        let label = Self::legacy(&url.label, font);
        Self {
            url: Some(url),
            ..label
        }
    }

    /// The number in front of a `#` list item; `local_number` counts from zero.
    pub(super) fn list_number(font: FontConfiguration, order: usize, local_number: usize) -> Self {
        Self {
            margins: Margins::ListNumber { order },
            ..Self::legacy(&format!("{}.", local_number + 1), font)
        }
    }

    fn margin_left(&self, string_bounder: &dyn StringBounder) -> f64 {
        match self.margins {
            Margins::None => 0.0,
            Margins::ListNumber { order } => self.width_of("9. ", string_bounder) * order as f64,
        }
    }

    fn margin_right(&self, string_bounder: &dyn StringBounder) -> f64 {
        match self.margins {
            Margins::None => 0.0,
            Margins::ListNumber { .. } => self.width_of(".", string_bounder),
        }
    }

    fn width_of(&self, text: &str, string_bounder: &dyn StringBounder) -> f64 {
        string_bounder
            .calculate_dimension(&self.font.font(), text)
            .width
    }

    fn is_tabulation(c: char) -> bool {
        c == '\t' || c == BLOCK_E1_REAL_TABULATION
    }

    fn tab_size(&self, string_bounder: &dyn StringBounder) -> f64 {
        let spaces = match self.font.tab_size() {
            size @ 1..7 => " ".repeat(size as usize),
            _ => " ".repeat(8),
        };
        let width = string_bounder
            .calculate_dimension(&self.font.font(), &spaces)
            .width;
        if width == 0.0 {
            self.font.font().size_2d() * 4.0
        } else {
            width
        }
    }

    /// Calls `visit` with each piece of text and its x offset, advancing past tabulations to the next stop.
    fn layout_pieces(
        &self,
        string_bounder: &dyn StringBounder,
        visit: impl FnMut(&str, f64),
    ) -> f64 {
        let font = self.font.font();
        layout_tabulated(
            &self.text,
            self.tab_size(string_bounder),
            |text| string_bounder.calculate_dimension(&font, text).width,
            visit,
        )
    }
}

impl TextBlock for AtomText {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let rect = string_bounder.calculate_dimension(&self.font.font(), &self.text);
        let width = if self.text.contains(Self::is_tabulation) {
            self.layout_pieces(string_bounder, |_, _| {})
        } else {
            rect.width
        };
        let margins = self.margin_left(string_bounder) + self.margin_right(string_bounder);
        XDimension2D::new(width + margins, rect.height.max(10.0))
    }

    fn draw_u(&self, ug: &UGraphic) {
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        self.draw_text(ug);
        if self.url.is_some() {
            ug.close_url();
        }
    }
}

impl AtomText {
    fn draw_text(&self, ug: &UGraphic) {
        let ug = &ug.translated(self.margin_left(ug.string_bounder()), 0.0);
        let string_bounder = ug.string_bounder();
        let font = self.font.font();
        let rect = string_bounder.calculate_dimension(&font, &self.text);
        let baseline = rect.height - string_bounder.descent(&font, &self.text);
        self.layout_pieces(string_bounder, |piece, x| {
            let text = UText::new(piece, self.font.clone());
            ug.translated(x, baseline).draw(&UShape::Text(text));
        });
    }
}

impl Atom for AtomText {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        f64::from(self.font.space())
    }
}

/// Resolves `&#NNN;` and `<U+XXXX>` character references, `~@start` and `\t`.
fn manage_special_chars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        let replacement = match c {
            '&' => decimal_reference(rest),
            '<' => unicode_reference(rest),
            '~' => rest
                .starts_with("~@start")
                .then(|| ("@start".to_owned(), "~@start".len())),
            '\\' => rest.starts_with("\\t").then(|| ("\t".to_owned(), 2)),
            _ => None,
        };
        let consumed = if let Some((text, consumed)) = replacement {
            result.push_str(&text);
            consumed
        } else {
            result.push(c);
            c.len_utf8()
        };
        rest = &rest[consumed..];
    }
    result
}

/// `&#233;`: the character and how many bytes the reference takes.
fn decimal_reference(s: &str) -> Option<(String, usize)> {
    let digits = s.strip_prefix("&#")?;
    let length = digits.find(|c: char| !c.is_ascii_digit())?;
    if length == 0 || !digits[length..].starts_with(';') {
        return None;
    }
    let character = char::from_u32(digits[..length].parse().ok()?)?;
    Some((character.to_string(), "&#".len() + length + 1))
}

/// `<U+1F600>`: four or five hex digits.
fn unicode_reference(s: &str) -> Option<(String, usize)> {
    let digits = s.strip_prefix("<U+")?;
    let length = digits
        .char_indices()
        .take(5)
        .take_while(|(_, c)| c.is_ascii_hexdigit())
        .count();
    if length < 4 || !digits[length..].starts_with('>') {
        return None;
    }
    let character = char::from_u32(u32::from_str_radix(&digits[..length], 16).ok()?)?;
    Some((character.to_string(), "<U+".len() + length + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_references_are_resolved() {
        assert_eq!(manage_special_chars("caf&#233; <U+1F600>!"), "café 😀!");
        assert_eq!(
            manage_special_chars("&#; &#12 <U+12> ~@startuml a\\tb"),
            "&#; &#12 <U+12> @startuml a\tb"
        );
    }
}

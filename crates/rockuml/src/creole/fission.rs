//! Wrapping a creole line that grows wider than allowed (PlantUML's `Fission` and `Neutron`): the line is cut
//! into neutrons, which are refilled into lines that break only where a separator allows.

use std::rc::Rc;

use super::{Atom, Stripe};
use crate::java;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NeutronType {
    Whitespace,
    CjkIdeograph,
    Unbreakable,
    /// Where a line may break; it takes no room.
    ZwspSeparator,
    /// An atom that is not text, kept whole.
    Unknown,
}

impl NeutronType {
    pub(super) fn of_char(c: char) -> Self {
        if java::is_whitespace(c) {
            Self::Whitespace
        } else if is_cjk_or_japanese(c) {
            Self::CjkIdeograph
        } else {
            Self::Unbreakable
        }
    }
}

/// Kanji, kana and Japanese punctuation, which a line may break around.
fn is_cjk_or_japanese(c: char) -> bool {
    matches!(
        c,
        '\u{4E00}'..='\u{9FFF}'
            | '\u{3040}'..='\u{309F}'
            | '\u{30A0}'..='\u{30FF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{FF65}'..='\u{FF9F}'
    )
}

#[derive(Clone)]
pub(crate) struct Neutron {
    kind: NeutronType,
    atom: Option<Rc<dyn Atom>>,
}

impl Neutron {
    fn zwsp_separator() -> Self {
        Self {
            kind: NeutronType::ZwspSeparator,
            atom: None,
        }
    }

    /// A piece of text, typed by its first character.
    pub(super) fn text(first: char, atom: Rc<dyn Atom>) -> Self {
        Self {
            kind: NeutronType::of_char(first),
            atom: Some(atom),
        }
    }

    fn whole(atom: Rc<dyn Atom>) -> Self {
        Self {
            kind: NeutronType::Unknown,
            atom: Some(atom),
        }
    }

    fn width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.atom
            .as_ref()
            .map_or(0.0, |atom| atom.calculate_dimension(string_bounder).width)
    }

    /// Adds `piece` to `neutrons`, between separators where a line may break around it.
    pub(super) fn add_breakable(neutrons: &mut Vec<Neutron>, piece: Neutron) {
        let breakable = matches!(
            piece.kind,
            NeutronType::Whitespace | NeutronType::CjkIdeograph
        );
        if breakable {
            neutrons.push(Self::zwsp_separator());
        }
        neutrons.push(piece);
        if breakable {
            neutrons.push(Self::zwsp_separator());
        }
    }
}

fn neutrons_of(atom: &Rc<dyn Atom>) -> Vec<Neutron> {
    atom.neutrons()
        .unwrap_or_else(|| vec![Neutron::whole(atom.clone())])
}

/// Splits `stripe` into lines no wider than `max_width` where it can (`Fission.getSplitted`).
pub(super) fn split(
    stripe: &Stripe,
    max_width: f64,
    string_bounder: &dyn StringBounder,
) -> Vec<Stripe> {
    let max_width = max_width.abs();
    let without_header = &stripe.atoms[usize::from(stripe.header.is_some())..];
    let mut all: Vec<Neutron> = without_header.iter().flat_map(neutrons_of).collect();
    let Some(last) = all.last() else {
        return vec![stripe.clone()];
    };
    if last.kind != NeutronType::ZwspSeparator {
        all.push(Neutron::zwsp_separator());
    }

    let mut lines = vec![Line::new(false, stripe.header.clone(), string_bounder)];
    let mut all = all.into_iter().collect::<std::collections::VecDeque<_>>();
    while let Some(current) = all.pop_front() {
        let line = lines.last_mut().expect("there is always a line");
        if current.kind == NeutronType::ZwspSeparator && line.width > max_width {
            all.push_front(current);
            for neutron in line.slightly_shorten().into_iter().rev() {
                all.push_front(neutron);
            }
            let header = stripe
                .header
                .clone()
                .map(|header| Rc::new(BlankAtom(header)) as Rc<dyn Atom>);
            lines.push(Line::new(true, header, string_bounder));
        } else {
            line.add_neutron(current, string_bounder);
        }
    }

    for line in &mut lines {
        line.remove_final_spaces();
    }
    while lines.len() > 1 && lines.last().is_some_and(Line::is_white) {
        lines.pop();
    }
    lines
        .into_iter()
        .map(|line| Stripe {
            header: None,
            atoms: line.atoms(),
            cell_alignment: stripe.cell_alignment,
        })
        .collect()
}

/// A line being filled (`StripeSimpleInternal`).
struct Line {
    remove_initial_spaces: bool,
    header: Option<Rc<dyn Atom>>,
    neutrons: Vec<Neutron>,
    width: f64,
}

impl Line {
    fn new(
        remove_initial_spaces: bool,
        header: Option<Rc<dyn Atom>>,
        string_bounder: &dyn StringBounder,
    ) -> Self {
        let width = header.as_ref().map_or(0.0, |header| {
            header.calculate_dimension(string_bounder).width
        });
        Self {
            remove_initial_spaces,
            header,
            neutrons: Vec::new(),
            width,
        }
    }

    fn last_kind(&self) -> Option<NeutronType> {
        self.neutrons.last().map(|neutron| neutron.kind)
    }

    fn add_neutron(&mut self, neutron: Neutron, string_bounder: &dyn StringBounder) {
        let is_separator = neutron.kind == NeutronType::ZwspSeparator;
        if is_separator
            && (self.neutrons.is_empty() || self.last_kind() == Some(NeutronType::ZwspSeparator))
        {
            return;
        }
        if self.remove_initial_spaces
            && self.neutrons.is_empty()
            && neutron.kind == NeutronType::Whitespace
        {
            return;
        }
        self.width += neutron.width(string_bounder);
        self.neutrons.push(neutron);
    }

    /// Gives back everything from the last separator on; the line is complete afterwards.
    fn slightly_shorten(&mut self) -> Vec<Neutron> {
        let last_zwsp = self
            .neutrons
            .iter()
            .rposition(|neutron| neutron.kind == NeutronType::ZwspSeparator);
        match last_zwsp {
            Some(index) => self.neutrons.split_off(index),
            None => Vec::new(),
        }
    }

    fn remove_final_spaces(&mut self) {
        let leading = self
            .neutrons
            .iter()
            .take_while(|neutron| neutron.kind == NeutronType::ZwspSeparator)
            .count();
        self.neutrons.drain(..leading);
        while self.neutrons.len() > 1
            && matches!(
                self.last_kind(),
                Some(NeutronType::Whitespace | NeutronType::ZwspSeparator)
            )
        {
            self.neutrons.pop();
        }
    }

    fn is_white(&self) -> bool {
        self.neutrons.iter().all(|neutron| {
            matches!(
                neutron.kind,
                NeutronType::ZwspSeparator | NeutronType::Whitespace
            )
        })
    }

    fn atoms(self) -> Vec<Rc<dyn Atom>> {
        let mut result: Vec<Rc<dyn Atom>> = self.header.into_iter().collect();
        for neutron in self.neutrons {
            let Some(atom) = neutron.atom else { continue };
            if self.remove_initial_spaces
                && result.is_empty()
                && neutron.kind == NeutronType::Whitespace
            {
                continue;
            }
            result.push(atom);
        }
        result
    }
}

/// Takes a list header's room on the lines after the first, drawing nothing.
struct BlankAtom(Rc<dyn Atom>);

impl TextBlock for BlankAtom {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.0.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}

impl Atom for BlankAtom {
    fn starting_altitude(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.0.starting_altitude(string_bounder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creole::CreoleParser;
    use crate::klimt::HorizontalAlignment;
    use crate::klimt::font::{FontConfiguration, UFont};

    struct TenPerChar;

    impl StringBounder for TenPerChar {
        fn calculate_dimension(&self, _font: &UFont, text: &str) -> XDimension2D {
            XDimension2D::new(10.0 * text.chars().count() as f64, 10.0)
        }
    }

    fn line_widths(text: &str, max_width: f64) -> Vec<f64> {
        let font = FontConfiguration::black_blue_true(UFont::serif(10));
        let sheet = CreoleParser::new(font, HorizontalAlignment::Left).create_sheet(&[text]);
        split(&sheet.stripes[0], max_width, &TenPerChar)
            .iter()
            .map(|stripe| {
                stripe
                    .atoms
                    .iter()
                    .map(|atom| atom.calculate_dimension(&TenPerChar).width)
                    .sum()
            })
            .collect()
    }

    #[test]
    fn lines_break_after_the_last_space_that_fits() {
        assert_eq!(line_widths("aa bb cc", 55.0), [50.0, 20.0]);
        assert_eq!(line_widths("aa bb cc", 45.0), [20.0, 20.0, 20.0]);
    }

    #[test]
    fn a_word_too_long_stays_whole() {
        assert_eq!(line_widths("abcdefgh ij", 30.0), [80.0, 20.0]);
    }
}

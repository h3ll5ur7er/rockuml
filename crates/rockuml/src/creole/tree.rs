//! Creole trees: `|_ item` lines, nested by two spaces or a tab of indentation (PlantUML's `StripeTree`,
//! `AtomTree` and `Skeleton2`).

use super::parser::StripeBuilder;
use super::sheet_block::SheetBlock1;
use super::table::cell_lines;
use super::{Atom, CreoleMode, Sheet};
use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

/// How far each level is indented, and what the connector of each item spans.
const LEVEL_WIDTH: f64 = 8.0;
const MARGIN: f64 = 2.0;

pub(super) fn is_tree_start(line: &str) -> bool {
    line.starts_with("|_")
}

pub(super) struct AtomTree {
    line_color: HColor,
    items: Vec<(SheetBlock1, usize)>,
}

impl AtomTree {
    pub(super) fn new(line: &str, font: &FontConfiguration) -> Self {
        let mut tree = Self {
            line_color: font.color().clone(),
            items: Vec::new(),
        };
        tree.add_line(line, font);
        tree
    }

    pub(super) fn add_line(&mut self, line: &str, font: &FontConfiguration) {
        for text in cell_lines(line) {
            let level = level_of(&text);
            let mut stripe = StripeBuilder::plain(font.clone(), CreoleMode::Full);
            stripe.analyze_and_add(without_marker(&text));
            let sheet = Sheet {
                stripes: vec![stripe.build()],
            };
            let item = SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none());
            self.items.push((item, level));
        }
    }
}

/// One plus the indentation, counted in pairs of spaces and tabs.
fn level_of(mut text: &str) -> usize {
    let mut level = 1;
    loop {
        if let Some(rest) = text.strip_prefix("  ") {
            text = rest;
        } else if let Some(rest) = text.strip_prefix('\t') {
            text = rest;
        } else {
            return level;
        }
        level += 1;
    }
}

fn without_marker(text: &str) -> &str {
    text.trim_start_matches(crate::java::is_regex_whitespace)
        .strip_prefix("|_")
        .unwrap_or(text)
}

fn level_start(level: usize) -> f64 {
    level as f64 * LEVEL_WIDTH
}

fn level_end(level: usize) -> f64 {
    level_start(level) + LEVEL_WIDTH
}

impl TextBlock for AtomTree {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.items
            .iter()
            .fold(XDimension2D::default(), |total, (item, level)| {
                let dimension = item.calculate_dimension(string_bounder);
                XDimension2D::new(
                    total
                        .width
                        .max(level_end(*level) + MARGIN + dimension.width),
                    total.height + dimension.height,
                )
            })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let mut connectors: Vec<(usize, f64)> = Vec::with_capacity(self.items.len());
        let mut y = 0.0;
        for (item, level) in &self.items {
            item.draw_u(&ug.translated(MARGIN + level_end(*level), y));
            let height = item.calculate_dimension(ug.string_bounder()).height;
            connectors.push((*level, y + height / 2.0));
            y += height;
        }
        draw_skeleton(&ug.with_color(self.line_color.clone()), &connectors);
    }
}

/// Each item's connector: a short line with a dot, and a line up to its parent or previous sibling.
fn draw_skeleton(ug: &UGraphic, connectors: &[(usize, f64)]) {
    for (index, &(level, y)) in connectors.iter().enumerate() {
        let x = level_start(level);
        ug.translated(x + LEVEL_WIDTH - 1.0, y - 1.0)
            .draw(&UShape::Rectangle(URectangle::new(2.0, 2.0)));
        ug.translated(x, y).draw(&UShape::Line {
            dx: LEVEL_WIDTH,
            dy: 0.0,
        });
        let above = connectors[..index]
            .iter()
            .rev()
            .find(|(other, _)| *other == level || *other + 1 == level)
            .map_or(0.0, |&(_, other_y)| other_y);
        ug.translated(x, above).draw(&UShape::Line {
            dx: 0.0,
            dy: y - above,
        });
    }
}

impl Atom for AtomTree {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indentation_gives_the_level() {
        assert_eq!(level_of("|_ a"), 1);
        assert_eq!(level_of("  |_ a"), 2);
        assert_eq!(level_of("\t  |_ a"), 3);
        assert_eq!(without_marker("  |_ a"), " a");
    }
}

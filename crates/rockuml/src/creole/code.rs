//! `<code>` … `</code>` blocks: the lines between, verbatim and monospaced (PlantUML's `StripeCode`).

use super::Atom;
use crate::klimt::TextBlock;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UShape, UText};
use crate::klimt::ugraphic::UGraphic;

pub(super) fn is_code_start(line: &str) -> bool {
    line == "<code>"
}

pub(super) struct AtomCode {
    font: FontConfiguration,
    lines: Vec<String>,
    terminated: bool,
}

impl AtomCode {
    pub(super) fn new(font: &FontConfiguration) -> Self {
        Self {
            font: font.with_family("monospaced"),
            lines: Vec::new(),
            terminated: false,
        }
    }

    pub(super) fn is_terminated(&self) -> bool {
        self.terminated
    }

    /// Takes a line of the block, or the `</code>` that ends it.
    pub(super) fn add_line(&mut self, line: &str) {
        if line == "</code>" {
            self.terminated = true;
        } else {
            self.lines.push(line.to_owned());
        }
    }
}

impl TextBlock for AtomCode {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let font = self.font.font();
        self.lines
            .iter()
            .fold(XDimension2D::default(), |total, line| {
                let dimension = string_bounder.calculate_dimension(&font, line);
                XDimension2D::new(
                    total.width.max(dimension.width),
                    total.height + dimension.height,
                )
            })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let font = self.font.font();
        let mut y = 0.0;
        for line in &self.lines {
            y += string_bounder.calculate_dimension(&font, line).height;
            let baseline = y - string_bounder.descent(&font, line);
            ug.translated(0.0, baseline)
                .draw(&UShape::Text(UText::new(line, self.font.clone())));
        }
    }
}

impl Atom for AtomCode {
    fn starting_altitude(&self, _string_bounder: &dyn StringBounder) -> f64 {
        0.0
    }
}

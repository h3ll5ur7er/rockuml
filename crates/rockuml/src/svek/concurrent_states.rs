//! The concurrent regions of a state side by side or one above the other, with dashed separators between them
//! (PlantUML's `ConcurrentStates`).

use super::IEntityImage;
use crate::color::HColor;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::style::{PName, SName, StyleSignature, ValueReading};

/// How far the separator overshoots the regions.
const DASH: f64 = 8.0;
const THICKNESS_BORDER: f64 = 1.5;

/// `||` puts regions side by side with vertical separators, `--` stacks them with horizontal ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Separator {
    Vertical,
    Horizontal,
}

impl Separator {
    /// # Panics
    ///
    /// For a character that separates no regions.
    fn from_char(sep: char) -> Self {
        match sep {
            '|' => Self::Vertical,
            '-' => Self::Horizontal,
            _ => panic!("regions are separated by | or -, not {sep}"),
        }
    }

    fn add(self, orig: XDimension2D, other: XDimension2D) -> XDimension2D {
        match self {
            Self::Vertical => {
                XDimension2D::new(orig.width + other.width, orig.height.max(other.height))
            }
            Self::Horizontal => {
                XDimension2D::new(orig.width.max(other.width), orig.height + other.height)
            }
        }
    }

    fn move_by(self, ug: &UGraphic, dim: XDimension2D) -> UGraphic {
        match self {
            Self::Vertical => ug.translated(dim.width, 0.0),
            Self::Horizontal => ug.translated(0.0, dim.height),
        }
    }

    fn draw_separator(self, ug: &UGraphic, dim_total: XDimension2D) {
        let ug = ug.with_stroke(UStroke {
            dash_visible: DASH,
            dash_space: 10.0,
            thickness: THICKNESS_BORDER,
        });
        match self {
            Self::Vertical => ug.draw(&UShape::Line {
                dx: 0.0,
                dy: dim_total.height + DASH,
            }),
            Self::Horizontal => ug.draw(&UShape::Line {
                dx: dim_total.width + DASH,
                dy: 0.0,
            }),
        }
    }
}

pub(crate) struct ConcurrentStates {
    inners: Vec<Box<dyn IEntityImage>>,
    separator: Separator,
    border_color: HColor,
}

impl ConcurrentStates {
    pub(crate) fn new(
        inners: Vec<Box<dyn IEntityImage>>,
        concurrent_separator: char,
        diagram: &CucaDiagram,
    ) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::StateDiagram,
            SName::State,
        ])
        .get_merged_style(&diagram.skin().current_style_builder());
        Self {
            inners,
            separator: Separator::from_char(concurrent_separator),
            border_color: style.value(PName::LineColor).as_color(),
        }
    }
}

impl TextBlock for ConcurrentStates {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.inners
            .iter()
            .fold(XDimension2D::default(), |result, inner| {
                self.separator
                    .add(result, inner.calculate_dimension(string_bounder))
            })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_total = self.calculate_dimension(string_bounder);
        let mut ug = ug.clone();
        for (i, inner) in self.inners.iter().enumerate() {
            inner.draw_u(&ug);
            ug = self
                .separator
                .move_by(&ug, inner.calculate_dimension(string_bounder));
            if i + 1 < self.inners.len() {
                self.separator
                    .draw_separator(&ug.with_color(self.border_color.clone()), dim_total);
            }
        }
    }
}

impl IEntityImage for ConcurrentStates {}

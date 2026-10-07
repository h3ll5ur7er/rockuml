//! The frame of a group with its title tab, and the dashed line of each `else` (PlantUML's
//! `ComponentRoseGroupingHeader` and `ComponentRoseGroupingElse`, as teoz draws them: the group's
//! background is painted by the tile, not by these components).

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::component::{Area, Component, TextualPart};

/// How far the title tab's bottom right corner is cut.
const CORNER_SIZE: f64 = 10.0;

pub(crate) struct ComponentRoseGroupingHeader {
    text: TextualPart,
    /// `[comment]`, after the title.
    comment: Option<Box<dyn TextBlock>>,
    fashion: Fashion,
    fashion_corner: Fashion,
    round_corner: f64,
}

impl ComponentRoseGroupingHeader {
    pub(crate) fn new(
        text: TextualPart,
        comment: Option<Box<dyn TextBlock>>,
        fashion: Fashion,
        fashion_corner: Fashion,
        round_corner: f64,
    ) -> Self {
        Self {
            text,
            comment,
            fashion,
            fashion_corner,
            round_corner,
        }
    }

    /// A comment taller than one line makes the header taller.
    fn supp_height_for_comment(&self, string_bounder: &dyn StringBounder) -> f64 {
        let Some(comment) = &self.comment else {
            return 0.0;
        };
        let height = comment.calculate_dimension(string_bounder).height;
        if height > 15.0 { height - 15.0 } else { 0.0 }
    }

    fn frame(&self, area: &Area) -> UShape {
        UShape::Rectangle(
            URectangle::new(area.dimension.width, area.dimension.height).rounded(self.round_corner),
        )
    }

    fn with_frame_stroke(&self, ug: &UGraphic) -> UGraphic {
        ug.with_stroke(self.fashion.stroke)
            .with_color(self.fashion.fore_color.clone())
    }

    /// The title tab.
    fn corner(&self, width: f64, height: f64) -> UShape {
        let round = self.round_corner;
        if round == 0.0 {
            return UShape::Path(vec![
                USegment::MoveTo(0.0, 0.0),
                USegment::LineTo(width, 0.0),
                USegment::LineTo(width, height - CORNER_SIZE),
                USegment::LineTo(width - CORNER_SIZE, height),
                USegment::LineTo(0.0, height),
                USegment::LineTo(0.0, 0.0),
            ]);
        }
        UShape::Path(vec![
            USegment::MoveTo(round / 2.0, 0.0),
            USegment::LineTo(width, 0.0),
            USegment::LineTo(width, height - CORNER_SIZE),
            USegment::LineTo(width - CORNER_SIZE, height),
            USegment::LineTo(0.0, height),
            USegment::LineTo(0.0, round / 2.0),
            USegment::arc_to((round / 2.0, 0.0), round / 2.0, true),
        ])
    }
}

impl Component for ComponentRoseGroupingHeader {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        let supp = match &self.comment {
            None => 0.0,
            Some(comment) => {
                self.text.padding().left + comment.calculate_dimension(string_bounder).width
            }
        };
        self.text.text_width(string_bounder) + supp
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + self.supp_height_for_comment(string_bounder)
    }

    fn draw_background_internal(&self, ug: &UGraphic, area: &Area) {
        self.with_frame_stroke(ug)
            .with_backcolor(HColor::NONE)
            .draw(&self.frame(area));
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let string_bounder = ug.string_bounder();
        let text_width = self.text.text_width(string_bounder);
        let text_height = self.text.text_height(string_bounder);
        self.fashion_corner
            .apply(ug)
            .draw(&self.corner(text_width, text_height));
        let ug = self.with_frame_stroke(ug);
        ug.draw(&self.frame(area));
        let ug = ug.with_stroke(UStroke::SIMPLE);
        let padding = self.text.padding();
        self.text
            .text_block()
            .draw_u(&ug.translated(padding.left, padding.top));
        if let Some(comment) = &self.comment {
            comment.draw_u(&ug.translated(padding.left + text_width, padding.top + 1.0));
        }
    }
}

/// The dashed line across a group where an `else` starts, with its `[comment]`.
pub(crate) struct ComponentRoseGroupingElse {
    text: TextualPart,
    line_color: HColor,
}

impl ComponentRoseGroupingElse {
    pub(crate) fn new(text: TextualPart, line_color: HColor) -> Self {
        Self { text, line_color }
    }
}

impl Component for ComponentRoseGroupingElse {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width(string_bounder)
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + 4.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let ug = ug
            .with_stroke(UStroke {
                dash_visible: 2.0,
                dash_space: 2.0,
                thickness: 1.0,
            })
            .with_color(self.line_color.clone());
        ug.translated(0.0, 1.0).draw(&UShape::Line {
            dx: area.dimension.width,
            dy: 0.0,
        });
        let padding = self.text.padding();
        self.text.text_block().draw_u(
            &ug.with_stroke(UStroke::SIMPLE)
                .translated(padding.left, padding.top + 2.0),
        );
    }
}

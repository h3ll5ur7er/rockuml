//! Notes: with a folded corner, as a box (`rnote`) or as a hexagon (`hnote`) (PlantUML's
//! `ComponentRoseNote`, `ComponentRoseNoteBox` and `ComponentRoseNoteHexagonal`).

use std::rc::Rc;

use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::stencil::Stencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::component::{Area, Component, TextualPart};
use crate::svek::image::{get_corner, get_polygon_normal};

/// How far the hexagon's points stick out.
const CORNER_SIZE: f64 = 10.0;

/// Where a note's text goes when the note is wider than its text.
fn text_x(
    position: HorizontalAlignment,
    left_padding: f64,
    area_width: f64,
    text_width: f64,
    diff_x: f64,
) -> f64 {
    match position {
        HorizontalAlignment::Left => left_padding,
        HorizontalAlignment::Right => area_width - text_width,
        HorizontalAlignment::Center => left_padding + diff_x / 2.0,
    }
}

/// A note with a folded corner; separators in its text span the note.
pub(crate) struct ComponentRoseNote {
    text: TextualPart,
    fashion: Fashion,
    position: HorizontalAlignment,
}

impl ComponentRoseNote {
    const PADDING_X: f64 = 5.0;
    const PADDING_Y: f64 = 5.0;

    pub(crate) fn new(text: TextualPart, fashion: Fashion, position: HorizontalAlignment) -> Self {
        Self {
            text,
            fashion,
            position,
        }
    }
}

/// Separators in a note's text go from its left edge to the end of its text.
struct NoteStencil {
    text_width: f64,
}

impl Stencil for NoteStencil {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        0.0
    }

    fn ending_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        self.text_width
    }
}

impl Component for ComponentRoseNote {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width(string_bounder) + 2.0 * Self::PADDING_X
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + 2.0 * Self::PADDING_Y
    }

    fn padding_x(&self) -> f64 {
        Self::PADDING_X
    }

    fn padding_y(&self) -> f64 {
        Self::PADDING_Y
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let string_bounder = ug.string_bounder();
        let text_height = self.text.text_height(string_bounder).trunc();
        let text_width = self.text.text_width(string_bounder);
        let mut x2 = text_width.trunc();
        let preferred_width = self.preferred_width(string_bounder);
        let diff_x = area.dimension.width - preferred_width;
        assert!(
            diff_x >= 0.0,
            "a note is drawn at least as wide as it wants"
        );
        if area.dimension.width > preferred_width {
            x2 = (area.dimension.width - 2.0 * Self::PADDING_X).trunc();
        }
        let ug = self.fashion.apply(ug);
        let round_corner = self.fashion.round_corner;
        ug.draw(&UShape::Path(get_polygon_normal(
            x2,
            text_height,
            round_corner,
        )));
        ug.draw(&UShape::Path(get_corner(x2, round_corner)));
        let ug = ug.with_stencil(Rc::new(NoteStencil { text_width }));
        let padding = self.text.padding();
        let x = text_x(
            self.position,
            padding.left,
            area.dimension.width,
            text_width,
            diff_x,
        );
        self.text
            .text_block()
            .draw_u(&ug.translated(x, padding.top));
    }
}

/// `rnote`: a plain box.
pub(crate) struct ComponentRoseNoteBox {
    text: TextualPart,
    fashion: Fashion,
}

impl ComponentRoseNoteBox {
    const PADDING: f64 = 5.0;

    pub(crate) fn new(text: TextualPart, fashion: Fashion) -> Self {
        Self { text, fashion }
    }
}

impl Component for ComponentRoseNoteBox {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width(string_bounder) + 2.0 * Self::PADDING
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + 2.0 * Self::PADDING
    }

    fn padding_x(&self) -> f64 {
        Self::PADDING
    }

    fn padding_y(&self) -> f64 {
        Self::PADDING
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let string_bounder = ug.string_bounder();
        let text_height = self.text.text_height(string_bounder).trunc();
        let mut x2 = self.text.text_width(string_bounder).trunc();
        let preferred_width = self.preferred_width(string_bounder);
        let diff_x = area.dimension.width - preferred_width;
        assert!(
            diff_x >= 0.0,
            "a note is drawn at least as wide as it wants"
        );
        if area.dimension.width > preferred_width {
            x2 = (area.dimension.width - 2.0 * Self::PADDING).trunc();
        }
        let ug = self.fashion.apply(ug);
        ug.draw(&UShape::Rectangle(
            URectangle::new(x2, text_height).rounded(self.fashion.round_corner),
        ));
        let padding = self.text.padding();
        self.text.text_block().draw_u(
            &ug.with_stroke(UStroke::SIMPLE)
                .translated(padding.left + diff_x / 2.0, padding.top),
        );
    }
}

/// `hnote`: a hexagon.
pub(crate) struct ComponentRoseNoteHexagonal {
    text: TextualPart,
    fashion: Fashion,
}

impl ComponentRoseNoteHexagonal {
    const PADDING: f64 = 5.0;

    pub(crate) fn new(text: TextualPart, fashion: Fashion) -> Self {
        Self { text, fashion }
    }
}

impl Component for ComponentRoseNoteHexagonal {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width(string_bounder) + 2.0 * Self::PADDING
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + 2.0 * Self::PADDING
    }

    fn padding_x(&self) -> f64 {
        Self::PADDING
    }

    fn padding_y(&self) -> f64 {
        Self::PADDING
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let string_bounder = ug.string_bounder();
        let text_height = self.text.text_height(string_bounder).trunc();
        let mut x2 = self.text.text_width(string_bounder).trunc();
        let preferred_width = self.preferred_width(string_bounder);
        let diff_x = area.dimension.width - preferred_width;
        assert!(
            diff_x >= 0.0,
            "a note is drawn at least as wide as it wants"
        );
        if area.dimension.width > preferred_width {
            x2 = (area.dimension.width - 2.0 * Self::PADDING).trunc();
        }
        // Java divides the whole-number height by 2 in integers.
        let middle = (text_height / 2.0).trunc();
        let polygon = vec![
            (CORNER_SIZE, 0.0),
            (x2 - CORNER_SIZE, 0.0),
            (x2, middle),
            (x2 - CORNER_SIZE, text_height),
            (CORNER_SIZE, text_height),
            (0.0, middle),
            (CORNER_SIZE, 0.0),
        ];
        let ug = self.fashion.apply(ug);
        ug.draw(&UShape::Polygon(polygon));
        let padding = self.text.padding();
        self.text.text_block().draw_u(
            &ug.with_stroke(UStroke::SIMPLE)
                .translated(padding.left + diff_x / 2.0, padding.top),
        );
    }
}

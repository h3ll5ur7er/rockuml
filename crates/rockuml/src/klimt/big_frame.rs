//! A frame around a whole diagram, its title in a tab in the top left corner (PlantUML's `BigFrame`).

use std::rc::Rc;

use super::TextBlock;
use super::fashion::Fashion;
use super::font::StringBounder;
use super::geom::{ClockwiseTopRightBottomLeft, MinMax, XDimension2D};
use super::limit_finder::LimitFinder;
use super::shape::{URectangle, USegment, UShape};
use super::ugraphic::UGraphic;
use crate::color::HColor;

pub(crate) struct BigFrame<'a> {
    pub title: &'a dyn TextBlock,
    pub original: &'a dyn TextBlock,
    pub padding: ClockwiseTopRightBottomLeft,
    pub symbol_context: &'a Fashion,
    /// Measures how far the original reaches, which needs a bounder of its own.
    pub string_bounder: Rc<dyn StringBounder>,
}

impl BigFrame<'_> {
    fn y_pos(dim_title: XDimension2D) -> f64 {
        if dim_title.width == 0.0 {
            12.0
        } else {
            dim_title.height + 3.0
        }
    }

    fn effective_padding(&self, string_bounder: &dyn StringBounder) -> ClockwiseTopRightBottomLeft {
        let dim_title = self.title.calculate_dimension(string_bounder);
        self.padding.inc_top(dim_title.height + 10.0)
    }

    fn original_min_max(&self) -> MinMax {
        LimitFinder::min_max_of(self.original, self.string_bounder.clone())
    }

    fn compute_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dim_title = self.title.calculate_dimension(string_bounder);
        let effective_padding = self.effective_padding(string_bounder);
        let min_max = self.original_min_max();
        let ww = if min_max.min_x() >= 0.0 {
            min_max.max_x()
        } else {
            min_max.dimension().width
        };
        effective_padding.left + (ww + 12.0).max(dim_title.width + 10.0) + effective_padding.right
    }

    fn compute_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dim_title = self.title.calculate_dimension(string_bounder);
        let effective_padding = self.effective_padding(string_bounder);
        let min_max = self.original_min_max();
        let hh = if min_max.min_y() >= 0.0 {
            min_max.max_y()
        } else {
            min_max.dimension().height
        };
        effective_padding.top + dim_title.height + hh + effective_padding.bottom
    }
}

impl TextBlock for BigFrame<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.compute_width(string_bounder),
            self.compute_height(string_bounder),
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim = self.calculate_dimension(string_bounder);
        let ug = self.symbol_context.apply(ug);
        let dim_title = self.title.calculate_dimension(string_bounder);
        let rectangle =
            URectangle::new(dim.width, dim.height).rounded(self.symbol_context.round_corner);
        ug.draw(&UShape::Rectangle(rectangle));

        let (text_width, corner_size) = if dim_title.width == 0.0 {
            (dim.width / 3.0, 7.0)
        } else {
            (dim_title.width + 10.0, 10.0)
        };
        let text_height = Self::y_pos(dim_title);
        let tab = vec![
            USegment::MoveTo(text_width, 0.0),
            USegment::LineTo(text_width, text_height - corner_size),
            USegment::LineTo(text_width - corner_size, text_height),
            USegment::LineTo(0.0, text_height),
        ];
        ug.with_backcolor(HColor::NONE).draw(&UShape::Path(tab));
        let ug_title = ug.translated(3.0, 1.0);
        if dim.width - dim_title.width < 25.0 {
            self.title.draw_u(&ug_title);
        } else {
            ug_title.draw_special_text(self.title);
        }
    }
}

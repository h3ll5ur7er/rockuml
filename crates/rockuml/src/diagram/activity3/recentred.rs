//! The compressed drawing moved to start 5 from the origin, with 15 to spare right and below (PlantUML's
//! `activitydiagram3.Recentred`).

use std::cell::OnceCell;

use crate::klimt::TextBlock;
use crate::klimt::compress::CompressionXorYBuilder;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{MinMax, UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct Recentred<T> {
    text_block: CompressionXorYBuilder<T>,
    min_max: OnceCell<MinMax>,
}

impl<T: TextBlock> Recentred<T> {
    pub(super) fn new(text_block: CompressionXorYBuilder<T>) -> Self {
        Self {
            text_block,
            min_max: OnceCell::new(),
        }
    }

    fn get_min_max(&self, string_bounder: &dyn StringBounder) -> MinMax {
        *self.min_max.get_or_init(|| {
            self.text_block
                .get_min_max(string_bounder)
                .enlarge(15.0, 15.0)
        })
    }
}

impl<T: TextBlock> TextBlock for Recentred<T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.get_min_max(string_bounder).dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let min_max = self.get_min_max(ug.string_bounder());
        self.text_block.draw_u(&ug.apply(UTranslate::new(
            -min_max.min_x() + 5.0,
            -min_max.min_y() + 5.0,
        )));
    }
}

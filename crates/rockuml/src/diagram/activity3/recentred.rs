//! The drawing moved to start 5 from the origin, with 15 to spare right and below (PlantUML's
//! `activitydiagram3.Recentred`).

use std::cell::OnceCell;
use std::rc::Rc;

use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{MinMax, UTranslate, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::ugraphic::UGraphic;

pub(super) struct Recentred<'a> {
    text_block: Box<dyn TextBlock + 'a>,
    /// Measured with `string_bounder`, which measures the drawing as PlantUML's `getMinMax` does.
    string_bounder: Rc<dyn StringBounder>,
    min_max: OnceCell<MinMax>,
}

impl<'a> Recentred<'a> {
    pub(super) fn new(
        text_block: Box<dyn TextBlock + 'a>,
        string_bounder: Rc<dyn StringBounder>,
    ) -> Self {
        Self {
            text_block,
            string_bounder,
            min_max: OnceCell::new(),
        }
    }

    fn get_min_max(&self) -> MinMax {
        *self.min_max.get_or_init(|| {
            LimitFinder::min_max_of(self.text_block.as_ref(), self.string_bounder.clone())
                .enlarge(15.0, 15.0)
        })
    }
}

impl TextBlock for Recentred<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.get_min_max().dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let min_max = self.get_min_max();
        self.text_block.draw_u(&ug.apply(UTranslate::new(
            -min_max.min_x() + 5.0,
            -min_max.min_y() + 5.0,
        )));
    }
}

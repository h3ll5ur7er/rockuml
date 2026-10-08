use std::cell::OnceCell;
use std::rc::Rc;

use super::CompressionMode;
use super::compression_transform::CompressionTransform;
use super::slot_finder::SlotFinder;
use super::ugraphic_compress_on_x_or_y::UGraphicCompressOnXorY;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{MinMax, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::ugraphic::UGraphic;

/// Free space narrower than this is kept: it separates what lies on either side.
const MARGIN: f64 = 5.0;

/// `text_block` with its empty space squeezed out along one axis (`CompressionXorYBuilder`). Activity
/// diagrams stack two: across, then down. The squeeze is found on the first measure or drawing, and kept.
pub(crate) struct CompressionXorYBuilder<T> {
    mode: CompressionMode,
    text_block: T,
    cached_affine: OnceCell<Rc<CompressionTransform>>,
    cached_min_max: OnceCell<MinMax>,
}

impl<T: TextBlock> CompressionXorYBuilder<T> {
    pub(crate) fn build(mode: CompressionMode, text_block: T) -> Self {
        Self {
            mode,
            text_block,
            cached_affine: OnceCell::new(),
            cached_min_max: OnceCell::new(),
        }
    }

    fn get_affine_transform(&self, string_bounder: &dyn StringBounder) -> Rc<CompressionTransform> {
        self.cached_affine
            .get_or_init(|| {
                let (ug, slot_finder) = SlotFinder::create(self.mode, string_bounder.shared());
                self.text_block.draw_u(&ug);
                let slot_set = slot_finder
                    .borrow()
                    .get_slot_set()
                    .reverse()
                    .smaller(MARGIN);
                Rc::new(CompressionTransform::new(&slot_set))
            })
            .clone()
    }

    /// How far the squeezed drawing reaches, the origin not counting (`getMinMax`).
    pub(crate) fn get_min_max(&self, string_bounder: &dyn StringBounder) -> MinMax {
        *self
            .cached_min_max
            .get_or_init(|| LimitFinder::min_max_of(self, string_bounder.shared()))
    }
}

impl<T: TextBlock> TextBlock for CompressionXorYBuilder<T> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let affine = self.get_affine_transform(string_bounder);
        let dim = self.text_block.calculate_dimension(string_bounder);
        match self.mode {
            CompressionMode::OnX => XDimension2D::new(affine.transform(dim.width), dim.height),
            CompressionMode::OnY => XDimension2D::new(dim.width, affine.transform(dim.height)),
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        let affine = self.get_affine_transform(ug.string_bounder());
        self.text_block.draw_u(&UGraphicCompressOnXorY::create(
            self.mode,
            ug.clone(),
            affine,
        ));
    }
}

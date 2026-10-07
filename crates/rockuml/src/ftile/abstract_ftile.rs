//! What most tiles share (PlantUML's `AbstractFtile`): the skin they are drawn with and their geometry,
//! computed once.

use std::cell::OnceCell;
use std::rc::Rc;

use super::FtileGeometry;
use crate::skin::SkinParam;

/// Embedded in a tile, which forwards [`super::Ftile::skin_param`] and
/// [`super::Ftile::calculate_dimension`] to it:
///
/// ```ignore
/// fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
///     self.base.calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
/// }
/// ```
pub(crate) struct AbstractFtile {
    skin_param: Rc<SkinParam>,
    /// Computed with the first string bounder asked, as in PlantUML: a tile is measured with one only.
    cached_geometry: OnceCell<FtileGeometry>,
}

impl AbstractFtile {
    pub(crate) fn new(skin_param: Rc<SkinParam>) -> Self {
        Self {
            skin_param,
            cached_geometry: OnceCell::new(),
        }
    }

    pub(crate) fn skin_param(&self) -> &SkinParam {
        &self.skin_param
    }

    /// The geometry `calculate_dimension_ftile` computes, once (`calculateDimension`).
    pub(crate) fn calculate_dimension(
        &self,
        calculate_dimension_ftile: impl FnOnce() -> FtileGeometry,
    ) -> FtileGeometry {
        *self.cached_geometry.get_or_init(calculate_dimension_ftile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn the_geometry_is_computed_once() {
        let base = AbstractFtile::new(Rc::new(SkinParam::default()));
        let computed = Cell::new(0);
        let compute = || {
            computed.set(computed.get() + 1);
            FtileGeometry::new(10.0, 20.0, 5.0, 0.0)
        };
        assert_eq!(base.calculate_dimension(compute).get_width(), 10.0);
        assert_eq!(base.calculate_dimension(compute).get_height(), 20.0);
        assert_eq!(computed.get(), 1);
    }
}

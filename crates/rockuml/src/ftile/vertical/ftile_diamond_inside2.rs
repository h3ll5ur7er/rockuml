//! A condition written inside a hexagon, with the label of the arrow out of it above (PlantUML's
//! `FtileDiamondInside2`): the conditions of an `if` with `elseif`s.

use std::rc::Rc;

use super::ftile_diamond_inside::{draw_hexagon, hexagon_dimension};
use super::ftile_diamond_wip::{FtileDiamondWIP, swimable_through_wip};
use crate::color::HColor;
use crate::diagram::activity3::SwimlaneId;
use crate::ftile::{Ftile, FtileGeometry};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileDiamondInside2 {
    wip: FtileDiamondWIP,
    label: Rc<dyn TextBlock>,
}

impl FtileDiamondInside2 {
    pub(crate) fn new(
        label: Rc<dyn TextBlock>,
        skin_param: Rc<SkinParam>,
        back_color: HColor,
        border_color: HColor,
        swimlane: Option<SwimlaneId>,
    ) -> Self {
        Self {
            wip: FtileDiamondWIP::new(skin_param, back_color, border_color, swimlane),
            label,
        }
    }

    #[must_use]
    pub(crate) fn with_north(mut self, north: Rc<dyn TextBlock>) -> Self {
        self.wip.north = north;
        self
    }

    #[must_use]
    pub(crate) fn with_west(mut self, west: Rc<dyn TextBlock>) -> Self {
        self.wip.set_west(west);
        self
    }

    #[must_use]
    pub(crate) fn with_east(mut self, east: Rc<dyn TextBlock>) -> Self {
        self.wip.set_east(east);
        self
    }

    fn calculate_dimension_alone(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        hexagon_dimension(self.label.calculate_dimension(string_bounder))
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let diamond = self.calculate_dimension_alone(string_bounder);
        let north = self.wip.north.calculate_dimension(string_bounder);
        let height = diamond.height + north.height;
        let left = diamond.width / 2.0;
        let width = if north.width > left {
            left + north.width
        } else {
            diamond.width
        };
        FtileGeometry::with_out(width, height, left, 0.0, diamond.height)
    }
}

swimable_through_wip!(FtileDiamondInside2);

impl Ftile for FtileDiamondInside2 {
    fn skin_param(&self) -> &SkinParam {
        self.wip.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.wip
            .base
            .calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_total = self.calculate_dimension_alone(ug.string_bounder());
        draw_hexagon(&self.wip, self.label.as_ref(), dim_total, ug);
    }
}

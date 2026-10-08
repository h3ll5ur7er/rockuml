//! A condition written inside a hexagon (PlantUML's `FtileDiamondInside`), the default
//! `skinparam conditionStyle`, and the diamonds of a `switch`.

use std::rc::Rc;

use super::ftile_diamond_wip::{FtileDiamondWIP, swimable_through_wip};
use crate::color::HColor;
use crate::diagram::activity3::SwimlaneId;
use crate::ftile::hexagon::{self, HEXAGON_HALF_SIZE};
use crate::ftile::{Ftile, FtileGeometry};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileDiamondInside {
    wip: FtileDiamondWIP,
    label: Rc<dyn TextBlock>,
}

impl FtileDiamondInside {
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
    pub(crate) fn with_south(mut self, south: Rc<dyn TextBlock>) -> Self {
        self.wip.south = south;
        self
    }

    #[must_use]
    pub(crate) fn with_east(mut self, east: Rc<dyn TextBlock>) -> Self {
        self.wip.set_east(east);
        self
    }

    #[must_use]
    pub(crate) fn with_west_and_east(
        mut self,
        west: Rc<dyn TextBlock>,
        east: Rc<dyn TextBlock>,
    ) -> Self {
        self.wip.set_west(west);
        self.with_east(east)
    }

    /// `swapEastWest`.
    pub(crate) fn swap_east_west(&self) {
        self.wip.swap_east_west();
    }

    pub(crate) fn get_east_label_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.wip.east().calculate_dimension(string_bounder).width
    }

    pub(crate) fn get_south_label_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.wip.south.calculate_dimension(string_bounder).height
    }

    fn calculate_dimension_alone(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let dim = hexagon_dimension(self.label.calculate_dimension(string_bounder));
        FtileGeometry::from_dim_with_out(dim, dim.width / 2.0, 0.0, dim.height)
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let north_height = self.wip.north.calculate_dimension(string_bounder).height;
        self.calculate_dimension_alone(string_bounder)
            .inc_height(north_height)
    }
}

/// The hexagon around a label: as wide as the label and its two points, at least as high as an empty
/// diamond; an empty diamond around no label.
pub(super) fn hexagon_dimension(dim_label: XDimension2D) -> XDimension2D {
    if dim_label.width == 0.0 || dim_label.height == 0.0 {
        XDimension2D::new(HEXAGON_HALF_SIZE * 2.0, HEXAGON_HALF_SIZE * 2.0)
    } else {
        dim_label
            .at_least(HEXAGON_HALF_SIZE * 2.0, HEXAGON_HALF_SIZE * 2.0)
            .delta(HEXAGON_HALF_SIZE * 2.0, 0.0)
    }
}

/// Draws a hexagon `dim_total` large with the labels inside and around it, as `FtileDiamondInside` and
/// `FtileDiamondInside2` do.
pub(super) fn draw_hexagon(
    wip: &FtileDiamondWIP,
    label: &dyn TextBlock,
    dim_total: XDimension2D,
    ug: &UGraphic,
) {
    let string_bounder = ug.string_bounder();
    let dim_label = label.calculate_dimension(string_bounder);
    let ug = wip.styled(ug);
    ug.draw(&hexagon::as_polygon_sized(
        dim_total.width,
        dim_total.height,
    ));
    wip.north.draw_u(&ug.apply(UTranslate::new(
        4.0 + dim_total.width / 2.0,
        dim_total.height,
    )));
    wip.south.draw_u(&ug.apply(UTranslate::new(
        4.0 + dim_total.width / 2.0,
        dim_total.height,
    )));
    let lx = (dim_total.width - dim_label.width) / 2.0;
    let ly = (dim_total.height - dim_label.height) / 2.0;
    label.draw_u(&ug.apply(UTranslate::new(lx, ly)));
    let dim_west = wip.west().calculate_dimension(string_bounder);
    wip.west().draw_u(&ug.apply(UTranslate::new(
        -dim_west.width,
        -dim_west.height + dim_total.height / 2.0,
    )));
    let dim_east = wip.east().calculate_dimension(string_bounder);
    wip.east().draw_u(&ug.apply(UTranslate::new(
        dim_total.width,
        -dim_east.height + dim_total.height / 2.0,
    )));
}

swimable_through_wip!(FtileDiamondInside);

impl Ftile for FtileDiamondInside {
    fn skin_param(&self) -> &SkinParam {
        self.wip.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.wip
            .base
            .calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_total = self
            .calculate_dimension_alone(ug.string_bounder())
            .dimension();
        draw_hexagon(&self.wip, self.label.as_ref(), dim_total, ug);
    }
}

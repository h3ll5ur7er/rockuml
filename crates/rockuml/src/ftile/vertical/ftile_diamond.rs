//! An empty diamond with labels around it (PlantUML's `FtileDiamond`): the end of an `if`, or its start
//! with `skinparam conditionStyle diamond`.

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

pub(crate) struct FtileDiamond {
    wip: FtileDiamondWIP,
}

impl FtileDiamond {
    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        back_color: HColor,
        border_color: HColor,
        swimlane: Option<SwimlaneId>,
    ) -> Self {
        Self {
            wip: FtileDiamondWIP::new(skin_param, back_color, border_color, swimlane),
        }
    }

    #[must_use]
    pub(crate) fn with_north(mut self, north: Rc<dyn TextBlock>) -> Self {
        self.wip.north = north;
        self
    }

    #[must_use]
    pub(crate) fn with_south(mut self, south: Rc<dyn TextBlock>) -> Self {
        self.wip.south = south;
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

    #[must_use]
    pub(crate) fn with_west_and_east(
        self,
        west: Rc<dyn TextBlock>,
        east: Rc<dyn TextBlock>,
    ) -> Self {
        self.with_west(west).with_east(east)
    }

    /// The height of the higher of the labels on the sides.
    pub(crate) fn get_west_east_label_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dim_east = self.wip.east().calculate_dimension(string_bounder);
        let dim_west = self.wip.west().calculate_dimension(string_bounder);
        dim_east.height.max(dim_west.height)
    }

    pub(crate) fn get_east_label_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.wip.east().calculate_dimension(string_bounder).width
    }

    pub(crate) fn get_south_label_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.wip.south.calculate_dimension(string_bounder).height
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let supp_y1 = self.wip.north.calculate_dimension(string_bounder).height;
        let dim = XDimension2D::new(HEXAGON_HALF_SIZE * 2.0, HEXAGON_HALF_SIZE * 2.0 + supp_y1);
        FtileGeometry::from_dim_with_out(dim, dim.width / 2.0, supp_y1, dim.height)
    }
}

swimable_through_wip!(FtileDiamond);

impl Ftile for FtileDiamond {
    fn skin_param(&self) -> &SkinParam {
        self.wip.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.wip
            .base
            .calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let supp_y1 = self.wip.north.calculate_dimension(string_bounder).height;
        let ug = ug.apply(UTranslate::new(0.0, supp_y1));
        self.wip.draw_outline(&ug, &hexagon::as_polygon());
        self.wip
            .north
            .draw_u(&ug.apply(UTranslate::new(HEXAGON_HALF_SIZE * 1.5, -supp_y1)));
        self.wip.south.draw_u(&ug.apply(UTranslate::new(
            HEXAGON_HALF_SIZE * 1.5,
            2.0 * HEXAGON_HALF_SIZE,
        )));
        let dim_west = self.wip.west().calculate_dimension(string_bounder);
        self.wip.west().draw_u(&ug.apply(UTranslate::new(
            -dim_west.width,
            -dim_west.height + HEXAGON_HALF_SIZE,
        )));
        let dim_east = self.wip.east().calculate_dimension(string_bounder);
        self.wip.east().draw_u(&ug.apply(UTranslate::new(
            HEXAGON_HALF_SIZE * 2.0,
            -dim_east.height + HEXAGON_HALF_SIZE,
        )));
    }
}

//! A condition written inside a diamond (PlantUML's `FtileDiamondSquare`), with
//! `skinparam conditionStyle InsideDiamond`.

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

pub(crate) struct FtileDiamondSquare {
    wip: FtileDiamondWIP,
    label: Rc<dyn TextBlock>,
}

impl FtileDiamondSquare {
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
        self,
        west: Rc<dyn TextBlock>,
        east: Rc<dyn TextBlock>,
    ) -> Self {
        self.with_west(west).with_east(east)
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_label = self.label.calculate_dimension(string_bounder);
        if dim_label.width == 0.0 || dim_label.height == 0.0 {
            return XDimension2D::new(HEXAGON_HALF_SIZE * 2.0, HEXAGON_HALF_SIZE * 2.0);
        }
        dim_label.delta(HEXAGON_HALF_SIZE * 2.0, HEXAGON_HALF_SIZE * 2.0)
    }
}

swimable_through_wip!(FtileDiamondSquare);

impl Ftile for FtileDiamondSquare {
    fn skin_param(&self) -> &SkinParam {
        self.wip.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.wip.base.calculate_dimension(|| {
            let dim = self.calculate_dimension_internal(string_bounder);
            FtileGeometry::from_dim_with_out(dim, dim.width / 2.0, 0.0, dim.height)
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_label = self.label.calculate_dimension(string_bounder);
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let wip = &self.wip;
        let ug = wip.styled(ug);
        ug.draw(&hexagon::as_polygon_square(
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
        self.label.draw_u(&ug.apply(UTranslate::new(lx, ly)));
        let dim_west = wip.west().calculate_dimension(string_bounder);
        wip.west().draw_u(&ug.apply(UTranslate::new(
            -dim_west.width,
            -dim_west.height + HEXAGON_HALF_SIZE,
        )));
        let dim_east = wip.east().calculate_dimension(string_bounder);
        wip.east().draw_u(&ug.apply(UTranslate::new(
            dim_total.width,
            -dim_east.height + HEXAGON_HALF_SIZE + 5.0,
        )));
    }
}

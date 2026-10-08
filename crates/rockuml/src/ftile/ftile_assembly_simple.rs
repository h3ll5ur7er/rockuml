//! One tile above another, their entry points aligned (PlantUML's `FtileAssemblySimple`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::{Ftile, FtileGeometry, Swimable, same};
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileAssemblySimple {
    tile1: Rc<dyn Ftile>,
    tile2: Rc<dyn Ftile>,
    calculate_dimension: OnceCell<FtileGeometry>,
}

impl FtileAssemblySimple {
    pub(crate) fn new(tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Self {
        Self {
            tile1,
            tile2,
            calculate_dimension: OnceCell::new(),
        }
    }

    fn get_translated1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let left = self.calculate_dimension(string_bounder).get_left();
        UTranslate::new(
            left - self.tile1.calculate_dimension(string_bounder).get_left(),
            0.0,
        )
    }

    fn get_translated2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim1 = self.tile1.calculate_dimension(string_bounder);
        let left = self.calculate_dimension(string_bounder).get_left();
        UTranslate::new(
            left - self.tile2.calculate_dimension(string_bounder).get_left(),
            dim1.get_height(),
        )
    }
}

impl Swimable for FtileAssemblySimple {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = self.tile1.get_swimlanes();
        result.extend(self.tile2.get_swimlanes());
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.tile1.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.tile2.get_swimlane_out()
    }
}

impl Ftile for FtileAssemblySimple {
    fn skin_param(&self) -> &SkinParam {
        self.tile1.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.tile1.get_in_link_rendering()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self.calculate_dimension.get_or_init(|| {
            self.tile1
                .calculate_dimension(string_bounder)
                .append_bottom(self.tile2.calculate_dimension(string_bounder))
        })
    }

    /// A tile below the first is found in it, as PlantUML finds it: its tiles fail rather than answer
    /// that they do not hold the child.
    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.tile1.as_ref()) {
            return self.get_translated1(string_bounder);
        }
        if same(child, self.tile2.as_ref()) {
            return self.get_translated2(string_bounder);
        }
        self.tile1
            .get_translate_for(child, string_bounder)
            .compose(self.get_translated1(string_bounder))
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile1.clone(), self.tile2.clone()]
    }

    fn get_welding_points(&self) -> Vec<Rc<dyn Ftile>> {
        let mut result = self.tile1.get_welding_points();
        result.extend(self.tile2.get_welding_points());
        result
    }

    fn arrow_horizontal_alignment(&self) -> HorizontalAlignment {
        self.tile1.arrow_horizontal_alignment()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        ug.apply(self.get_translate_for(self.tile1.as_ref(), string_bounder))
            .draw(&self.tile1);
        ug.apply(self.get_translate_for(self.tile2.as_ref(), string_bounder))
            .draw(&self.tile2);
    }
}

//! A tile with room left and right of it (PlantUML's `FtileMarged`).

use std::rc::Rc;

use super::{AbstractFtile, Ftile, FtileGeometry, Swimable, same};
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileMarged {
    base: AbstractFtile,
    tile: Rc<dyn Ftile>,
    margin1: f64,
    margin2: f64,
}

impl FtileMarged {
    /// `margin1` left of `tile` and `margin2` right of it; `skin_param` is the one `tile` is drawn with.
    pub(crate) fn new(
        tile: Rc<dyn Ftile>,
        skin_param: Rc<SkinParam>,
        margin1: f64,
        margin2: f64,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            tile,
            margin1,
            margin2,
        }
    }

    fn get_translate(&self) -> UTranslate {
        UTranslate::new(self.margin1, 0.0)
    }
}

impl Swimable for FtileMarged {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.tile.get_swimlanes()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.tile.get_swimlane_out()
    }
}

impl Ftile for FtileMarged {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.tile.get_in_link_rendering()
    }

    fn get_out_link_rendering(&self) -> LinkRendering {
        self.tile.get_out_link_rendering()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let orig = self.tile.calculate_dimension(string_bounder);
            FtileGeometry::with_out(
                orig.get_width() + self.margin1 + self.margin2,
                orig.get_height(),
                orig.get_left() + self.margin1,
                orig.get_in_y(),
                orig.get_out_y(),
            )
        })
    }

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.tile.as_ref()) {
            return self.get_translate();
        }
        self.tile
            .get_translate_for(child, string_bounder)
            .compose(self.get_translate())
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile.clone()]
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.apply(self.get_translate()).draw(&self.tile);
    }
}

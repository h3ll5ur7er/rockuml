//! Tiles wrapping another and changing little of it (PlantUML's abstract `FtileDecorate`), and those that
//! change how arrows reach it (`FtileDecorateIn`, `FtileDecorateOut`, `FtileDecorateInLabel` and
//! `FtileDecorateOutLabel`).

use std::rc::Rc;

use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::ftile::{Ftile, FtileGeometry, Swimable, same};
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

/// A decorator implements [`Self::get_ftile_delegated`] and overrides what it changes; it is an [`Ftile`]
/// through the blanket implementations below. The defaults are PlantUML's: everything is the wrapped
/// tile's, which is drawn as `ftile.drawU(ug)`, not through the layers.
pub(crate) trait FtileDecorate: 'static {
    /// The tile wrapped.
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile>;

    fn get_out_link_rendering(&self) -> LinkRendering {
        self.get_ftile_delegated().get_out_link_rendering()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.get_ftile_delegated().get_in_link_rendering()
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.get_ftile_delegated().draw_u(ug);
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.get_ftile_delegated()
            .calculate_dimension(string_bounder)
    }

    fn get_swimlanes(&self) -> SwimlaneSet {
        self.get_ftile_delegated().get_swimlanes()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.get_ftile_delegated().get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.get_ftile_delegated().get_swimlane_out()
    }

    fn get_welding_points(&self) -> Vec<Rc<dyn Ftile>> {
        self.get_ftile_delegated().get_welding_points()
    }

    /// Where the wrapped tile is drawn: where this one is, whatever the decorator moves it by.
    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let ftile = self.get_ftile_delegated();
        if same(child, ftile.as_ref()) {
            return UTranslate::default();
        }
        ftile.get_translate_for(child, string_bounder)
    }
}

impl<T: FtileDecorate> Swimable for T {
    fn get_swimlanes(&self) -> SwimlaneSet {
        FtileDecorate::get_swimlanes(self)
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        FtileDecorate::get_swimlane_in(self)
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        FtileDecorate::get_swimlane_out(self)
    }
}

impl<T: FtileDecorate> Ftile for T {
    fn skin_param(&self) -> &Rc<SkinParam> {
        self.get_ftile_delegated().skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        FtileDecorate::get_in_link_rendering(self)
    }

    fn get_out_link_rendering(&self) -> LinkRendering {
        FtileDecorate::get_out_link_rendering(self)
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        FtileDecorate::calculate_dimension(self, string_bounder)
    }

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        FtileDecorate::get_translate_for(self, child, string_bounder)
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.get_ftile_delegated().clone()]
    }

    fn get_welding_points(&self) -> Vec<Rc<dyn Ftile>> {
        FtileDecorate::get_welding_points(self)
    }

    fn arrow_horizontal_alignment(&self) -> HorizontalAlignment {
        self.get_ftile_delegated().arrow_horizontal_alignment()
    }

    fn draw_u(&self, ug: &UGraphic) {
        FtileDecorate::draw_u(self, ug);
    }
}

/// The tile, the arrow into it drawn as `link_rendering` says.
pub(crate) struct FtileDecorateIn {
    ftile: Rc<dyn Ftile>,
    link_rendering: LinkRendering,
}

impl FtileDecorateIn {
    pub(crate) fn new(ftile: Rc<dyn Ftile>, link_rendering: LinkRendering) -> Self {
        Self {
            ftile,
            link_rendering,
        }
    }
}

impl FtileDecorate for FtileDecorateIn {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.link_rendering.clone()
    }
}

/// The tile, the arrow out of it drawn as `link_rendering` says.
pub(crate) struct FtileDecorateOut {
    ftile: Rc<dyn Ftile>,
    link_rendering: LinkRendering,
}

impl FtileDecorateOut {
    pub(crate) fn new(ftile: Rc<dyn Ftile>, link_rendering: LinkRendering) -> Self {
        Self {
            ftile,
            link_rendering,
        }
    }
}

impl FtileDecorate for FtileDecorateOut {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn get_out_link_rendering(&self) -> LinkRendering {
        self.link_rendering.clone()
    }
}

/// The tile with room above it for the label of the arrow into it, `dim` large.
pub(crate) struct FtileDecorateInLabel {
    ftile: Rc<dyn Ftile>,
    xl: f64,
    yl: f64,
}

impl FtileDecorateInLabel {
    pub(crate) fn new(ftile: Rc<dyn Ftile>, dim: XDimension2D) -> Self {
        Self {
            ftile,
            xl: dim.width,
            yl: dim.height,
        }
    }
}

impl FtileDecorate for FtileDecorateInLabel {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let result = self
            .ftile
            .calculate_dimension(string_bounder)
            .add_top(self.yl);
        let missing = self.xl - result.get_right();
        if missing > 0.0 {
            return result.inc_right(missing);
        }
        result
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.ftile.draw_u(&ug.apply(UTranslate::new(0.0, self.yl)));
    }
}

/// The tile with room below it for the label of the arrow out of it, `dim` large.
pub(crate) struct FtileDecorateOutLabel {
    ftile: Rc<dyn Ftile>,
    xl: f64,
    yl: f64,
}

impl FtileDecorateOutLabel {
    pub(crate) fn new(ftile: Rc<dyn Ftile>, dim: XDimension2D) -> Self {
        Self {
            ftile,
            xl: dim.width,
            yl: dim.height,
        }
    }
}

impl FtileDecorate for FtileDecorateOutLabel {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let result = self
            .ftile
            .calculate_dimension(string_bounder)
            .add_bottom(self.yl);
        let missing = self.xl - result.get_right();
        if missing > 0.0 {
            return result.inc_right(missing);
        }
        result
    }
}

//! A tile with arrows added to it (PlantUML's `FtileWithConnection`).

use std::rc::Rc;

use super::{Connection, Ftile, FtileGeometry, Swimable, same};
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(crate) struct FtileWithConnection {
    ftile: Rc<dyn Ftile>,
    connections: Vec<Rc<dyn Connection>>,
}

impl FtileWithConnection {
    pub(crate) fn new(ftile: Rc<dyn Ftile>, connections: Vec<Rc<dyn Connection>>) -> Self {
        Self { ftile, connections }
    }
}

impl Swimable for FtileWithConnection {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.ftile.get_swimlanes()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.ftile.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.ftile.get_swimlane_out()
    }
}

impl Ftile for FtileWithConnection {
    fn skin_param(&self) -> &SkinParam {
        self.ftile.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.ftile.get_in_link_rendering()
    }

    fn get_out_link_rendering(&self) -> LinkRendering {
        self.ftile.get_out_link_rendering()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.ftile.calculate_dimension(string_bounder)
    }

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.ftile.as_ref()) {
            return UTranslate::default();
        }
        self.ftile.get_translate_for(child, string_bounder)
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.ftile.clone()]
    }

    /// Only the arrows added here: PlantUML drops those of the tile.
    fn get_inner_connections(&self) -> Vec<Rc<dyn Connection>> {
        self.connections.clone()
    }

    fn get_welding_points(&self) -> Vec<Rc<dyn Ftile>> {
        self.ftile.get_welding_points()
    }

    fn arrow_horizontal_alignment(&self) -> HorizontalAlignment {
        self.ftile.arrow_horizontal_alignment()
    }

    /// The tile draws itself on `ug` rather than through it, as in PlantUML.
    fn draw_u(&self, ug: &UGraphic) {
        self.ftile.draw_u(ug);
        for connection in &self.connections {
            ug.draw(connection);
        }
    }
}

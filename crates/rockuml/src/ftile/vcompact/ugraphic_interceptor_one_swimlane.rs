//! The layer that draws the tile tree of a diagram with swimlanes one lane at a time (PlantUML's
//! `UGraphicInterceptorOneSwimlane`): it keeps the tiles lying in its lane and the connections within it.

use std::rc::Rc;

use crate::diagram::activity3::SwimlaneId;
use crate::klimt::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};

pub(crate) struct UGraphicInterceptorOneSwimlane {
    ug: UGraphic,
    swimlane: SwimlaneId,
    ordered_list: Rc<[SwimlaneId]>,
}

impl UGraphicInterceptorOneSwimlane {
    /// Drawing `swimlane`, one of `ordered_list` or the lane after them all.
    pub(crate) fn create(
        ug: UGraphic,
        swimlane: SwimlaneId,
        ordered_list: Rc<[SwimlaneId]>,
    ) -> UGraphic {
        UGraphic::from_layer(Self {
            ug,
            swimlane,
            ordered_list,
        })
    }

    pub(crate) fn get_swimlane(&self) -> SwimlaneId {
        self.swimlane
    }

    /// The lanes in their order of declaration, without the lane after them all.
    pub(crate) fn get_ordered_list_of_all_swimlanes(&self) -> &[SwimlaneId] {
        &self.ordered_list
    }
}

impl UGraphicLayer for UGraphicInterceptorOneSwimlane {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    fn apply(&self, change: UChange) -> UGraphic {
        Self::create(
            self.ug.apply(change),
            self.swimlane,
            self.ordered_list.clone(),
        )
    }

    fn draw(&self, this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::Ftile(tile) => {
                if tile.get_swimlanes().contains(&Some(self.swimlane)) {
                    tile.draw_u(this);
                }
            }
            AnyShape::Connection(connection) => {
                let contained1 = connection
                    .get_ftile1()
                    .and_then(|tile1| tile1.get_swimlane_out())
                    .is_none_or(|out| out == self.swimlane);
                let contained2 = connection
                    .get_ftile2()
                    .and_then(|tile2| tile2.get_swimlane_in())
                    .is_none_or(|into| into == self.swimlane);
                if contained1 && contained2 {
                    connection.draw_u(this);
                }
            }
            shape => self.ug.draw(shape),
        }
    }
}

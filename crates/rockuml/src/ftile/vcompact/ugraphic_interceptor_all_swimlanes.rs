//! The layer that measures every swimlane in one walk of the tile tree (PlantUML's
//! `UGraphicInterceptorAllSwimlanes`): what lies in a lane goes to that lane's surface.

use std::rc::Rc;

use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::klimt::group::UGroup;
use crate::klimt::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};
use crate::klimt::url::Url;

pub(crate) struct UGraphicInterceptorAllSwimlanes {
    /// Each lane's surface, in the order of the lanes.
    swimlane_to_ug: Rc<[(SwimlaneId, UGraphic)]>,
    /// The lanes of the tiles being drawn, in the order of the lanes: shapes go to their surfaces.
    active_swimlanes: Rc<[SwimlaneId]>,
}

impl UGraphicInterceptorAllSwimlanes {
    /// Over `swimlane_to_ug`, which holds at least one lane.
    pub(crate) fn create(swimlane_to_ug: Vec<(SwimlaneId, UGraphic)>) -> UGraphic {
        let active_swimlanes = swimlane_to_ug
            .iter()
            .map(|(swimlane, _)| *swimlane)
            .collect();
        UGraphic::from_layer(Self {
            swimlane_to_ug: swimlane_to_ug.into(),
            active_swimlanes,
        })
    }

    fn surfaces_of_active_swimlanes(&self) -> impl Iterator<Item = (SwimlaneId, &UGraphic)> {
        self.swimlane_to_ug
            .iter()
            .filter(|(swimlane, _)| self.active_swimlanes.contains(swimlane))
            .map(|(swimlane, ug)| (*swimlane, ug))
    }

    /// The layer keeping, of the active lanes, those of a tile lying in `tile_swimlanes`.
    fn with_active_swimlanes(&self, this: &UGraphic, tile_swimlanes: &SwimlaneSet) -> UGraphic {
        let in_tile = |swimlane: &SwimlaneId| tile_swimlanes.contains(&Some(*swimlane));
        if self.active_swimlanes.iter().all(in_tile) {
            return this.clone();
        }
        UGraphic::from_layer(Self {
            swimlane_to_ug: self.swimlane_to_ug.clone(),
            active_swimlanes: self
                .active_swimlanes
                .iter()
                .copied()
                .filter(in_tile)
                .collect(),
        })
    }
}

impl UGraphicLayer for UGraphicInterceptorAllSwimlanes {
    /// The first lane's surface, which answers the queries.
    fn ug(&self) -> &UGraphic {
        &self.swimlane_to_ug[0].1
    }

    fn apply(&self, change: UChange) -> UGraphic {
        UGraphic::from_layer(Self {
            swimlane_to_ug: self
                .swimlane_to_ug
                .iter()
                .map(|(swimlane, ug)| (*swimlane, ug.apply(change.clone())))
                .collect(),
            active_swimlanes: self.active_swimlanes.clone(),
        })
    }

    fn draw(&self, this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::Ftile(tile) => {
                let tile_swimlanes = tile.get_swimlanes();
                let has_match = self
                    .active_swimlanes
                    .iter()
                    .any(|swimlane| tile_swimlanes.contains(&Some(*swimlane)));
                if has_match {
                    tile.draw_u(&self.with_active_swimlanes(this, &tile_swimlanes));
                }
            }
            AnyShape::Connection(connection) => {
                let swimlane_out = connection
                    .get_ftile1()
                    .and_then(|tile1| tile1.get_swimlane_out());
                let swimlane_in = connection
                    .get_ftile2()
                    .and_then(|tile2| tile2.get_swimlane_in());
                for (swimlane, ug) in self.surfaces_of_active_swimlanes() {
                    if swimlane_out.is_none_or(|out| out == swimlane)
                        && swimlane_in.is_none_or(|into| into == swimlane)
                    {
                        connection.draw_u(ug);
                    }
                }
            }
            shape => {
                for (_, ug) in self.surfaces_of_active_swimlanes() {
                    ug.draw(shape);
                }
            }
        }
    }

    fn flush_ug(&self) {
        for (_, ug) in self.swimlane_to_ug.iter() {
            ug.flush_ug();
        }
    }

    fn start_group(&self, _group: &UGroup) {}

    fn close_group(&self) {}

    fn start_url(&self, _url: &Url) {}

    fn close_url(&self) {}
}

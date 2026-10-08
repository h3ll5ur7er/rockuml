//! Tiles that draw nothing (PlantUML's `FtileEmpty`, and its subclasses `FtileLabel`, `FtileGoto` and
//! `FtileBreak`).

use std::rc::{Rc, Weak};

use super::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

/// An empty list, or a point the flow passes through.
pub(crate) struct FtileEmpty {
    base: AbstractFtile,
    width: f64,
    height: f64,
    swimlane: Option<SwimlaneId>,
}

impl FtileEmpty {
    pub(crate) fn new(skin_param: Rc<SkinParam>, swimlane: Option<SwimlaneId>) -> Self {
        Self::with_size(skin_param, 0.0, 0.0, swimlane)
    }

    /// A point taking `width` and `height`, as where only one branch of an `if` goes on.
    pub(crate) fn with_size(
        skin_param: Rc<SkinParam>,
        width: f64,
        height: f64,
        swimlane: Option<SwimlaneId>,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            width,
            height,
            swimlane,
        }
    }

    /// The tile's size, entered at the top and left at the bottom (`calculateDimensionEmpty`).
    fn calculate_dimension_empty(&self) -> FtileGeometry {
        FtileGeometry::with_out(self.width, self.height, self.width / 2.0, 0.0, self.height)
    }
}

impl Swimable for FtileEmpty {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.swimlane.map(Some).into_iter().collect()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.swimlane
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.swimlane
    }
}

impl Ftile for FtileEmpty {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base
            .calculate_dimension(|| self.calculate_dimension_empty())
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}

/// Forwards what an empty tile's subclass does not change to its `empty` field.
macro_rules! empty_swimable {
    ($tile:ty) => {
        impl Swimable for $tile {
            fn get_swimlanes(&self) -> SwimlaneSet {
                self.empty.get_swimlanes()
            }

            fn get_swimlane_in(&self) -> Option<SwimlaneId> {
                self.empty.get_swimlane_in()
            }

            fn get_swimlane_out(&self) -> Option<SwimlaneId> {
                self.empty.get_swimlane_out()
            }
        }
    };
}

/// Where a `goto` of the same name jumps to.
pub(crate) struct FtileLabel {
    empty: FtileEmpty,
    name: String,
}

impl FtileLabel {
    pub(crate) fn new(skin_param: Rc<SkinParam>, swimlane: Option<SwimlaneId>, name: &str) -> Self {
        Self {
            empty: FtileEmpty::new(skin_param, swimlane),
            name: name.to_owned(),
        }
    }

    pub(crate) fn get_name(&self) -> &str {
        &self.name
    }
}

empty_swimable!(FtileLabel);

impl Ftile for FtileLabel {
    fn skin_param(&self) -> &SkinParam {
        self.empty.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.empty.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}

/// Jumps to the label of its name; the flow does not go on below it.
pub(crate) struct FtileGoto {
    empty: FtileEmpty,
    name: String,
}

impl FtileGoto {
    pub(crate) fn new(skin_param: Rc<SkinParam>, swimlane: Option<SwimlaneId>, name: &str) -> Self {
        Self {
            empty: FtileEmpty::new(skin_param, swimlane),
            name: name.to_owned(),
        }
    }

    pub(crate) fn get_name(&self) -> &str {
        &self.name
    }
}

empty_swimable!(FtileGoto);

impl Ftile for FtileGoto {
    fn skin_param(&self) -> &SkinParam {
        self.empty.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.empty
            .base
            .calculate_dimension(|| self.empty.calculate_dimension_empty().without_point_out())
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}

/// Leaves the loop around it: the loop joins it to its exit (a `WeldingPoint`).
pub(crate) struct FtileBreak {
    empty: FtileEmpty,
    /// The tile itself, which it lists as its welding point.
    me: Weak<FtileBreak>,
}

impl FtileBreak {
    pub(crate) fn create(skin_param: Rc<SkinParam>, swimlane: Option<SwimlaneId>) -> Rc<Self> {
        Rc::new_cyclic(|me| Self {
            empty: FtileEmpty::new(skin_param, swimlane),
            me: me.clone(),
        })
    }
}

empty_swimable!(FtileBreak);

impl Ftile for FtileBreak {
    fn skin_param(&self) -> &SkinParam {
        self.empty.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.empty
            .base
            .calculate_dimension(|| self.empty.calculate_dimension_empty().without_point_out())
    }

    fn get_welding_points(&self) -> Vec<Rc<dyn Ftile>> {
        self.me
            .upgrade()
            .map(|me| me as Rc<dyn Ftile>)
            .into_iter()
            .collect()
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}

//! What the condition tiles share (PlantUML's `FtileDiamondWIP`): their colours, their lane and the labels
//! around them.

use std::cell::Cell;
use std::rc::Rc;

use crate::color::HColor;
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::ftile::AbstractFtile;
use crate::klimt::TextBlock;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UChange, UGraphic};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::style::{SName, Style, StyleSignature};

/// Embedded in each condition tile, which forwards [`crate::ftile::Swimable`] to it.
///
/// PlantUML's `with...` methods build a new tile; these set the label on the tile being built, which nothing
/// has measured yet.
pub(crate) struct FtileDiamondWIP {
    pub(super) base: AbstractFtile,
    back_color: HColor,
    border_color: HColor,
    swimlane: Option<SwimlaneId>,
    pub(super) north: Rc<dyn TextBlock>,
    pub(super) south: Rc<dyn TextBlock>,
    west: Rc<dyn TextBlock>,
    east: Rc<dyn TextBlock>,
    /// `FtileIfDown` moves the labels of a tile already built to the other side.
    east_west_swapped: Cell<bool>,
}

/// A label taking no room (`TextBlockUtils.empty(0, 0)`).
pub(crate) fn empty_label() -> Rc<dyn TextBlock> {
    Rc::new(TextBlockEmpty::default())
}

impl FtileDiamondWIP {
    pub(super) fn new(
        skin_param: Rc<SkinParam>,
        back_color: HColor,
        border_color: HColor,
        swimlane: Option<SwimlaneId>,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            back_color,
            border_color,
            swimlane,
            north: empty_label(),
            south: empty_label(),
            west: empty_label(),
            east: empty_label(),
            east_west_swapped: Cell::new(false),
        }
    }

    pub(super) fn set_west(&mut self, west: Rc<dyn TextBlock>) {
        self.west = west;
    }

    pub(super) fn set_east(&mut self, east: Rc<dyn TextBlock>) {
        self.east = east;
    }

    pub(super) fn west(&self) -> &Rc<dyn TextBlock> {
        if self.east_west_swapped.get() {
            &self.east
        } else {
            &self.west
        }
    }

    pub(super) fn east(&self) -> &Rc<dyn TextBlock> {
        if self.east_west_swapped.get() {
            &self.west
        } else {
            &self.east
        }
    }

    /// `swapEastWest`.
    pub(super) fn swap_east_west(&self) {
        self.east_west_swapped.set(!self.east_west_swapped.get());
    }

    fn get_style(&self) -> Style {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
            SName::Diamond,
        ])
        .get_merged_style(&self.base.skin_param().current_style_builder())
    }

    /// `ug` drawing outlines in the border colour and the diamond style's stroke, filled with the back
    /// colour.
    pub(super) fn styled(&self, ug: &UGraphic) -> UGraphic {
        ug.apply(UChange::Color(self.border_color.clone()))
            .apply(self.get_style().stroke())
            .apply(UChange::Background(self.back_color.clone()))
    }

    pub(super) fn draw_outline(&self, ug: &UGraphic, outline: &UShape) {
        self.styled(ug).draw(outline);
    }

    /// No lane for a diagram without swimlanes, as PlantUML's empty set.
    pub(super) fn get_swimlanes(&self) -> SwimlaneSet {
        self.swimlane.map(Some).into_iter().collect()
    }

    pub(super) fn get_swimlane(&self) -> Option<SwimlaneId> {
        self.swimlane
    }
}

/// Implements [`crate::ftile::Swimable`] for a condition tile through its `wip` field.
macro_rules! swimable_through_wip {
    ($tile:ty) => {
        impl $crate::ftile::Swimable for $tile {
            fn get_swimlanes(&self) -> $crate::diagram::activity3::SwimlaneSet {
                self.wip.get_swimlanes()
            }

            fn get_swimlane_in(&self) -> Option<$crate::diagram::activity3::SwimlaneId> {
                self.wip.get_swimlane()
            }

            fn get_swimlane_out(&self) -> Option<$crate::diagram::activity3::SwimlaneId> {
                self.wip.get_swimlane()
            }
        }
    };
}
pub(super) use swimable_through_wip;

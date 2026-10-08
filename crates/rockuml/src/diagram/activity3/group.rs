//! A titled frame around instructions: `partition`, `package`, `rectangle`, `card` or `group` (PlantUML's
//! `InstructionGroup`).

use super::instruction::{InstructionId, InstructionList, PositionedNote};
use super::link_rendering::LinkRendering;
use super::swimlanes::SwimlaneId;
use crate::color::HColor;
use crate::creole::Display;
use crate::decoration::symbol::USymbol;
use crate::style::Style;

pub(crate) struct InstructionGroup {
    pub(crate) list: InstructionList,
    pub(crate) parent: InstructionId,
    pub(crate) back_color: Option<HColor>,
    pub(crate) link_rendering: LinkRendering,
    pub(crate) type_: USymbol,
    pub(crate) title: Display,
    /// The note written before the group's first instruction; a later one replaces it.
    pub(crate) note: Option<PositionedNote>,
    pub(crate) style: Style,
}

impl InstructionGroup {
    pub(crate) fn new(
        parent: InstructionId,
        title: Display,
        back_color: Option<HColor>,
        swimlane: Option<SwimlaneId>,
        link_rendering: LinkRendering,
        type_: USymbol,
        style: Style,
    ) -> Self {
        Self {
            list: InstructionList::new(swimlane),
            parent,
            back_color,
            link_rendering,
            type_,
            title,
            note: None,
            style,
        }
    }
}

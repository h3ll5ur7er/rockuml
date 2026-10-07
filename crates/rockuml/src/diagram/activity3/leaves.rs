//! The instructions that hold no others: activities, circles, `break`, `goto` and `label` (PlantUML's
//! `InstructionSimple`, `InstructionSpot`, `InstructionStart`, `InstructionStop`, `InstructionEnd`,
//! `InstructionBreak`, `InstructionGoto` and `InstructionLabel`).

use std::rc::Rc;

use super::instruction::MonoSwimable;
use super::link_rendering::LinkRendering;
use super::swimlanes::SwimlaneId;
use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::ftile::BoxStyle;
use crate::klimt::url::Url;
use crate::stereo::Stereotype;
use crate::style::StyleBuilder;

/// An activity: `:label;`.
pub(crate) struct InstructionSimple {
    pub(crate) mono: MonoSwimable,
    pub(crate) killed: bool,
    pub(crate) label: Display,
    pub(crate) colors: Colors,
    pub(crate) inlink_rendering: LinkRendering,
    pub(crate) box_style: BoxStyle,
    pub(crate) url: Option<Url>,
    pub(crate) stereotype: Option<Stereotype>,
    pub(crate) style_builder: Rc<StyleBuilder>,
}

/// A circle with a letter: `(A)`.
pub(crate) struct InstructionSpot {
    pub(crate) mono: MonoSwimable,
    pub(crate) killed: bool,
    pub(crate) inlink_rendering: LinkRendering,
    pub(crate) spot: String,
    pub(crate) color: Option<HColor>,
}

/// `start`, `stop` and `end`: the circles the flow starts and ends at.
pub(crate) struct InstructionStart {
    pub(crate) mono: MonoSwimable,
    pub(crate) inlink_rendering: LinkRendering,
    pub(crate) colors: Colors,
}

pub(crate) struct InstructionStop {
    pub(crate) mono: MonoSwimable,
    pub(crate) inlink_rendering: LinkRendering,
    pub(crate) colors: Colors,
}

pub(crate) struct InstructionEnd {
    pub(crate) mono: MonoSwimable,
    pub(crate) inlink_rendering: LinkRendering,
    pub(crate) colors: Colors,
}

/// Leaves the loop around it.
pub(crate) struct InstructionBreak {
    pub(crate) mono: MonoSwimable,
    pub(crate) inlink_rendering: LinkRendering,
}

/// Jumps to the label of that name.
pub(crate) struct InstructionGoto {
    pub(crate) mono: MonoSwimable,
    pub(crate) name: String,
}

pub(crate) struct InstructionLabel {
    pub(crate) mono: MonoSwimable,
    pub(crate) name: String,
}

impl InstructionSimple {
    #[expect(clippy::too_many_arguments, reason = "PlantUML's constructor")]
    pub(crate) fn new(
        label: Display,
        inlink_rendering: LinkRendering,
        swimlane: Option<SwimlaneId>,
        box_style: BoxStyle,
        url: Option<Url>,
        colors: Colors,
        stereotype: Option<Stereotype>,
        style_builder: Rc<StyleBuilder>,
    ) -> Self {
        Self {
            mono: MonoSwimable::new(swimlane),
            killed: false,
            label,
            colors,
            inlink_rendering,
            box_style,
            url,
            stereotype,
            style_builder,
        }
    }
}

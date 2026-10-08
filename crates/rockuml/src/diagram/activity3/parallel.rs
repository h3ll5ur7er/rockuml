//! Flows side by side: `fork` (ending in a join or a merge) and `split` (PlantUML's `InstructionFork`,
//! `InstructionSplit` and `ForkStyle`).

use super::instruction::{InstructionId, InstructionList, WithNote};
use super::link_rendering::LinkRendering;
use super::swimlanes::SwimlaneId;
use crate::color::Colors;

/// How the flows side by side end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ForkStyle {
    Fork,
    Split,
    Merge,
}

pub(crate) struct InstructionFork {
    pub(crate) forks: Vec<InstructionList>,
    pub(crate) parent: InstructionId,
    pub(crate) inlink_rendering: LinkRendering,
    pub(crate) swimlane_in: Option<SwimlaneId>,
    pub(crate) swimlane_out: Option<SwimlaneId>,
    pub(crate) style: ForkStyle,
    /// The join specification, like `{or}`.
    pub(crate) label: Option<String>,
    pub(crate) finished: bool,
    pub(crate) colors: Colors,
    pub(crate) notes: WithNote,
}

impl InstructionFork {
    pub(crate) fn new(
        parent: InstructionId,
        inlink_rendering: LinkRendering,
        swimlane: Option<SwimlaneId>,
        colors: Colors,
    ) -> Self {
        Self {
            forks: vec![InstructionList::new(None)],
            parent,
            inlink_rendering,
            swimlane_in: swimlane,
            swimlane_out: swimlane,
            style: ForkStyle::Fork,
            label: None,
            finished: false,
            colors,
            notes: WithNote::default(),
        }
    }

    pub(crate) fn get_last_list(&self) -> &InstructionList {
        self.forks.last().expect("a fork has a first flow")
    }

    pub(crate) fn get_last_list_mut(&mut self) -> &mut InstructionList {
        self.forks.last_mut().expect("a fork has a first flow")
    }

    pub(crate) fn fork_again(&mut self, swimlane: Option<SwimlaneId>) {
        self.swimlane_out = swimlane;
        self.forks.push(InstructionList::new(None));
    }

    pub(crate) fn manage_out_rendering(
        &mut self,
        next_link_renderer: LinkRendering,
        end_fork: bool,
    ) {
        if end_fork {
            self.finished = true;
        }
        self.get_last_list_mut()
            .set_out_rendering(next_link_renderer);
    }

    pub(crate) fn set_style(
        &mut self,
        style: ForkStyle,
        label: Option<String>,
        swimlane: Option<SwimlaneId>,
    ) {
        self.style = style;
        self.label = label;
        self.swimlane_out = swimlane;
    }
}

pub(crate) struct InstructionSplit {
    pub(crate) splits: Vec<InstructionList>,
    pub(crate) parent: InstructionId,
    pub(crate) inlink_rendering: LinkRendering,
    pub(crate) swimlane_in: Option<SwimlaneId>,
    pub(crate) swimlane_out: Option<SwimlaneId>,
}

impl InstructionSplit {
    pub(crate) fn new(
        parent: InstructionId,
        inlink_rendering: LinkRendering,
        swimlane: Option<SwimlaneId>,
    ) -> Self {
        Self {
            splits: vec![InstructionList::new(swimlane)],
            parent,
            inlink_rendering,
            swimlane_in: swimlane,
            swimlane_out: None,
        }
    }

    pub(crate) fn get_last(&self) -> &InstructionList {
        self.splits.last().expect("a split has a first flow")
    }

    pub(crate) fn get_last_mut(&mut self) -> &mut InstructionList {
        self.splits.last_mut().expect("a split has a first flow")
    }

    pub(crate) fn split_again(&mut self, inlink_rendering: LinkRendering) {
        self.get_last_mut().set_out_rendering(inlink_rendering);
        self.splits.push(InstructionList::new(self.swimlane_in));
    }

    pub(crate) fn end_split(
        &mut self,
        inlink_rendering: LinkRendering,
        end_swimlane: Option<SwimlaneId>,
    ) {
        self.get_last_mut().set_out_rendering(inlink_rendering);
        self.swimlane_out = end_swimlane;
    }
}

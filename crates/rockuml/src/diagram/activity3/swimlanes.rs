//! The swimlanes of an activity diagram, and the state its commands build the instructions with: the
//! instruction that takes the next ones, the lane they go to, the arrow leading to the next one (the model
//! half of PlantUML's `Swimlanes`, and `Swimlane`).

use super::instruction::{InstructionId, Instructions};
use super::link_rendering::LinkRendering;
use crate::color::{ColorType, Colors, HColor};
use crate::creole::Display;

/// A swimlane, known by its order of declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SwimlaneId(usize);

pub(crate) struct Swimlane {
    pub(crate) name: String,
    pub(crate) order: SwimlaneId,
    pub(crate) display: Display,
    pub(crate) colors: Colors,
}

pub(crate) struct Swimlanes {
    lanes: Vec<Swimlane>,
    current_swimlane: Option<SwimlaneId>,
    /// Every instruction, the root list first.
    pub(crate) instructions: Instructions,
    current_instruction: InstructionId,
    next_link_renderer: LinkRendering,
}

impl Swimlanes {
    pub(crate) fn new() -> Self {
        Self {
            lanes: Vec::new(),
            current_swimlane: None,
            instructions: Instructions::new(),
            current_instruction: Instructions::ROOT,
            next_link_renderer: LinkRendering::none(),
        }
    }

    /// Makes the lane `name` current, declaring it first if needed, and colours or labels it.
    pub(crate) fn swimlane(&mut self, name: &str, color: Option<HColor>, label: Option<Display>) {
        let id = self.get_or_create(name);
        self.current_swimlane = Some(id);
        let lane = &mut self.lanes[id.0];
        lane.colors = lane.colors.with(ColorType::Back, color);
        if let Some(label) = label {
            lane.display = label;
        }
    }

    fn get_or_create(&mut self, name: &str) -> SwimlaneId {
        if let Some(lane) = self.lanes.iter().find(|lane| lane.name == name) {
            return lane.order;
        }
        let order = SwimlaneId(self.lanes.len());
        self.lanes.push(Swimlane {
            name: name.to_owned(),
            order,
            display: Display::with_newlines(name),
            colors: Colors::default(),
        });
        order
    }

    /// The lanes in their order of declaration.
    pub(crate) fn swimlanes(&self) -> &[Swimlane] {
        &self.lanes
    }

    pub(crate) fn lane(&self, id: SwimlaneId) -> &Swimlane {
        &self.lanes[id.0]
    }

    pub(crate) fn get_current(&self) -> InstructionId {
        self.current_instruction
    }

    pub(crate) fn set_current(&mut self, current: InstructionId) {
        self.current_instruction = current;
    }

    pub(crate) fn next_link_renderer(&self) -> &LinkRendering {
        &self.next_link_renderer
    }

    pub(crate) fn set_next_link_renderer(&mut self, link: LinkRendering) {
        self.next_link_renderer = link;
    }

    pub(crate) fn get_current_swimlane(&self) -> Option<SwimlaneId> {
        self.current_swimlane
    }
}

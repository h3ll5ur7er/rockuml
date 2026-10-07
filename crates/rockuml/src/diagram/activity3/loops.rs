//! Loops: `while` tested before its body, `repeat` after it (PlantUML's `InstructionWhile` and
//! `InstructionRepeat`).

use std::rc::Rc;

use super::instruction::{InstructionId, InstructionList, PositionedNote, WithNote};
use super::link_rendering::LinkRendering;
use super::swimlanes::SwimlaneId;
use crate::color::HColor;
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::ftile::BoxStyle;
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::StyleBuilder;

pub(crate) struct InstructionWhile {
    pub(crate) repeat_list: InstructionList,
    pub(crate) parent: InstructionId,
    pub(crate) next_link_renderer: LinkRendering,
    pub(crate) color: Option<HColor>,
    pub(crate) killed: bool,
    pub(crate) test: Display,
    pub(crate) yes: Option<Display>,
    pub(crate) test_called: bool,
    pub(crate) out_color: LinkRendering,
    pub(crate) swimlane: Option<SwimlaneId>,
    /// A `stop` or `end` right after `endwhile`, which ends the loop's way out.
    pub(crate) special_out: Option<InstructionId>,
    pub(crate) box_style: Option<BoxStyle>,
    pub(crate) backward: Option<Display>,
    pub(crate) stereotype: Option<Stereotype>,
    pub(crate) incoming1: LinkRendering,
    pub(crate) incoming2: LinkRendering,
    pub(crate) backward_called: bool,
    pub(crate) current_style_builder: Rc<StyleBuilder>,
    pub(crate) notes: WithNote,
}

impl InstructionWhile {
    pub(crate) fn new(
        swimlane: Option<SwimlaneId>,
        parent: InstructionId,
        test: Display,
        next_link_renderer: LinkRendering,
        yes: Option<Display>,
        color: Option<HColor>,
        current_style_builder: Rc<StyleBuilder>,
    ) -> Self {
        Self {
            repeat_list: InstructionList::new(None),
            parent,
            next_link_renderer,
            color,
            killed: false,
            test,
            yes,
            test_called: false,
            out_color: LinkRendering::none(),
            swimlane,
            special_out: None,
            box_style: None,
            backward: None,
            stereotype: None,
            incoming1: LinkRendering::none(),
            incoming2: LinkRendering::none(),
            backward_called: false,
            current_style_builder,
            notes: WithNote::default(),
        }
    }

    pub(crate) fn overwrite_yes(&mut self, yes: Option<Display>) {
        self.yes = yes;
    }

    pub(crate) fn out_display(&mut self, out: Option<Display>) {
        self.out_color = self.out_color.with_display(out);
    }

    pub(crate) fn out_color(&mut self, rainbow: Rainbow) {
        self.out_color = self.out_color.with_rainbow(rainbow);
    }

    pub(crate) fn set_special(&mut self, special: InstructionId) {
        self.special_out = Some(special);
    }

    pub(crate) fn set_backward(
        &mut self,
        label: Display,
        box_style: BoxStyle,
        incoming1: LinkRendering,
        incoming2: LinkRendering,
        stereotype: Option<Stereotype>,
    ) {
        self.backward = Some(label);
        self.box_style = Some(box_style);
        self.incoming1 = incoming1;
        self.incoming2 = incoming2;
        self.backward_called = true;
        self.stereotype = stereotype;
    }

    /// The arrow into `endwhile`, which goes back to the test unless `backward` drew its own.
    pub(crate) fn incoming(&mut self, incoming: &LinkRendering) {
        if !self.backward_called {
            self.incoming1 = incoming.clone();
            self.incoming2 = incoming.clone();
        }
        self.test_called = true;
    }
}

pub(crate) struct InstructionRepeat {
    pub(crate) repeat_list: InstructionList,
    pub(crate) parent: InstructionId,
    pub(crate) next_link_renderer: LinkRendering,
    pub(crate) swimlane: Option<SwimlaneId>,
    pub(crate) swimlane_out: Option<SwimlaneId>,
    pub(crate) swimlane_backward: Option<SwimlaneId>,
    pub(crate) box_style: Option<BoxStyle>,
    pub(crate) killed: bool,
    pub(crate) box_style_in: BoxStyle,
    pub(crate) backward: Option<Display>,
    pub(crate) stereogroup_loop: Stereogroup,
    pub(crate) stereotype_back: Option<Stereotype>,
    /// The arrow back to the start; while the test is missing, the diagram's next arrow when drawn.
    pub(crate) incoming1: LinkRendering,
    pub(crate) incoming2: LinkRendering,
    pub(crate) backward_notes: Vec<PositionedNote>,
    pub(crate) test: Option<Display>,
    pub(crate) yes: Option<Display>,
    pub(crate) out: Option<Display>,
    pub(crate) start_label: Option<Display>,
    pub(crate) test_called: bool,
    pub(crate) end_repeat_link_rendering: LinkRendering,
    pub(crate) current_style_builder: Rc<StyleBuilder>,
    pub(crate) stereotype2: Stereogroup,
}

impl InstructionRepeat {
    pub(crate) fn new(
        swimlane: Option<SwimlaneId>,
        parent: InstructionId,
        next_link_renderer: LinkRendering,
        start_label: Option<Display>,
        box_style_in: BoxStyle,
        stereogroup: Stereogroup,
        current_style_builder: Rc<StyleBuilder>,
    ) -> Self {
        Self {
            repeat_list: InstructionList::new(swimlane),
            parent,
            next_link_renderer,
            swimlane,
            swimlane_out: None,
            swimlane_backward: None,
            box_style: None,
            killed: false,
            box_style_in,
            backward: None,
            stereogroup_loop: stereogroup,
            stereotype_back: None,
            incoming1: LinkRendering::none(),
            incoming2: LinkRendering::none(),
            backward_notes: Vec::new(),
            test: None,
            yes: None,
            out: None,
            start_label,
            test_called: false,
            end_repeat_link_rendering: LinkRendering::none(),
            current_style_builder,
            stereotype2: Stereogroup::default(),
        }
    }

    pub(crate) fn set_backward(
        &mut self,
        label: Display,
        swimlane_backward: Option<SwimlaneId>,
        box_style: BoxStyle,
        incoming1: LinkRendering,
        incoming2: LinkRendering,
        stereotype: Option<Stereotype>,
    ) {
        self.backward = Some(label);
        self.swimlane_backward = swimlane_backward;
        self.box_style = Some(box_style);
        self.incoming1 = incoming1;
        self.incoming2 = incoming2;
        self.stereotype_back = stereotype;
    }

    #[expect(clippy::too_many_arguments, reason = "PlantUML's setTest")]
    pub(crate) fn set_test(
        &mut self,
        test: Option<Display>,
        yes: Option<Display>,
        out: Option<Display>,
        end_repeat_link_rendering: LinkRendering,
        back: LinkRendering,
        swimlane_out: Option<SwimlaneId>,
        stereotype2: Stereogroup,
    ) {
        self.stereotype2 = stereotype2;
        self.swimlane_out = swimlane_out;
        self.test = test;
        self.yes = yes;
        self.out = out;
        self.end_repeat_link_rendering = end_repeat_link_rendering;
        if !back.is_none() {
            self.incoming1 = back;
        }
        self.test_called = true;
    }
}

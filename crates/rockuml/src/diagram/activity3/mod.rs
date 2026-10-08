//! Activity diagrams (PlantUML's `activitydiagram3` package): the commands build a tree of instructions,
//! which the tiles of `ftile` draw.

// The instructions keep what their tiles are drawn from, which the rendering will read once ported.
#![expect(dead_code, reason = "the ftile rendering is not ported yet")]

mod branch;
mod commands;
mod create_ftile;
mod group;
mod instruction;
mod leaves;
mod link_rendering;
mod loops;
mod parallel;
mod recentred;
mod swimlanes;
#[cfg(test)]
mod tests;

use std::rc::Rc;

pub(crate) use self::branch::BranchFtile;
use self::branch::{InstructionIf, InstructionSwitch};
use self::group::InstructionGroup;
use self::instruction::{Instruction, MonoSwimable};
pub(crate) use self::instruction::{
    InstructionId, Instructions, NoteType, PositionedNote, SwimlaneSet,
};
use self::leaves::{
    InstructionBreak, InstructionEnd, InstructionGoto, InstructionLabel, InstructionSimple,
    InstructionSpot, InstructionStart, InstructionStop,
};
pub(crate) use self::link_rendering::LinkRendering;
use self::loops::{InstructionRepeat, InstructionWhile};
pub(crate) use self::parallel::ForkStyle;
use self::parallel::{InstructionFork, InstructionSplit};
use self::recentred::Recentred;
pub(crate) use self::swimlanes::SwimlaneId;
use self::swimlanes::{Swimlanes, SwimlanesDrawing};
use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::cuca_commands;
use super::diagram_type::DiagramType;
use super::titled::{PragmaKey, Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::{Colors, HColor};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult};
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::decoration::symbol::USymbol;
pub(crate) use crate::diagram::sequence::model::NotePosition;
use crate::ftile::BoxStyle;
use crate::klimt::TextBlock;
use crate::klimt::compress::{CompressionMode, CompressionXorYBuilder};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::url::Url;
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::{SName, Style};

/// Whether swimlanes may still be declared: only before the first instruction that goes in one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SwimlaneStrategy {
    SwimlaneForbidden,
    SwimlaneAllowed,
}

pub(super) struct ActivityDiagram3 {
    source: Rc<UmlSource>,
    titled: Titled,
    swimlane_strategy: Option<SwimlaneStrategy>,
    swimlanes: Swimlanes,
}

/// Reads activity diagrams (PlantUML's `ActivityDiagramFactory3`).
pub(super) struct ActivityDiagramFactory3;

impl CommandFactory for ActivityDiagramFactory3 {
    type Diagram = ActivityDiagram3;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Activity;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ActivityDiagram3 {
        let titled = Titled::new(SName::ActivityDiagram, "ACTIVITY", source);
        ActivityDiagram3 {
            source: source.clone(),
            titled,
            swimlane_strategy: None,
            swimlanes: Swimlanes::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<ActivityDiagram3>>> {
        let mut commands = vec![cuca_commands::footbox_ignored()];
        commands.extend(add_common_commands1());
        commands.extend([
            commands::swimlane(),
            commands::swimlane2(),
            commands::partition3(),
            commands::close_group3(),
            commands::close_group_legacy3(),
            commands::arrow3(),
            commands::arrow_long3(),
            commands::repeat3(),
            commands::activity3(),
            commands::if4(),
            commands::if2(),
            commands::if2_multine(),
            commands::if_legacy1(),
            commands::else_if3(),
            commands::else_if2(),
            commands::else3(),
            commands::else3_multine(),
            commands::else_legacy1(),
            commands::endif3(),
            commands::switch(),
            commands::case(),
            commands::end_switch(),
            commands::repeat_while3(),
            commands::repeat_while3_multilines(),
            commands::backward3(),
            commands::backward_long3(),
            commands::while3(),
            commands::while_end3(),
            commands::fork3(),
            commands::fork_again3(),
            commands::fork_end3(),
            commands::split3(),
            commands::split_again3(),
            commands::split_end3(),
            commands::start3(),
            commands::stop3(),
            commands::circle_spot3(),
            commands::break_command(),
            commands::end3(),
            commands::kill3(),
            commands::link3(),
            commands::note3(),
            commands::note_long3(),
            commands::activity_long3(),
            commands::activity_list(),
            commands::label(),
            commands::goto(),
            commands::else_if2_multine(),
        ]);
        commands
    }
}

impl AbstractDiagram for ActivityDiagram3 {}

impl TitledDiagram for ActivityDiagram3 {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for ActivityDiagram3 {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        if self.swimlanes.swimlanes().len() > 1 {
            return Err(NotYetPorted("activity diagrams with swimlanes (track E2)"));
        }
        let swimlanes = SwimlanesDrawing::new(
            &self.swimlanes,
            Rc::new(self.titled.skin.clone()),
            self.titled.pragma.is_true(PragmaKey::UseVerticalIf),
            string_bounder.clone(),
        );
        let compressed = CompressionXorYBuilder::build(
            CompressionMode::OnY,
            CompressionXorYBuilder::build(CompressionMode::OnX, swimlanes),
        );
        let result = Recentred::new(compressed);
        Ok(self.titled.add_chrome(Box::new(result), string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

/// The builder the commands call (PlantUML's `ActivityDiagram3` methods).
impl ActivityDiagram3 {
    fn manage_swimlane_strategy(&mut self) {
        self.swimlane_strategy
            .get_or_insert(SwimlaneStrategy::SwimlaneForbidden);
    }

    fn swimlane(
        &mut self,
        name: &str,
        color: Option<HColor>,
        label: Option<Display>,
    ) -> CommandResult {
        if *self
            .swimlane_strategy
            .get_or_insert(SwimlaneStrategy::SwimlaneAllowed)
            == SwimlaneStrategy::SwimlaneForbidden
        {
            return Err(CommandError::new(
                "This swimlane must be defined at the start of the diagram.",
            ));
        }
        self.swimlanes.swimlane(name, color, label);
        Ok(())
    }

    fn instructions(&mut self) -> &mut Instructions {
        &mut self.swimlanes.instructions
    }

    fn current(&self) -> InstructionId {
        self.swimlanes.get_current()
    }

    fn set_current(&mut self, current: InstructionId) {
        self.swimlanes.set_current(current);
    }

    fn next_link_renderer(&self) -> LinkRendering {
        self.swimlanes.next_link_renderer().clone()
    }

    fn current_swimlane(&self) -> Option<SwimlaneId> {
        self.swimlanes.get_current_swimlane()
    }

    /// Adds the instruction where the current one takes the next; it stays out of the tree if refused.
    fn add(&mut self, instruction: Instruction) -> (InstructionId, CommandResult) {
        let current = self.current();
        let id = self.instructions().push(instruction);
        (id, self.instructions().add(current, id))
    }

    fn add_activity(
        &mut self,
        activity: Display,
        box_style: BoxStyle,
        url: Option<Url>,
        stereogroup: &Stereogroup,
    ) -> CommandResult {
        let colors = stereogroup.get_inner_colors()?;
        self.manage_swimlane_strategy();
        let instruction = InstructionSimple::new(
            activity,
            self.next_link_renderer(),
            self.current_swimlane(),
            box_style,
            url,
            colors,
            stereogroup.build_stereotype(),
            self.titled.skin.current_style_builder(),
        );
        self.add(Instruction::Simple(instruction)).1?;
        self.set_next_link_renderer_internal(LinkRendering::none());
        Ok(())
    }

    fn add_spot(&mut self, spot: &str, color: Option<HColor>) {
        let instruction = InstructionSpot {
            mono: MonoSwimable::new(self.current_swimlane()),
            killed: false,
            inlink_rendering: self.next_link_renderer(),
            spot: spot.to_owned(),
            color,
        };
        let _ = self.add(Instruction::Spot(instruction));
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.manage_swimlane_strategy();
    }

    fn add_goto(&mut self, name: &str) {
        let instruction = InstructionGoto {
            mono: MonoSwimable::new(self.current_swimlane()),
            name: name.to_owned(),
        };
        let _ = self.add(Instruction::Goto(instruction));
        self.set_next_link_renderer_internal(LinkRendering::none());
    }

    fn add_label(&mut self, name: &str) {
        let instruction = InstructionLabel {
            mono: MonoSwimable::new(self.current_swimlane()),
            name: name.to_owned(),
        };
        let _ = self.add(Instruction::Label(instruction));
        self.set_next_link_renderer_internal(LinkRendering::none());
    }

    fn start(&mut self, colors: Colors) {
        self.manage_swimlane_strategy();
        let instruction = InstructionStart {
            mono: MonoSwimable::new(self.current_swimlane()),
            inlink_rendering: self.next_link_renderer(),
            colors,
        };
        let _ = self.add(Instruction::Start(instruction));
        self.set_next_link_renderer_internal(LinkRendering::none());
    }

    fn stop(&mut self, colors: Colors) {
        self.manage_swimlane_strategy();
        let instruction = InstructionStop {
            mono: MonoSwimable::new(self.current_swimlane()),
            inlink_rendering: self.next_link_renderer(),
            colors,
        };
        self.add_stop_or_end(Instruction::Stop(instruction));
    }

    fn end(&mut self, colors: Colors) {
        self.manage_swimlane_strategy();
        let instruction = InstructionEnd {
            mono: MonoSwimable::new(self.current_swimlane()),
            inlink_rendering: self.next_link_renderer(),
            colors,
        };
        self.add_stop_or_end(Instruction::End(instruction));
    }

    fn add_stop_or_end(&mut self, special: Instruction) {
        let current = self.current();
        let id = self.instructions().push(special);
        if !self.manage_special_stop_end_after_end_while(id) {
            let _ = self.instructions().add(current, id);
        }
    }

    /// A `stop` or `end` right after a loop without `break` ends the loop's way out instead of following it.
    fn manage_special_stop_end_after_end_while(&mut self, special: InstructionId) -> bool {
        let Instruction::List(current) = self.swimlanes.instructions.get(self.current()) else {
            return false;
        };
        let Some(last) = current.get_last() else {
            return false;
        };
        if !matches!(self.swimlanes.instructions.get(last), Instruction::While(_))
            || self.swimlanes.instructions.contains_break(last)
        {
            return false;
        }
        if let Instruction::While(instruction_while) = self.instructions().get_mut(last) {
            instruction_while.set_special(special);
        }
        true
    }

    fn break_instruction(&mut self) {
        self.manage_swimlane_strategy();
        let instruction = InstructionBreak {
            mono: MonoSwimable::new(self.current_swimlane()),
            inlink_rendering: self.next_link_renderer(),
        };
        let _ = self.add(Instruction::Break(instruction));
    }

    fn fork(&mut self, colors: Colors) {
        self.manage_swimlane_strategy();
        let instruction = InstructionFork::new(
            self.current(),
            self.next_link_renderer(),
            self.current_swimlane(),
            colors,
        );
        let (id, _) = self.add(Instruction::Fork(instruction));
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(id);
    }

    fn fork_again(&mut self) -> CommandResult {
        let next = self.next_link_renderer();
        let swimlane = self.current_swimlane();
        let current = self.current();
        let Instruction::Fork(fork) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find fork"));
        };
        fork.manage_out_rendering(next, false);
        fork.fork_again(swimlane);
        self.set_next_link_renderer_internal(LinkRendering::none());
        Ok(())
    }

    fn end_fork(&mut self, fork_style: ForkStyle, label: Option<String>) -> CommandResult {
        let next = self.next_link_renderer();
        let swimlane = self.current_swimlane();
        let current = self.current();
        let Instruction::Fork(fork) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find fork"));
        };
        fork.set_style(fork_style, label, swimlane);
        fork.manage_out_rendering(next, true);
        let parent = fork.parent;
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(parent);
        Ok(())
    }

    fn split(&mut self) {
        let instruction = InstructionSplit::new(
            self.current(),
            self.next_link_renderer(),
            self.current_swimlane(),
        );
        self.set_next_link_renderer_internal(LinkRendering::none());
        let (id, _) = self.add(Instruction::Split(instruction));
        self.set_current(id);
    }

    fn split_again(&mut self) -> CommandResult {
        let next = self.next_link_renderer();
        let current = self.current();
        let Instruction::Split(split) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find split"));
        };
        split.split_again(next);
        self.set_next_link_renderer_internal(LinkRendering::none());
        Ok(())
    }

    fn end_split(&mut self) -> CommandResult {
        let next = self.next_link_renderer();
        let swimlane = self.current_swimlane();
        let current = self.current();
        let Instruction::Split(split) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find split"));
        };
        split.end_split(next, swimlane);
        let parent = split.parent;
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(parent);
        Ok(())
    }

    fn start_switch(&mut self, test: Option<Display>, colors: Colors) {
        self.manage_swimlane_strategy();
        let instruction = InstructionSwitch::new(
            self.current_swimlane(),
            self.current(),
            test,
            self.next_link_renderer(),
            colors,
        );
        let (id, _) = self.add(Instruction::Switch(instruction));
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(id);
    }

    fn switch_case(&mut self, label_case: Option<Display>) -> CommandResult {
        let next = self.next_link_renderer();
        let current = self.current();
        let Instruction::Switch(switch) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find switch"));
        };
        switch.switch_case(label_case, next);
        self.set_next_link_renderer_internal(LinkRendering::none());
        Ok(())
    }

    fn end_switch(&mut self, colors: Colors) -> CommandResult {
        let next = self.next_link_renderer();
        let current = self.current();
        let Instruction::Switch(switch) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find switch"));
        };
        switch.end_switch(next, colors);
        let parent = switch.parent;
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(parent);
        Ok(())
    }

    fn start_if(
        &mut self,
        test: Option<Display>,
        when_then: Option<Display>,
        color: Option<HColor>,
        url: Option<Url>,
        stereotype: Option<Stereotype>,
    ) {
        self.manage_swimlane_strategy();
        let instruction = InstructionIf::new(
            self.current_swimlane(),
            self.current(),
            test,
            LinkRendering::none().with_display(when_then),
            self.next_link_renderer(),
            color,
            self.titled.skin.current_style_builder(),
            url,
            stereotype,
        );
        let (id, _) = self.add(Instruction::If(instruction));
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(id);
    }

    fn else_if(
        &mut self,
        inlabel: LinkRendering,
        test: Option<Display>,
        when_then: LinkRendering,
        color: Option<HColor>,
    ) -> CommandResult {
        let next = self.next_link_renderer();
        let current = self.current();
        let Instruction::If(instruction_if) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find if"));
        };
        if !instruction_if.else_if(inlabel, test, when_then, next, color) {
            return Err(CommandError::new("You cannot put an elseIf here"));
        }
        self.set_next_link_renderer_internal(LinkRendering::none());
        Ok(())
    }

    fn else2(&mut self, when_else: LinkRendering) -> CommandResult {
        let next = self.next_link_renderer();
        let current = self.current();
        let Instruction::If(instruction_if) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find if"));
        };
        if !instruction_if.switch_to_else2(when_else, next) {
            return Err(CommandError::new("Cannot find if"));
        }
        self.set_next_link_renderer_internal(LinkRendering::none());
        Ok(())
    }

    fn endif(&mut self, colors: Colors) -> CommandResult {
        let next = self.next_link_renderer();
        let current = self.current();
        let Instruction::If(instruction_if) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find if"));
        };
        instruction_if.endif(next, colors);
        let parent = instruction_if.parent;
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(parent);
        Ok(())
    }

    fn start_repeat(
        &mut self,
        label: Option<Display>,
        box_style_in: BoxStyle,
        stereogroup: Stereogroup,
    ) {
        self.manage_swimlane_strategy();
        let instruction = InstructionRepeat::new(
            self.current_swimlane(),
            self.current(),
            self.next_link_renderer(),
            label,
            box_style_in,
            stereogroup,
            self.titled.skin.current_style_builder(),
        );
        let (id, _) = self.add(Instruction::Repeat(instruction));
        self.set_current(id);
        self.set_next_link_renderer_internal(LinkRendering::none());
    }

    fn repeat_while(
        &mut self,
        label: Option<Display>,
        yes: Option<Display>,
        out: Option<Display>,
        link_label: Option<Display>,
        link_color: Rainbow,
        stereogroup: Stereogroup,
    ) -> CommandResult {
        self.manage_swimlane_strategy();
        let next = self.next_link_renderer();
        let swimlane = self.current_swimlane();
        let current = self.current();
        let Instruction::Repeat(repeat) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find repeat"));
        };
        let back = LinkRendering::create(link_color).with_display(link_label);
        repeat.set_test(label, yes, out, next, back, swimlane, stereogroup);
        let parent = repeat.parent;
        self.set_current(parent);
        self.set_next_link_renderer_internal(LinkRendering::none());
        Ok(())
    }

    fn backward(
        &mut self,
        label: Display,
        box_style: BoxStyle,
        incoming1: LinkRendering,
        incoming2: LinkRendering,
        stereotype: Option<Stereotype>,
    ) -> CommandResult {
        self.manage_swimlane_strategy();
        let swimlane = self.current_swimlane();
        let current = self.current();
        match self.instructions().get_mut(current) {
            Instruction::Repeat(repeat) => {
                repeat.set_backward(label, swimlane, box_style, incoming1, incoming2, stereotype);
                Ok(())
            }
            Instruction::While(instruction_while) => {
                instruction_while.set_backward(label, box_style, incoming1, incoming2, stereotype);
                Ok(())
            }
            _ => Err(CommandError::new("Cannot find repeat")),
        }
    }

    fn do_while(&mut self, test: Display, yes: Option<Display>, color: Option<HColor>) {
        self.manage_swimlane_strategy();
        let instruction = InstructionWhile::new(
            self.current_swimlane(),
            self.current(),
            test,
            self.next_link_renderer(),
            yes,
            color,
            self.titled.skin.current_style_builder(),
        );
        let (id, _) = self.add(Instruction::While(instruction));
        self.set_current(id);
    }

    fn endwhile(&mut self, out: Option<Display>) -> CommandResult {
        let next = self.next_link_renderer();
        let current = self.current();
        let Instruction::While(instruction_while) = self.instructions().get_mut(current) else {
            return Err(CommandError::new("Cannot find while"));
        };
        instruction_while.incoming(&next);
        instruction_while.out_display(out);
        let parent = instruction_while.parent;
        self.set_next_link_renderer_internal(LinkRendering::none());
        self.set_current(parent);
        Ok(())
    }

    fn kill(&mut self) -> CommandResult {
        let current = self.current();
        if self.instructions().kill(current) {
            Ok(())
        } else {
            Err(CommandError::new("kill cannot be used here"))
        }
    }

    fn start_group(
        &mut self,
        name: Display,
        back_color: Option<HColor>,
        type_: USymbol,
        style: Style,
    ) {
        self.manage_swimlane_strategy();
        let instruction = InstructionGroup::new(
            self.current(),
            name,
            back_color,
            self.current_swimlane(),
            self.next_link_renderer(),
            type_,
            style,
        );
        let (id, _) = self.add(Instruction::Group(instruction));
        self.set_current(id);
    }

    fn close_group(&mut self) -> CommandResult {
        let Instruction::Group(group) = self.swimlanes.instructions.get(self.current()) else {
            return Err(CommandError::new("Cannot find group"));
        };
        self.set_current(group.parent);
        Ok(())
    }

    fn set_next_link_renderer_internal(&mut self, link: LinkRendering) {
        self.swimlanes.set_next_link_renderer(link);
    }

    /// A coloured arrow out of a loop or a conditional colours its way out too.
    fn set_next_link(&mut self, link_renderer: LinkRendering) {
        let current = self.current();
        if let Some(last) = self.swimlanes.instructions.get_last(current) {
            match self.instructions().get_mut(last) {
                Instruction::While(instruction_while) => {
                    instruction_while.out_color(link_renderer.rainbow.clone());
                }
                Instruction::If(instruction_if) => instruction_if.out_color(link_renderer.clone()),
                _ => {}
            }
        }
        self.set_next_link_renderer_internal(link_renderer);
    }

    /// The label of the next arrow; right after `while`, the label of the way into the loop.
    fn set_label_next_arrow(&mut self, label: Option<Display>) {
        let current = self.current();
        if let Instruction::While(instruction_while) = self.instructions().get_mut(current)
            && instruction_while.repeat_list.get_last().is_none()
        {
            instruction_while.overwrite_yes(label);
            return;
        }
        let link = self.next_link_renderer().with_display(label);
        self.set_next_link_renderer_internal(link);
    }

    fn set_color_next_arrow(&mut self, color: Rainbow) {
        self.set_next_link(LinkRendering::create(color));
    }

    fn add_note(
        &mut self,
        note: Display,
        position: NotePosition,
        type_: NoteType,
        colors: Colors,
        stereotype: Option<Stereotype>,
    ) {
        let note = PositionedNote {
            display: note,
            note_position: position,
            type_,
            colors,
            swimlane_note: self.current_swimlane(),
            stereotype,
        };
        let current = self.current();
        self.instructions().add_note(current, note);
    }
}

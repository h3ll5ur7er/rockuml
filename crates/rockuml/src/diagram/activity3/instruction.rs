//! The instructions an activity diagram is made of, kept by id in one arena so that the commands can
//! point at the instruction taking the next ones, and instructions at their parent (PlantUML's
//! `Instruction`, `InstructionList`, `WithNote`, `MonoSwimable` and `PositionedNote`).

use std::collections::BTreeSet;

use super::branch::{Branch, InstructionIf, InstructionSwitch};
use super::group::InstructionGroup;
use super::leaves::{
    InstructionBreak, InstructionEnd, InstructionGoto, InstructionLabel, InstructionSimple,
    InstructionSpot, InstructionStart, InstructionStop,
};
use super::link_rendering::LinkRendering;
use super::loops::{InstructionRepeat, InstructionWhile};
use super::parallel::{InstructionFork, InstructionSplit};
use super::swimlanes::SwimlaneId;
use crate::color::Colors;
use crate::command::{CommandError, CommandResult};
use crate::creole::Display;
use crate::diagram::sequence::model::NotePosition;
use crate::stereo::Stereotype;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct InstructionId(usize);

/// The swimlanes an instruction spans. A fork outside any lane spans no lane, `None`.
pub(crate) type SwimlaneSet = BTreeSet<Option<SwimlaneId>>;

pub(crate) enum Instruction {
    /// Only the root: the other lists belong to the instructions that hold them.
    List(InstructionList),
    If(InstructionIf),
    Switch(InstructionSwitch),
    While(InstructionWhile),
    Repeat(InstructionRepeat),
    Fork(InstructionFork),
    Split(InstructionSplit),
    Group(InstructionGroup),
    Simple(InstructionSimple),
    Spot(InstructionSpot),
    Start(InstructionStart),
    Stop(InstructionStop),
    End(InstructionEnd),
    Break(InstructionBreak),
    Goto(InstructionGoto),
    Label(InstructionLabel),
}

/// Every instruction of a diagram, the root list first. An instruction that could not be added stays here
/// unreachable, as it is lost in PlantUML.
pub(crate) struct Instructions {
    all: Vec<Instruction>,
}

static NONE: LinkRendering = LinkRendering::none();

impl Instructions {
    pub(crate) const ROOT: InstructionId = InstructionId(0);

    pub(crate) fn new() -> Self {
        Self {
            all: vec![Instruction::List(InstructionList::new(None))],
        }
    }

    pub(crate) fn get(&self, id: InstructionId) -> &Instruction {
        &self.all[id.0]
    }

    pub(crate) fn get_mut(&mut self, id: InstructionId) -> &mut Instruction {
        &mut self.all[id.0]
    }

    pub(crate) fn push(&mut self, instruction: Instruction) -> InstructionId {
        self.all.push(instruction);
        InstructionId(self.all.len() - 1)
    }

    /// Adds `child` where `container` takes its next instruction.
    pub(crate) fn add(&mut self, container: InstructionId, child: InstructionId) -> CommandResult {
        let list = match self.get_mut(container) {
            Instruction::List(list) => list,
            Instruction::If(ins) => &mut ins.current_mut().list,
            Instruction::Switch(ins) => match ins.switches.last_mut() {
                Some(current) => &mut current.list,
                None => return Err(CommandError::new("No 'case' in this switch")),
            },
            Instruction::While(ins) => &mut ins.repeat_list,
            Instruction::Repeat(ins) => &mut ins.repeat_list,
            Instruction::Fork(ins) => ins.get_last_list_mut(),
            Instruction::Split(ins) => ins.get_last_mut(),
            Instruction::Group(ins) => &mut ins.list,
            _ => unreachable!("only instructions holding others become current"),
        };
        list.all.push(child);
        Ok(())
    }

    /// Kills the flow after the last instruction, if it can be.
    pub(crate) fn kill(&mut self, id: InstructionId) -> bool {
        let last = match self.get_mut(id) {
            Instruction::List(list) => list.get_last(),
            Instruction::If(ins) if ins.endif_called => {
                // PlantUML checks the first branch only, and the else branch.
                let first = ins.thens[0].get_last();
                let otherwise = ins.else_branch.as_ref().and_then(Branch::get_last);
                return first.is_none_or(|last| self.kill(last))
                    && otherwise.is_none_or(|last| self.kill(last));
            }
            Instruction::If(ins) => ins.current().get_last(),
            Instruction::Switch(ins) => ins.switches.last().and_then(Branch::get_last),
            Instruction::While(ins) if ins.test_called => {
                ins.killed = true;
                return true;
            }
            Instruction::While(ins) => ins.repeat_list.get_last(),
            Instruction::Repeat(ins) if ins.test_called => {
                ins.killed = true;
                return true;
            }
            Instruction::Repeat(ins) => ins.repeat_list.get_last(),
            Instruction::Fork(ins) => ins.get_last_list().get_last(),
            Instruction::Split(ins) => ins.get_last().get_last(),
            Instruction::Group(ins) => ins.list.get_last(),
            Instruction::Simple(ins) => {
                ins.killed = true;
                return true;
            }
            Instruction::Spot(ins) => {
                ins.killed = true;
                return true;
            }
            Instruction::Start(_)
            | Instruction::Stop(_)
            | Instruction::End(_)
            | Instruction::Break(_)
            | Instruction::Goto(_)
            | Instruction::Label(_) => return false,
        };
        last.is_some_and(|last| self.kill(last))
    }

    /// Attaches the note to the instruction it is written after, or to `id` itself.
    pub(crate) fn add_note(&mut self, id: InstructionId, note: PositionedNote) {
        let handed_on = match self.get_mut(id) {
            Instruction::List(list) => list.add_note(note),
            Instruction::If(ins) if ins.endif_called || ins.current().is_empty() => {
                ins.notes.add_note(note)
            }
            Instruction::If(ins) => ins.current_mut().list.add_note(note),
            Instruction::Switch(ins) => match ins.switches.last_mut() {
                Some(current) if !current.is_empty() => current.list.add_note(note),
                _ => ins.notes.add_note(note),
            },
            Instruction::While(ins) if ins.repeat_list.is_empty() => ins.notes.add_note(note),
            Instruction::While(ins) => ins.repeat_list.add_note(note),
            Instruction::Repeat(ins) if ins.backward.is_none() => ins.repeat_list.add_note(note),
            Instruction::Repeat(ins) => {
                ins.backward_notes.push(note);
                None
            }
            Instruction::Fork(ins) if ins.finished => ins.notes.add_note(note),
            Instruction::Fork(ins) => ins.get_last_list_mut().add_note(note),
            Instruction::Split(ins) => ins.get_last_mut().add_note(note),
            Instruction::Group(ins) if ins.list.is_empty() => {
                ins.note = Some(note);
                None
            }
            Instruction::Group(ins) => ins.list.add_note(note),
            Instruction::Simple(ins) => ins.mono.notes.add_note(note),
            Instruction::Spot(ins) => ins.mono.notes.add_note(note),
            Instruction::Start(ins) => ins.mono.notes.add_note(note),
            Instruction::Stop(ins) => ins.mono.notes.add_note(note),
            Instruction::End(ins) => ins.mono.notes.add_note(note),
            Instruction::Break(ins) => ins.mono.notes.add_note(note),
            Instruction::Goto(ins) => ins.mono.notes.add_note(note),
            Instruction::Label(ins) => ins.mono.notes.add_note(note),
        };
        if let Some((last, note)) = handed_on {
            self.add_note(last, note);
        }
    }

    pub(crate) fn contains_break(&self, id: InstructionId) -> bool {
        match self.get(id) {
            Instruction::List(list) => self.list_contains_break(list),
            Instruction::If(ins) => self.lists_contain_break(
                ins.thens
                    .iter()
                    .chain(&ins.else_branch)
                    .map(|branch| &branch.list),
            ),
            Instruction::Switch(ins) => {
                self.lists_contain_break(ins.switches.iter().map(|branch| &branch.list))
            }
            Instruction::While(ins) => self.list_contains_break(&ins.repeat_list),
            Instruction::Repeat(ins) => self.list_contains_break(&ins.repeat_list),
            Instruction::Fork(ins) => self.lists_contain_break(&ins.forks),
            Instruction::Split(ins) => self.lists_contain_break(&ins.splits),
            Instruction::Group(ins) => self.list_contains_break(&ins.list),
            Instruction::Break(_) => true,
            Instruction::Simple(_)
            | Instruction::Spot(_)
            | Instruction::Start(_)
            | Instruction::Stop(_)
            | Instruction::End(_)
            | Instruction::Goto(_)
            | Instruction::Label(_) => false,
        }
    }

    pub(crate) fn list_contains_break(&self, list: &InstructionList) -> bool {
        list.all.iter().any(|ins| self.contains_break(*ins))
    }

    /// Whether the list is a lone killed activity, spot, or `stop` or `end` without notes
    /// (`InstructionList.isOnlySingleStopOrSpot`).
    pub(crate) fn list_is_only_single_stop_or_spot(&self, list: &InstructionList) -> bool {
        let [only] = list.all.as_slice() else {
            return false;
        };
        match self.get(*only) {
            Instruction::Simple(ins) => ins.killed,
            Instruction::Spot(_) => true,
            Instruction::Stop(ins) => ins.mono.notes.notes.is_empty(),
            Instruction::End(ins) => ins.mono.notes.notes.is_empty(),
            _ => false,
        }
    }

    fn lists_contain_break<'a>(
        &self,
        lists: impl IntoIterator<Item = &'a InstructionList>,
    ) -> bool {
        lists.into_iter().any(|list| self.list_contains_break(list))
    }

    /// How the arrow into the instruction is drawn.
    pub(crate) fn get_in_link_rendering(&self, id: InstructionId) -> &LinkRendering {
        match self.get(id) {
            Instruction::List(list) => list
                .all
                .first()
                .map_or(&NONE, |first| self.get_in_link_rendering(*first)),
            Instruction::If(ins) => &ins.top_inlink_rendering,
            Instruction::Switch(ins) => &ins.top_inlink_rendering,
            Instruction::While(ins) => &ins.next_link_renderer,
            Instruction::Repeat(ins) => &ins.next_link_renderer,
            Instruction::Fork(ins) => &ins.inlink_rendering,
            Instruction::Split(ins) => &ins.inlink_rendering,
            Instruction::Group(ins) => &ins.link_rendering,
            Instruction::Simple(ins) => &ins.inlink_rendering,
            Instruction::Spot(ins) => &ins.inlink_rendering,
            Instruction::Start(ins) => &ins.inlink_rendering,
            Instruction::Stop(ins) => &ins.inlink_rendering,
            Instruction::End(ins) => &ins.inlink_rendering,
            Instruction::Break(ins) => &ins.inlink_rendering,
            Instruction::Goto(_) | Instruction::Label(_) => &NONE,
        }
    }

    /// The last instruction of one that holds others in a row (`InstructionCollection.getLast`); none for
    /// the others.
    pub(crate) fn get_last(&self, id: InstructionId) -> Option<InstructionId> {
        match self.get(id) {
            Instruction::List(list) => list.get_last(),
            Instruction::If(ins) => ins.get_last(),
            Instruction::Switch(ins) => ins.switches.last().and_then(Branch::get_last),
            Instruction::While(ins) => ins.repeat_list.get_last(),
            Instruction::Group(ins) => ins.list.get_last(),
            _ => None,
        }
    }
}

/// Instructions one after the other.
pub(crate) struct InstructionList {
    pub(crate) all: Vec<InstructionId>,
    pub(crate) default_swimlane: Option<SwimlaneId>,
    /// How the arrow out of a branch of a fork or split is drawn.
    pub(crate) outlink_rendering: Option<LinkRendering>,
    pub(crate) notes: WithNote,
}

impl InstructionList {
    pub(crate) fn new(default_swimlane: Option<SwimlaneId>) -> Self {
        Self {
            all: Vec::new(),
            default_swimlane,
            outlink_rendering: None,
            notes: WithNote::default(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    pub(crate) fn get_last(&self) -> Option<InstructionId> {
        self.all.last().copied()
    }

    pub(crate) fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.default_swimlane
    }

    pub(crate) fn set_out_rendering(&mut self, outlink_rendering: LinkRendering) {
        self.outlink_rendering = Some(outlink_rendering);
    }

    /// Keeps the note while the list is empty; else hands it on to the last instruction.
    #[must_use]
    pub(crate) fn add_note(
        &mut self,
        note: PositionedNote,
    ) -> Option<(InstructionId, PositionedNote)> {
        match self.get_last() {
            Some(last) => Some((last, note)),
            None => self.notes.add_note(note),
        }
    }
}

/// The notes attached to an instruction.
#[derive(Default)]
pub(crate) struct WithNote {
    pub(crate) notes: Vec<PositionedNote>,
}

impl WithNote {
    /// Keeps the note, handing on nothing.
    #[must_use]
    pub(crate) fn add_note(
        &mut self,
        note: PositionedNote,
    ) -> Option<(InstructionId, PositionedNote)> {
        self.notes.push(note);
        None
    }
}

/// An instruction drawn in a single lane, with notes.
pub(crate) struct MonoSwimable {
    pub(crate) swimlane: Option<SwimlaneId>,
    pub(crate) notes: WithNote,
}

impl MonoSwimable {
    pub(crate) fn new(swimlane: Option<SwimlaneId>) -> Self {
        Self {
            swimlane,
            notes: WithNote::default(),
        }
    }
}

/// Whether a note sits beside the flow or floats next to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteType {
    Note,
    FloatingNote,
}

impl NoteType {
    /// `note` or `floating note`.
    pub(crate) fn default_type(name: &str) -> Self {
        if name.eq_ignore_ascii_case("floating note") {
            Self::FloatingNote
        } else {
            Self::Note
        }
    }
}

pub(crate) struct PositionedNote {
    pub(crate) display: Display,
    pub(crate) note_position: NotePosition,
    pub(crate) type_: NoteType,
    pub(crate) colors: Colors,
    pub(crate) swimlane_note: Option<SwimlaneId>,
    pub(crate) stereotype: Option<Stereotype>,
}

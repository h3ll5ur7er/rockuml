//! Building the tiles of instructions (PlantUML's `Instruction.createFtile` and `Branch.updateFtile`).
//! Building changes nothing: the tree is built anew each time the diagram is measured or drawn.

use std::rc::Rc;

use super::ForkStyle;
use super::branch::{Branch, BranchFtile, InstructionIf, InstructionSwitch};
use super::instruction::{Instruction, InstructionId, InstructionList, Instructions, WithNote};
use super::leaves::InstructionSimple;
use super::link_rendering::LinkRendering;
use super::loops::{InstructionRepeat, InstructionWhile};
use crate::color::Colors;
use crate::ftile::vcompact::{FtileWithNoteOpale, FtileWithNotes};
use crate::ftile::{
    Ftile, FtileBreak, FtileDecorateWelding, FtileEmpty, FtileFactory, FtileGoto, FtileKilled,
    FtileLabel, ftile_utils,
};
use crate::klimt::VerticalAlignment;

impl Instructions {
    /// The tile of the instruction, built with `factory`, the outermost of the chain.
    pub(crate) fn create_ftile(
        &self,
        id: InstructionId,
        factory: &dyn FtileFactory,
    ) -> Rc<dyn Ftile> {
        let skin_param = || factory.skin_param().clone();
        match self.get(id) {
            Instruction::List(list) => self.create_ftile_list(list, factory),
            Instruction::Simple(ins) => create_ftile_simple(ins, factory),
            Instruction::Spot(ins) => {
                let result = factory.spot(ins.mono.swimlane, &ins.spot, ins.color.clone());
                killed_if(ins.killed, with_notes(factory, result, &ins.mono.notes))
            }
            Instruction::Start(ins) => {
                let result = factory.start(ins.mono.swimlane, &ins.colors);
                with_notes(factory, result, &ins.mono.notes)
            }
            Instruction::Stop(ins) => {
                let result = factory.stop(ins.mono.swimlane, &ins.colors);
                with_notes(factory, result, &ins.mono.notes)
            }
            Instruction::End(ins) => {
                let result = factory.end(ins.mono.swimlane, &ins.colors);
                with_notes(factory, result, &ins.mono.notes)
            }
            Instruction::Break(ins) => FtileBreak::create(skin_param(), ins.mono.swimlane),
            Instruction::Goto(ins) => {
                Rc::new(FtileGoto::new(skin_param(), ins.mono.swimlane, &ins.name))
            }
            Instruction::Label(ins) => {
                Rc::new(FtileLabel::new(skin_param(), ins.mono.swimlane, &ins.name))
            }
            Instruction::If(ins) => self.create_ftile_if(ins, factory),
            Instruction::Switch(ins) => self.create_ftile_switch(ins, factory),
            Instruction::While(ins) => self.create_ftile_while(ins, factory),
            Instruction::Repeat(ins) => self.create_ftile_repeat(id, ins, factory),
            Instruction::Fork(ins) => {
                let all = self.create_ftile_lists(&ins.forks, factory);
                let result = factory.create_parallel(
                    all,
                    ins.style,
                    ins.label.as_deref(),
                    ins.swimlane_in,
                    ins.swimlane_out,
                    &ins.colors,
                );
                with_notes_beside(result, &ins.notes)
            }
            Instruction::Split(ins) => {
                let all = self.create_ftile_lists(&ins.splits, factory);
                factory.create_parallel(
                    all,
                    ForkStyle::Split,
                    None,
                    ins.swimlane_in,
                    ins.swimlane_out,
                    &Colors::default(),
                )
            }
            Instruction::Group(ins) => {
                let mut tmp = self.create_ftile_list(&ins.list, factory);
                if let Some(note) = &ins.note {
                    tmp = Rc::new(FtileWithNotes::new(
                        tmp,
                        std::slice::from_ref(note),
                        VerticalAlignment::Center,
                    ));
                }
                factory.create_group(
                    tmp,
                    &ins.title,
                    ins.back_color.clone(),
                    None,
                    ins.type_,
                    &ins.style,
                )
            }
        }
    }

    /// The tile of instructions one after the other, the arrow into each drawn as the instruction says; an
    /// empty list is an empty tile (`InstructionList.createFtile`).
    pub(crate) fn create_ftile_list(
        &self,
        list: &InstructionList,
        factory: &dyn FtileFactory,
    ) -> Rc<dyn Ftile> {
        let mut breaks = Vec::new();
        // An empty list has no tile, not even for its notes.
        let notes = (!list.is_empty() && !list.notes.notes.is_empty()).then(|| {
            factory.add_note(
                None,
                list.get_swimlane_in(),
                &list.notes.notes,
                VerticalAlignment::Center,
            )
        });
        let tiles = list.all.iter().map(|ins| {
            let cur = self.create_ftile(*ins, factory);
            breaks.extend(cur.get_welding_points());
            let in_link_rendering = self.get_in_link_rendering(*ins);
            if in_link_rendering.is_none() {
                return cur;
            }
            factory.decorate_in(cur, in_link_rendering)
        });
        let Some(mut result) = notes
            .into_iter()
            .chain(tiles)
            .reduce(|result, cur| factory.assembly(result, cur))
        else {
            let result = Rc::new(FtileEmpty::new(
                factory.skin_param().clone(),
                list.default_swimlane,
            ));
            // PlantUML decorates the way into an empty list with its way out, on purpose.
            return match &list.outlink_rendering {
                Some(outlink_rendering) => factory.decorate_in(result, outlink_rendering),
                None => result,
            };
        };
        if let Some(outlink_rendering) = &list.outlink_rendering {
            result = factory.decorate_out(result, outlink_rendering);
        }
        if !breaks.is_empty() {
            result = Rc::new(FtileDecorateWelding::new(result, breaks));
        }
        result
    }

    fn create_ftile_lists(
        &self,
        lists: &[InstructionList],
        factory: &dyn FtileFactory,
    ) -> Vec<Rc<dyn Ftile>> {
        lists
            .iter()
            .map(|list| self.create_ftile_list(list, factory))
            .collect()
    }

    /// The branch's list, its way out drawn as the branch says (`Branch.updateFtile`).
    fn update_ftile<'a>(&self, branch: &'a Branch, factory: &dyn FtileFactory) -> BranchFtile<'a> {
        let ftile = factory.decorate_out(
            self.create_ftile_list(&branch.list, factory),
            &branch.inlink_rendering,
        );
        BranchFtile { branch, ftile }
    }

    fn create_ftile_if(&self, ins: &InstructionIf, factory: &dyn FtileFactory) -> Rc<dyn Ftile> {
        let thens: Vec<BranchFtile<'_>> = ins
            .thens
            .iter()
            .map(|branch| self.update_ftile(branch, factory))
            .collect();
        let empty_else;
        let else_branch = if let Some(else_branch) = &ins.else_branch {
            else_branch
        } else {
            empty_else = Branch::new(
                ins.swimlane,
                LinkRendering::none(),
                None,
                None,
                LinkRendering::none(),
            );
            &empty_else
        };
        let else_branch = self.update_ftile(else_branch, factory);
        let result = factory.create_if(
            self,
            ins.swimlane,
            &thens,
            &else_branch,
            &ins.out_color,
            &ins.top_inlink_rendering,
            ins.url.as_ref(),
            &ins.notes.notes,
            ins.stereotype.as_ref(),
            &ins.current_style_builder,
        );
        let welding_points: Vec<Rc<dyn Ftile>> = thens
            .iter()
            .chain([&else_branch])
            .flat_map(|branch| branch.ftile.get_welding_points())
            .collect();
        if welding_points.is_empty() {
            return result;
        }
        Rc::new(FtileDecorateWelding::new(result, welding_points))
    }

    fn create_ftile_switch(
        &self,
        ins: &InstructionSwitch,
        factory: &dyn FtileFactory,
    ) -> Rc<dyn Ftile> {
        let branches: Vec<BranchFtile<'_>> = ins
            .switches
            .iter()
            .map(|branch| self.update_ftile(branch, factory))
            .collect();
        let end_colors = match &ins.end_colors {
            Some(end_colors) => ins.colors.merge_with(end_colors),
            None => ins.colors.clone(),
        };
        let result = factory.create_switch(
            self,
            ins.swimlane,
            &branches,
            &LinkRendering::none(),
            &ins.top_inlink_rendering,
            ins.label_test.as_ref(),
            &ins.colors,
            Some(&end_colors),
        );
        eventually_add_note(
            factory,
            result,
            ins.swimlane,
            &ins.notes,
            VerticalAlignment::Top,
        )
    }

    fn create_ftile_while(
        &self,
        ins: &InstructionWhile,
        factory: &dyn FtileFactory,
    ) -> Rc<dyn Ftile> {
        let back = match (&ins.backward, ins.box_style) {
            (Some(backward), Some(box_style)) => Some(factory.activity(
                backward,
                ins.swimlane,
                box_style,
                &Colors::default(),
                ins.stereotype.as_ref(),
                &factory.skin_param().current_style_builder(),
            )),
            _ => None,
        };
        let tmp = self.create_ftile_list(&ins.repeat_list, factory);
        let tmp = factory.create_while(
            self,
            &ins.out_color,
            ins.swimlane,
            tmp,
            &ins.test,
            ins.yes.as_ref(),
            ins.color.clone(),
            ins.special_out,
            back,
            &ins.incoming1,
            &ins.incoming2,
            &ins.current_style_builder,
        );
        let tmp = with_notes_beside(tmp, &ins.notes);
        killed_if(ins.killed || ins.special_out.is_some(), tmp)
    }

    fn create_ftile_repeat(
        &self,
        id: InstructionId,
        ins: &InstructionRepeat,
        factory: &dyn FtileFactory,
    ) -> Rc<dyn Ftile> {
        let back = get_ftile_backward(ins, factory);
        let tmp = self.create_ftile_list(&ins.repeat_list, factory);
        let tmp = factory.decorate_out(tmp, &ins.end_repeat_link_rendering);
        let tmp = factory.repeat(
            &ins.stereogroup_loop,
            &ins.stereotype2,
            ins.box_style_in,
            ins.swimlane,
            ins.swimlane_out,
            ins.start_label.as_ref(),
            tmp,
            ins.test.as_ref(),
            ins.yes.as_ref(),
            ins.out.as_ref(),
            back,
            self.is_last_of_the_parent(id, ins.parent),
            &ins.incoming1,
            &ins.incoming2,
            &ins.current_style_builder,
        );
        let tmp = ftile_utils::with_swimlane_in(tmp, ins.swimlane);
        killed_if(ins.killed, tmp)
    }

    /// Whether the instruction ends the diagram's main list (`isLastOfTheParent`).
    fn is_last_of_the_parent(&self, id: InstructionId, parent: InstructionId) -> bool {
        matches!(self.get(parent), Instruction::List(_)) && self.get_last(parent) == Some(id)
    }
}

fn create_ftile_simple(ins: &InstructionSimple, factory: &dyn FtileFactory) -> Rc<dyn Ftile> {
    let mut result = factory.activity(
        &ins.label,
        ins.mono.swimlane,
        ins.box_style,
        &ins.colors,
        ins.stereotype.as_ref(),
        &ins.style_builder,
    );
    if let Some(url) = &ins.url {
        result = factory.add_url(result, url);
    }
    killed_if(ins.killed, with_notes(factory, result, &ins.mono.notes))
}

/// The activity going back up a `repeat` loop, with its notes (`InstructionRepeat.getFtileBackward`).
fn get_ftile_backward(
    ins: &InstructionRepeat,
    factory: &dyn FtileFactory,
) -> Option<Rc<dyn Ftile>> {
    let (Some(backward), Some(box_style)) = (&ins.backward, ins.box_style) else {
        return None;
    };
    let result = factory.activity(
        backward,
        ins.swimlane_backward,
        box_style,
        &Colors::default(),
        ins.stereotype_back.as_ref(),
        &factory.skin_param().current_style_builder(),
    );
    if ins.backward_notes.is_empty() {
        return Some(result);
    }
    Some(factory.add_note(
        Some(result),
        ins.swimlane_backward,
        &ins.backward_notes,
        VerticalAlignment::Center,
    ))
}

/// The tile of a single instruction with its notes, in the tile's lane.
fn with_notes(factory: &dyn FtileFactory, ftile: Rc<dyn Ftile>, notes: &WithNote) -> Rc<dyn Ftile> {
    let swimlane = ftile.get_swimlane_in();
    eventually_add_note(factory, ftile, swimlane, notes, VerticalAlignment::Center)
}

/// The tile with the notes beside it, if there are any (`WithNote.eventuallyAddNote`).
fn eventually_add_note(
    factory: &dyn FtileFactory,
    ftile: Rc<dyn Ftile>,
    swimlane: Option<super::SwimlaneId>,
    notes: &WithNote,
    vertical_alignment: VerticalAlignment,
) -> Rc<dyn Ftile> {
    if notes.notes.is_empty() {
        return ftile;
    }
    factory.add_note(Some(ftile), swimlane, &notes.notes, vertical_alignment)
}

/// The tile of a fork or a loop with the notes written right after it beside it, pointing at nothing.
fn with_notes_beside(ftile: Rc<dyn Ftile>, notes: &WithNote) -> Rc<dyn Ftile> {
    if notes.notes.is_empty() {
        return ftile;
    }
    FtileWithNoteOpale::create(ftile, &notes.notes, false, VerticalAlignment::Center)
}

fn killed_if(killed: bool, ftile: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
    if killed {
        return Rc::new(FtileKilled::new(ftile));
    }
    ftile
}

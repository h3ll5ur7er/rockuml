//! Lays out composite states that no link crosses on their own, deepest first, and turns them into leaves
//! drawn by their layout (PlantUML's `CucaDiagramSimplifierStateSmetana`).

use std::rc::Rc;

use super::group_maker_state_smetana::GroupMakerStateSmetana;
use crate::abel::{EntityId, GroupType, LeafType};
use crate::diagram::NotYetPorted;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;

pub(crate) struct CucaDiagramSimplifierStateSmetana;

impl CucaDiagramSimplifierStateSmetana {
    pub(crate) fn simplify(
        diagram: &mut CucaDiagram,
        string_bounder: &dyn StringBounder,
    ) -> Result<(), NotYetPorted> {
        loop {
            let mut changed = false;
            for g in get_ordered(diagram) {
                if !diagram.entity(g).is_autarkic(diagram) {
                    continue;
                }
                let image = GroupMakerStateSmetana::new(diagram, g, string_bounder).get_image()?;
                let leaf_type = if diagram.entity(g).get_group_type() == GroupType::ConcurrentState
                {
                    LeafType::StateConcurrent
                } else {
                    LeafType::State
                };
                diagram.override_image(g, Rc::from(image), leaf_type);
                changed = true;
            }
            if !changed {
                return Ok(());
            }
        }
    }
}

/// Every group below the root, the deepest first: groups are collected level by level, the children of
/// each in reverse order, and the whole list is reversed.
fn get_ordered(diagram: &CucaDiagram) -> Vec<EntityId> {
    let root = diagram.get_root_group();
    let mut ordered = vec![root];
    loop {
        let size = ordered.len();
        for g in ordered.clone() {
            for child in diagram.entity(g).groups(diagram).into_iter().rev() {
                if !ordered.contains(&child) {
                    ordered.push(child);
                }
            }
        }
        if size == ordered.len() {
            break;
        }
    }
    ordered.into_iter().rev().filter(|g| *g != root).collect()
}

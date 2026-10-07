//! Whether a link stays inside a group (PlantUML's `EntityUtils`).

use super::{Entity, EntityId, Link};
use crate::diagram::cuca::CucaDiagram;

/// Whether `group_to_be_tested` is `parent_group` or a group inside it.
fn is_parent(
    group_to_be_tested: Option<EntityId>,
    parent_group: EntityId,
    diagram: &CucaDiagram,
) -> bool {
    let mut current = group_to_be_tested;
    while let Some(group) = current.map(|id| diagram.entity(id)) {
        if !group.is_group() || group.is_root() {
            return false;
        }
        if group.id() == parent_group {
            return true;
        }
        current = group.get_parent_container(diagram);
    }
    false
}

fn parents_inside(group: &Entity, link: &Link, diagram: &CucaDiagram) -> (bool, bool) {
    assert!(group.is_group(), "links are inside groups only");
    let parent = |id: EntityId| diagram.entity(id).get_parent_container(diagram);
    (
        is_parent(parent(link.get_entity1()), group.id(), diagram),
        is_parent(parent(link.get_entity2()), group.id(), diagram),
    )
}

/// Whether both ends of the link are inside `group`.
pub(crate) fn is_pure_inner_link12(group: &Entity, link: &Link, diagram: &CucaDiagram) -> bool {
    let (inside1, inside2) = parents_inside(group, link, diagram);
    inside1 && inside2
}

/// Whether the link does not cross the border of `group`: both ends inside, or both outside.
pub(crate) fn is_pure_inner_link3(group: &Entity, link: &Link, diagram: &CucaDiagram) -> bool {
    let (inside1, inside2) = parents_inside(group, link, diagram);
    inside1 == inside2
}

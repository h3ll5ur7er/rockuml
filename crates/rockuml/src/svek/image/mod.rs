//! The drawings of single entities, which the layout places as nodes (PlantUML's `svek.image` package).

mod circle_end;
mod circle_start;
mod entity_image_branch;
mod entity_image_circle_end;
mod entity_image_circle_start;
mod entity_image_pseudo_state;
mod entity_image_state;
mod entity_image_state2;
mod entity_image_state_border;
mod entity_image_state_common;
mod entity_image_state_empty_description;
mod entity_image_synchro_bar;

pub(crate) use entity_image_branch::EntityImageBranch;
pub(crate) use entity_image_circle_end::EntityImageCircleEnd;
pub(crate) use entity_image_circle_start::EntityImageCircleStart;
pub(crate) use entity_image_pseudo_state::EntityImagePseudoState;
pub(crate) use entity_image_state::EntityImageState;
pub(crate) use entity_image_state_border::EntityImageStateBorder;
pub(crate) use entity_image_state_common::{get_state_description, get_style_state};
pub(crate) use entity_image_state_empty_description::EntityImageStateEmptyDescription;
pub(crate) use entity_image_state2::EntityImageState2;
pub(crate) use entity_image_synchro_bar::EntityImageSynchroBar;

use crate::abel::Entity;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::group::{UGroup, UGroupType};
use crate::text::LineLocation;

/// The group SVG puts an entity's shapes in, named after the entity; `class` tells its kind.
pub(crate) fn entity_group(
    entity: &Entity,
    diagram: &CucaDiagram,
    class: &str,
    location: Option<&LineLocation>,
) -> UGroup {
    let name = entity.get_name(diagram);
    let mut group = UGroup::at(location);
    group.put(UGroupType::Class, class);
    group.put(UGroupType::Id, &format!("entity_{name}"));
    group.put(UGroupType::DataEntity, name);
    group.put(UGroupType::DataUid, entity.get_uid());
    group.put(
        UGroupType::DataQualifiedName,
        diagram.quark(entity.get_quark()).get_qualified_name(),
    );
    group
}

//! The drawings of single entities, which the layout places as nodes (PlantUML's `svek.image` package).

mod circle_end;
mod circle_start;
mod entity_image_branch;
mod entity_image_circle_end;
mod entity_image_circle_start;
mod entity_image_pseudo_state;
mod entity_image_state;
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
pub(crate) use entity_image_state_empty_description::EntityImageStateEmptyDescription;
pub(crate) use entity_image_synchro_bar::EntityImageSynchroBar;

use super::IEntityImage;
use crate::abel::{Entity, LeafType};
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

/// The image of a leaf of a state diagram (the state arms of `GeneralImageBuilder.createEntityImageBlock`);
/// `None` for leaves of other kinds.
pub(crate) fn create_state_entity_image(
    leaf: &Entity,
    diagram: &CucaDiagram,
    hide_empty_description: bool,
) -> Option<Box<dyn IEntityImage>> {
    let image: Box<dyn IEntityImage> = match leaf.get_leaf_type()? {
        LeafType::State if !leaf.get_entity_position().is_normal() => {
            Box::new(EntityImageStateBorder::new(leaf, diagram))
        }
        LeafType::State if hide_empty_description && leaf.bodier.get_raw_body().is_empty() => {
            Box::new(EntityImageStateEmptyDescription::new(leaf, diagram))
        }
        LeafType::State => Box::new(EntityImageState::new(leaf, diagram)),
        LeafType::CircleStart => Box::new(EntityImageCircleStart::new(leaf, diagram)),
        LeafType::CircleEnd => Box::new(EntityImageCircleEnd::new(leaf, diagram)),
        LeafType::Branch | LeafType::StateChoice => Box::new(EntityImageBranch::new(leaf, diagram)),
        LeafType::SynchroBar | LeafType::StateForkJoin => {
            Box::new(EntityImageSynchroBar::new(leaf, diagram))
        }
        LeafType::PseudoState => Box::new(EntityImagePseudoState::new(leaf, diagram)),
        LeafType::DeepHistory => Box::new(EntityImagePseudoState::deep_history(leaf, diagram)),
        _ => return None,
    };
    Some(image)
}

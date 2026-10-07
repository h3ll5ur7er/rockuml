//! The drawings of single entities, which the layout places as nodes (PlantUML's `svek.image` package).

mod entity_image_description;
mod entity_image_port;

pub(crate) use entity_image_description::EntityImageDescription;
pub(crate) use entity_image_port::EntityImagePort;

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

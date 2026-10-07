//! The images of entities (PlantUML's `svek.image` package).

mod association;
mod class;
mod entity_image_note;
mod entity_image_note_link;
mod entity_image_tips;
mod json;
mod lollipop;
mod map;
mod object;
mod opale;

pub(crate) use association::{EntityImageAssociation, EntityImageAssociationPoint};
pub(crate) use class::EntityImageClass;
pub(crate) use entity_image_note::{EntityImageNote, OpaleLink};
pub(crate) use entity_image_note_link::EntityImageNoteLink;
pub(crate) use entity_image_tips::EntityImageTips;
pub(crate) use json::EntityImageJson;
pub(crate) use lollipop::EntityImageLollipopInterface;
pub(crate) use map::EntityImageMap;
pub(crate) use object::EntityImageObject;
pub(crate) use opale::{get_corner, get_polygon_normal};

use crate::abel::Entity;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::shape::{URectangle, USegment, UShape};

/// The group an entity's drawing is in, which SVG names after the entity.
fn entity_group(entity: &Entity, diagram: &CucaDiagram) -> UGroup {
    let name = entity.get_name(diagram);
    let mut group = UGroup::at(entity.get_location());
    group.put(UGroupType::Class, "entity");
    group.put(UGroupType::Id, &format!("entity_{name}"));
    group.put(UGroupType::DataEntity, name);
    group.put(UGroupType::DataUid, entity.get_uid());
    group.put(
        UGroupType::DataQualifiedName,
        diagram.quark(entity.get_quark()).get_qualified_name(),
    );
    group
}

/// A rectangle whose top corners only are rounded (`URectangle.halfRounded`).
fn half_rounded(width: f64, height: f64, round_corner: f64) -> UShape {
    if round_corner == 0.0 {
        return UShape::Rectangle(URectangle::new(width, height));
    }
    let r = round_corner / 2.0;
    let arc = |end| USegment::ArcTo {
        radius: (r, r),
        x_axis_rotation: 0.0,
        large_arc: false,
        sweep: true,
        end,
    };
    UShape::Path(vec![
        USegment::MoveTo(r, 0.0),
        USegment::LineTo(width - r, 0.0),
        arc((width, r)),
        USegment::LineTo(width, height),
        USegment::LineTo(0.0, height),
        USegment::LineTo(0.0, r),
        arc((r, 0.0)),
    ])
}

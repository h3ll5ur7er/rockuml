//! The image each kind of leaf is drawn with (PlantUML's `GeneralImageBuilder`).

use super::image::{
    EntityImageAssociation, EntityImageAssociationPoint, EntityImageChenAttribute,
    EntityImageChenCircle, EntityImageChenEntity, EntityImageChenRelationship, EntityImageClass,
    EntityImageJson, EntityImageLollipopInterface, EntityImageMap, EntityImageNote,
    EntityImageObject, EntityImageTips,
};
use super::{Bibliotekon, IEntityImage};
use crate::abel::{EntityId, LeafType};
use crate::diagram::NotYetPorted;
use crate::diagram::cuca::CucaDiagram;

/// The image of `leaf`, a leaf the layout draws. Kinds of leaves whose image is not ported yet are reported
/// as such.
///
/// # Panics
///
/// If the leaf is removed: removed leaves are not laid out.
pub(crate) fn create_entity_image_block(
    leaf: EntityId,
    diagram: &CucaDiagram,
    _bibliotekon: &Bibliotekon,
) -> Result<Box<dyn IEntityImage>, NotYetPorted> {
    let entity = diagram.entity(leaf);
    assert!(!entity.is_removed(diagram), "removed leaves are not drawn");
    let not_ported = |image: &'static str| Err(NotYetPorted(image));
    let Some(leaf_type) = entity.get_leaf_type() else {
        return not_ported("EntityImageGroup");
    };
    if leaf_type.is_like_class() {
        return Ok(Box::new(EntityImageClass::new(entity, diagram)));
    }
    match leaf_type {
        LeafType::Note => Ok(Box::new(EntityImageNote::new(entity, diagram))),
        LeafType::Activity => not_ported("EntityImageActivity"),
        LeafType::Portin | LeafType::Portout => not_ported("EntityImagePort"),
        LeafType::State => not_ported("EntityImageState"),
        LeafType::CircleStart => not_ported("EntityImageCircleStart"),
        LeafType::CircleEnd => not_ported("EntityImageCircleEnd"),
        LeafType::Branch | LeafType::StateChoice => not_ported("EntityImageBranch"),
        LeafType::LollipopFull | LeafType::LollipopHalf => Ok(Box::new(
            EntityImageLollipopInterface::new(entity, diagram),
        )),
        LeafType::Object => Ok(Box::new(EntityImageObject::new(entity, diagram))),
        LeafType::Map => Ok(Box::new(EntityImageMap::new(entity, diagram))),
        LeafType::Json => Ok(Box::new(EntityImageJson::new(entity, diagram))),
        LeafType::PointForAssociation => {
            Ok(Box::new(EntityImageAssociationPoint::new(entity, diagram)))
        }
        LeafType::Association => Ok(Box::new(EntityImageAssociation::new(entity, diagram))),
        LeafType::Circle
        | LeafType::Description
        | LeafType::Usecase
        | LeafType::UsecaseBusiness => not_ported("EntityImageDescription"),
        LeafType::SynchroBar | LeafType::StateForkJoin => not_ported("EntityImageSynchroBar"),
        LeafType::ArcCircle => not_ported("EntityImageArcCircle"),
        LeafType::EmptyPackage => {
            if entity.get_usymbol().is_some() {
                not_ported("EntityImageDescription")
            } else {
                not_ported("EntityImageEmptyPackage")
            }
        }
        LeafType::PseudoState => not_ported("EntityImagePseudoState"),
        LeafType::StateTransitionLabel => not_ported("EntityImageTransitionLabel"),
        LeafType::DeepHistory => not_ported("EntityImageDeepHistory"),
        LeafType::Tips => Ok(Box::new(EntityImageTips::new(entity, diagram))),
        LeafType::ChenEntity => Ok(Box::new(EntityImageChenEntity::new(entity, diagram))),
        LeafType::ChenRelationship => {
            Ok(Box::new(EntityImageChenRelationship::new(entity, diagram)))
        }
        LeafType::ChenAttribute => Ok(Box::new(EntityImageChenAttribute::new(entity, diagram))),
        LeafType::ChenCircle => Ok(Box::new(EntityImageChenCircle::new(entity, diagram))),
        LeafType::Domain | LeafType::Requirement => not_ported("EntityImageDomain"),
        // PlantUML has no image for the other kinds and fails.
        _ => not_ported("GeneralImageBuilder"),
    }
}

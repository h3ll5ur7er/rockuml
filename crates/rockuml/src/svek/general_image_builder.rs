//! The image each kind of leaf is drawn with (PlantUML's `GeneralImageBuilder`).

use super::image::{
    EntityImageBranch, EntityImageChenAttribute, EntityImageChenCircle, EntityImageChenEntity,
    EntityImageChenRelationship, EntityImageCircleEnd, EntityImageCircleStart,
    EntityImageDescription, EntityImageNote, EntityImagePort, EntityImagePseudoState,
    EntityImageState, EntityImageState2, EntityImageStateBorder, EntityImageStateEmptyDescription,
    EntityImageSynchroBar, EntityImageTips,
};
use super::{Bibliotekon, IEntityImage};
use crate::abel::{Entity, EntityId, LeafType};
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
        return not_ported("EntityImageClass");
    }
    match leaf_type {
        LeafType::Note => Ok(Box::new(EntityImageNote::new(entity, diagram))),
        LeafType::Activity => not_ported("EntityImageActivity"),
        LeafType::Portin | LeafType::Portout => Ok(Box::new(EntityImagePort::new(entity, diagram))),
        LeafType::State => Ok(state_image(entity, diagram)),
        LeafType::CircleStart => Ok(Box::new(EntityImageCircleStart::new(entity, diagram))),
        LeafType::CircleEnd => Ok(Box::new(EntityImageCircleEnd::new(entity, diagram))),
        LeafType::Branch | LeafType::StateChoice => {
            Ok(Box::new(EntityImageBranch::new(entity, diagram)))
        }
        LeafType::LollipopFull | LeafType::LollipopHalf => {
            not_ported("EntityImageLollipopInterface")
        }
        LeafType::Circle
        | LeafType::Description
        | LeafType::Usecase
        | LeafType::UsecaseBusiness => Ok(Box::new(EntityImageDescription::new(entity, diagram))),
        LeafType::Object => not_ported("EntityImageObject"),
        LeafType::Map => not_ported("EntityImageMap"),
        LeafType::Json => not_ported("EntityImageJson"),
        LeafType::SynchroBar | LeafType::StateForkJoin => {
            Ok(Box::new(EntityImageSynchroBar::new(entity, diagram)))
        }
        LeafType::ArcCircle => not_ported("EntityImageArcCircle"),
        LeafType::PointForAssociation => not_ported("EntityImageAssociationPoint"),
        LeafType::EmptyPackage => {
            if entity.get_usymbol().is_some() {
                Ok(Box::new(EntityImageDescription::new(entity, diagram)))
            } else {
                not_ported("EntityImageEmptyPackage")
            }
        }
        LeafType::Association => not_ported("EntityImageAssociation"),
        LeafType::PseudoState => Ok(Box::new(EntityImagePseudoState::new(entity, diagram))),
        LeafType::StateTransitionLabel => not_ported("EntityImageTransitionLabel"),
        LeafType::DeepHistory => Ok(Box::new(EntityImagePseudoState::deep_history(
            entity, diagram,
        ))),
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

/// A state on its composite's border, without description under `hide empty description`, framed for
/// `<<sdlreceive>>`, or else a plain state.
fn state_image(entity: &Entity, diagram: &CucaDiagram) -> Box<dyn IEntityImage> {
    if !entity.get_entity_position().is_normal() {
        return Box::new(EntityImageStateBorder::new(entity, diagram));
    }
    if diagram.is_hide_empty_description_for_state() && entity.bodier.get_raw_body().is_empty() {
        return Box::new(EntityImageStateEmptyDescription::new(entity, diagram));
    }
    let sdl_receive = entity
        .stereotype
        .as_ref()
        .is_some_and(|stereotype| stereotype.label_double_comparator() == "<<sdlreceive>>");
    if sdl_receive {
        return Box::new(EntityImageState2::new(entity, diagram));
    }
    Box::new(EntityImageState::new(entity, diagram))
}

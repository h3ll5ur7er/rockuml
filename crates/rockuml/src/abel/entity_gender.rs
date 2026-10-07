//! Which entities a `hide` or `show` command is about (PlantUML's `EntityGender` and `EntityGenderUtils`).

use super::{Entity, EntityId, LeafType};
use crate::diagram::cuca::CucaDiagram;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EntityGender {
    ByEntityType(LeafType),
    ByEntityAlone(EntityId),
    /// A label as written, like `<<Serializable>>`.
    ByStereotype(String),
    /// The entities right inside a group other than the root.
    ByPackage(EntityId),
    And(Box<EntityGender>, Box<EntityGender>),
    All,
    ByClassName(String),
    /// Classes that show no method.
    EmptyMethods,
    /// Classes that show no field.
    EmptyFields,
}

impl EntityGender {
    pub(crate) fn contains(&self, test: &Entity, diagram: &CucaDiagram) -> bool {
        match self {
            Self::ByEntityType(leaf_type) => test.get_leaf_type() == Some(*leaf_type),
            Self::ByEntityAlone(entity) => test.id() == *entity,
            Self::ByStereotype(stereotype) => test.stereotype.as_ref().is_some_and(|test| {
                test.labels_double_comparator()
                    .iter()
                    .any(|label| label == stereotype)
            }),
            Self::ByPackage(group) => test
                .get_parent_container(diagram)
                .is_some_and(|parent| !diagram.entity(parent).is_root() && parent == *group),
            Self::And(gender1, gender2) => {
                gender1.contains(test, diagram) && gender2.contains(test, diagram)
            }
            Self::All => true,
            Self::ByClassName(class_name) => class_name == test.get_name(diagram),
            Self::EmptyMethods => test
                .bodier
                .get_methods_to_display(diagram.get_hides_visibility_modifier())
                .is_empty(),
            Self::EmptyFields => test
                .bodier
                .get_fields_to_display(diagram.get_hides_visibility_modifier())
                .is_empty(),
        }
    }

    /// What the gender names, which `hide <<stereotype>> stereotype` compares stereotype labels with.
    pub(crate) fn get_gender(&self, diagram: &CucaDiagram) -> Option<String> {
        match self {
            Self::ByEntityType(leaf_type) => Some(leaf_type.name().to_owned()),
            Self::ByEntityAlone(entity) => Some(diagram.entity(*entity).get_uid().to_owned()),
            Self::ByStereotype(name) | Self::ByClassName(name) => Some(name.clone()),
            Self::ByPackage(_)
            | Self::And(..)
            | Self::All
            | Self::EmptyMethods
            | Self::EmptyFields => None,
        }
    }
}

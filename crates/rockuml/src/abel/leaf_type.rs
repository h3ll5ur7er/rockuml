//! What kind of element a leaf or a group entity is (PlantUML's `LeafType` and `GroupType`).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LeafType {
    EmptyPackage,
    AbstractClass,
    Class,
    Interface,
    Annotation,
    Protocol,
    Struct,
    Exception,
    Metaclass,
    Stereotype,
    LollipopFull,
    LollipopHalf,
    Note,
    Tips,
    Object,
    Map,
    Json,
    Association,
    Enum,
    Circle,
    Dataclass,
    Record,
    Usecase,
    UsecaseBusiness,
    Description,
    ArcCircle,
    Activity,
    Branch,
    SynchroBar,
    CircleStart,
    CircleEnd,
    PointForAssociation,
    ActivityConcurrent,
    State,
    StateConcurrent,
    PseudoState,
    DeepHistory,
    StateChoice,
    StateForkJoin,
    StateTransitionLabel,
    Block,
    Entity,
    Domain,
    Requirement,
    Portin,
    Portout,
    ChenEntity,
    ChenRelationship,
    ChenAttribute,
    ChenCircle,
    StillUnknown,
}

impl LeafType {
    /// Every type, in declaration order.
    const ALL: [Self; 51] = [
        Self::EmptyPackage,
        Self::AbstractClass,
        Self::Class,
        Self::Interface,
        Self::Annotation,
        Self::Protocol,
        Self::Struct,
        Self::Exception,
        Self::Metaclass,
        Self::Stereotype,
        Self::LollipopFull,
        Self::LollipopHalf,
        Self::Note,
        Self::Tips,
        Self::Object,
        Self::Map,
        Self::Json,
        Self::Association,
        Self::Enum,
        Self::Circle,
        Self::Dataclass,
        Self::Record,
        Self::Usecase,
        Self::UsecaseBusiness,
        Self::Description,
        Self::ArcCircle,
        Self::Activity,
        Self::Branch,
        Self::SynchroBar,
        Self::CircleStart,
        Self::CircleEnd,
        Self::PointForAssociation,
        Self::ActivityConcurrent,
        Self::State,
        Self::StateConcurrent,
        Self::PseudoState,
        Self::DeepHistory,
        Self::StateChoice,
        Self::StateForkJoin,
        Self::StateTransitionLabel,
        Self::Block,
        Self::Entity,
        Self::Domain,
        Self::Requirement,
        Self::Portin,
        Self::Portout,
        Self::ChenEntity,
        Self::ChenRelationship,
        Self::ChenAttribute,
        Self::ChenCircle,
        Self::StillUnknown,
    ];

    /// The Java constant's name, like `ABSTRACT_CLASS`, which `hide` commands report as the gender.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::EmptyPackage => "EMPTY_PACKAGE",
            Self::AbstractClass => "ABSTRACT_CLASS",
            Self::Class => "CLASS",
            Self::Interface => "INTERFACE",
            Self::Annotation => "ANNOTATION",
            Self::Protocol => "PROTOCOL",
            Self::Struct => "STRUCT",
            Self::Exception => "EXCEPTION",
            Self::Metaclass => "METACLASS",
            Self::Stereotype => "STEREOTYPE",
            Self::LollipopFull => "LOLLIPOP_FULL",
            Self::LollipopHalf => "LOLLIPOP_HALF",
            Self::Note => "NOTE",
            Self::Tips => "TIPS",
            Self::Object => "OBJECT",
            Self::Map => "MAP",
            Self::Json => "JSON",
            Self::Association => "ASSOCIATION",
            Self::Enum => "ENUM",
            Self::Circle => "CIRCLE",
            Self::Dataclass => "DATACLASS",
            Self::Record => "RECORD",
            Self::Usecase => "USECASE",
            Self::UsecaseBusiness => "USECASE_BUSINESS",
            Self::Description => "DESCRIPTION",
            Self::ArcCircle => "ARC_CIRCLE",
            Self::Activity => "ACTIVITY",
            Self::Branch => "BRANCH",
            Self::SynchroBar => "SYNCHRO_BAR",
            Self::CircleStart => "CIRCLE_START",
            Self::CircleEnd => "CIRCLE_END",
            Self::PointForAssociation => "POINT_FOR_ASSOCIATION",
            Self::ActivityConcurrent => "ACTIVITY_CONCURRENT",
            Self::State => "STATE",
            Self::StateConcurrent => "STATE_CONCURRENT",
            Self::PseudoState => "PSEUDO_STATE",
            Self::DeepHistory => "DEEP_HISTORY",
            Self::StateChoice => "STATE_CHOICE",
            Self::StateForkJoin => "STATE_FORK_JOIN",
            Self::StateTransitionLabel => "STATE_TRANSITION_LABEL",
            Self::Block => "BLOCK",
            Self::Entity => "ENTITY",
            Self::Domain => "DOMAIN",
            Self::Requirement => "REQUIREMENT",
            Self::Portin => "PORTIN",
            Self::Portout => "PORTOUT",
            Self::ChenEntity => "CHEN_ENTITY",
            Self::ChenRelationship => "CHEN_RELATIONSHIP",
            Self::ChenAttribute => "CHEN_ATTRIBUTE",
            Self::ChenCircle => "CHEN_CIRCLE",
            Self::StillUnknown => "STILL_UNKNOWN",
        }
    }

    /// The type a command keyword names, like `abstract class` or `diamond`; `None` for a word naming none.
    pub(crate) fn get_leaf_type(keyword: &str) -> Option<Self> {
        let keyword = keyword.to_uppercase();
        if keyword.starts_with("ABSTRACT") {
            return Some(Self::AbstractClass);
        }
        if keyword.starts_with("DIAMOND") {
            return Some(Self::StateChoice);
        }
        if keyword.starts_with("STATIC") {
            return Some(Self::Class);
        }
        Self::ALL
            .into_iter()
            .find(|leaf_type| leaf_type.name() == keyword)
    }

    pub(crate) fn is_like_class(self) -> bool {
        matches!(
            self,
            Self::Annotation
                | Self::AbstractClass
                | Self::Class
                | Self::Interface
                | Self::Enum
                | Self::Entity
                | Self::Protocol
                | Self::Struct
                | Self::Exception
                | Self::Metaclass
                | Self::Stereotype
                | Self::Dataclass
                | Self::Record
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GroupType {
    Root,
    Package,
    State,
    ConcurrentState,
    Domain,
    Requirement,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_name_types() {
        assert_eq!(LeafType::get_leaf_type("class"), Some(LeafType::Class));
        assert_eq!(
            LeafType::get_leaf_type("abstract class"),
            Some(LeafType::AbstractClass)
        );
        assert_eq!(LeafType::get_leaf_type("static"), Some(LeafType::Class));
        assert_eq!(
            LeafType::get_leaf_type("diamond"),
            Some(LeafType::StateChoice)
        );
        assert_eq!(
            LeafType::get_leaf_type("usecase_business"),
            Some(LeafType::UsecaseBusiness)
        );
        assert_eq!(LeafType::get_leaf_type("nothing"), None);
        assert_eq!(LeafType::LollipopFull.name(), "LOLLIPOP_FULL");
    }

    #[test]
    fn class_like_types_are_plantumls() {
        assert!(LeafType::Entity.is_like_class());
        assert!(LeafType::Record.is_like_class());
        assert!(!LeafType::Object.is_like_class());
        assert!(!LeafType::Note.is_like_class());
    }
}

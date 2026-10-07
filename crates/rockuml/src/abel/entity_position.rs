//! Where an entity sits on its container (PlantUML's `EntityPosition`), and the parts of an entity `hide`
//! and `show` act on (`EntityPortion`).

use crate::svek::ShapeType;

/// Entry and exit points, pins and ports sit on their container's border; normal entities inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityPosition {
    Normal,
    EntryPoint,
    ExitPoint,
    InputPin,
    OutputPin,
    ExpansionInput,
    ExpansionOutput,
    Portin,
    Portout,
}

impl EntityPosition {
    pub(crate) const RADIUS: f64 = 6.0;

    pub(crate) fn is_normal(self) -> bool {
        self == Self::Normal
    }

    pub(crate) fn is_input(self) -> bool {
        matches!(
            self,
            Self::EntryPoint | Self::InputPin | Self::ExpansionInput | Self::Portin
        )
    }

    pub(crate) fn is_output(self) -> bool {
        matches!(
            self,
            Self::ExitPoint | Self::OutputPin | Self::ExpansionOutput | Self::Portout
        )
    }

    /// # Panics
    ///
    /// For normal entities, which have no symbol of their own.
    pub(crate) fn get_shape_type(self) -> ShapeType {
        match self {
            Self::Normal => panic!("normal entities have no position symbol"),
            Self::EntryPoint | Self::ExitPoint => ShapeType::RectanglePort,
            _ => ShapeType::Rectangle,
        }
    }

    /// The position a state's stereotype like `<<entryPoint>>` asks for, ignoring case.
    pub(crate) fn from_stereotype(label: &str) -> Self {
        [
            ("<<entrypoint>>", Self::EntryPoint),
            ("<<exitpoint>>", Self::ExitPoint),
            ("<<inputpin>>", Self::InputPin),
            ("<<outputpin>>", Self::OutputPin),
            ("<<expansioninput>>", Self::ExpansionInput),
            ("<<expansionoutput>>", Self::ExpansionOutput),
        ]
        .into_iter()
        .find(|(name, _)| label.eq_ignore_ascii_case(name))
        .map_or(Self::Normal, |(_, position)| position)
    }

    pub(crate) fn is_port(self) -> bool {
        matches!(self, Self::Portin | Self::Portout)
    }

    /// Whether links attach to the symbol itself rather than to a node's border.
    pub(crate) fn use_port_p(self) -> bool {
        self.is_port() || matches!(self, Self::ExitPoint | Self::EntryPoint)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityPortion {
    Field,
    Method,
    Member,
    CircledCharacter,
    Stereotype,
}

impl EntityPortion {
    /// The portions this one stands for: members are fields and methods.
    pub(crate) fn as_set(self) -> Vec<Self> {
        match self {
            Self::Member => vec![Self::Field, Self::Method],
            other => vec![other],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereotypes_place_states_on_the_border() {
        assert_eq!(
            EntityPosition::from_stereotype("<<entryPoint>>"),
            EntityPosition::EntryPoint
        );
        assert_eq!(
            EntityPosition::from_stereotype("<<ExpansionOutput>>"),
            EntityPosition::ExpansionOutput
        );
        assert_eq!(
            EntityPosition::from_stereotype("<<choice>>"),
            EntityPosition::Normal
        );
        assert!(EntityPosition::ExitPoint.use_port_p());
        assert!(!EntityPosition::InputPin.use_port_p());
        assert_eq!(
            EntityPosition::EntryPoint.get_shape_type(),
            ShapeType::RectanglePort
        );
    }

    #[test]
    fn members_are_fields_and_methods() {
        assert_eq!(
            EntityPortion::Member.as_set(),
            [EntityPortion::Field, EntityPortion::Method]
        );
        assert_eq!(
            EntityPortion::Stereotype.as_set(),
            [EntityPortion::Stereotype]
        );
    }
}

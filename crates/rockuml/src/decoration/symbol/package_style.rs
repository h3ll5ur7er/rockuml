//! The shape packages take, set with `skinparam packageStyle` or a stereotype like `<<Node>>` (PlantUML's
//! `svek.PackageStyle`).

use super::{USymbol, USymbols};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PackageStyle {
    Folder,
    Rectangle,
    Node,
    Frame,
    Cloud,
    Database,
    Agent,
    Storage,
    Component1,
    Component2,
    Artifact,
    Card,
}

impl PackageStyle {
    /// In declaration order, which picks the first match.
    const ALL: [Self; 12] = [
        Self::Folder,
        Self::Rectangle,
        Self::Node,
        Self::Frame,
        Self::Cloud,
        Self::Database,
        Self::Agent,
        Self::Storage,
        Self::Component1,
        Self::Component2,
        Self::Artifact,
        Self::Card,
    ];

    /// The Java constant's name.
    fn name(self) -> &'static str {
        match self {
            Self::Folder => "FOLDER",
            Self::Rectangle => "RECTANGLE",
            Self::Node => "NODE",
            Self::Frame => "FRAME",
            Self::Cloud => "CLOUD",
            Self::Database => "DATABASE",
            Self::Agent => "AGENT",
            Self::Storage => "STORAGE",
            Self::Component1 => "COMPONENT1",
            Self::Component2 => "COMPONENT2",
            Self::Artifact => "ARTIFACT",
            Self::Card => "CARD",
        }
    }

    /// A style by name in any case; `rect` is a rectangle.
    pub(crate) fn from_string(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|style| style.name().eq_ignore_ascii_case(value))
            .or_else(|| {
                value
                    .eq_ignore_ascii_case("rect")
                    .then_some(Self::Rectangle)
            })
    }

    /// The symbol a package of this style is drawn as; agents, storages, components and artifacts have none.
    pub(crate) fn to_u_symbol(self) -> Option<USymbol> {
        match self {
            Self::Node => Some(USymbols::NODE),
            Self::Card => Some(USymbols::CARD),
            Self::Database => Some(USymbols::DATABASE),
            Self::Cloud => Some(USymbols::CLOUD),
            Self::Frame => Some(USymbols::FRAME),
            Self::Rectangle => Some(USymbols::RECTANGLE),
            Self::Folder => Some(USymbols::PACKAGE),
            Self::Agent | Self::Storage | Self::Component1 | Self::Component2 | Self::Artifact => {
                None
            }
        }
    }
}

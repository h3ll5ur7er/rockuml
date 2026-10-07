//! How components are drawn, chosen with `skinparam componentStyle` (PlantUML's `ComponentStyle`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComponentStyle {
    Uml1,
    Uml2,
    Rectangle,
}

//! How far an arrow may be merged with the arrows it touches (PlantUML's `MergeStrategy`).

/// Declared from the most to the least merging, so that the stricter of two strategies is their `max`, as
/// in PlantUML.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MergeStrategy {
    Full,
    Limited,
    None,
}

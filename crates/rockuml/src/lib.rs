//! PlantUML-compatible diagram engine.

mod java;
mod pattern;
pub mod preproc;
mod text;

/// The PlantUML release whose input language and output rockuml reproduces.
pub const PLANTUML_VERSION: &str = "1.2026.8";

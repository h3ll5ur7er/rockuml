//! PlantUML-compatible diagram engine.

mod assets;
mod color;
pub mod host;
mod java;
mod jaws;
pub mod json;
mod pattern;
pub mod preproc;
mod stdlib;
mod text;
mod tim;

/// The PlantUML release whose input language and output rockuml reproduces.
pub const PLANTUML_VERSION: &str = "1.2026.8";

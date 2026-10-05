//! PlantUML-compatible diagram engine.

mod assets;
mod color;
mod command;
mod creole;
mod deflate;
pub mod diagram;
pub mod host;
mod java;
mod jaws;
pub mod json;
mod klimt;
mod pattern;
pub mod preproc;
mod stdlib;
mod style;
mod text;
mod tim;
pub mod url_code;
pub mod url_policy;

/// The PlantUML release whose input language and output rockuml reproduces.
pub const PLANTUML_VERSION: &str = "1.2026.8";

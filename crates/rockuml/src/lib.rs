//! PlantUML-compatible diagram engine.

mod assets;
mod color;
mod command;
mod creole;
mod deflate;
pub mod diagram;
/// The fonts text is measured with: embedded ones, and any the embedding application registers.
pub mod fonts {
    pub use crate::klimt::typeface::{FontRegistry, NotAFont};
}
pub mod host;
mod java;
mod jaws;
pub mod json;
mod klimt;
mod openiconic;
mod pattern;
pub mod preproc;
mod real;
mod skin;
mod stdlib;
mod style;
mod text;
mod tim;
mod ubrex;
pub mod url_code;
pub mod url_policy;

/// The PlantUML release whose input language and output rockuml reproduces.
pub const PLANTUML_VERSION: &str = "1.2026.8";

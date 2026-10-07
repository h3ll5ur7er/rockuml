//! PlantUML-compatible diagram engine.

mod abel;
mod assets;
mod color;
mod command;
mod creole;
mod cucadiagram;
mod decoration;
mod deflate;
pub mod diagram;
mod direction;
mod emoji;
mod file_policy;
mod ftile;
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
mod plasma;
pub mod preproc;
mod real;
mod sdot;
mod security_profile;
mod skin;
mod stdlib;
mod stereo;
mod style;
mod svek;
mod svg_parser;
mod text;
mod tim;
mod ubrex;
pub mod url_code;
pub mod url_policy;

/// The PlantUML release whose input language and output rockuml reproduces.
pub const PLANTUML_VERSION: &str = "1.2026.8";

//! Creole, PlantUML's wiki-like text markup: bold, lists, headings, tables and more inside any label.
//!
//! A label's lines become a [`Sheet`] of [`Stripe`]s (one per visual line), each a row of [`Atom`]s that
//! [`SheetBlock1`] lays out.

mod atom_text;
mod atoms;
mod char_hidder;
mod commands;
mod display;
mod parser;
mod sheet_block;

pub use display::Display;
pub use parser::CreoleParser;
pub use sheet_block::SheetBlock1;

use crate::klimt::font::StringBounder;
use crate::klimt::{HorizontalAlignment, TextBlock};

/// How much markup a text reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CreoleMode {
    #[default]
    Full,
    /// Inline markup only: no lists, headings or separators.
    SimpleLine,
    NoCreole,
    /// Full creole except `__underline__` and lists.
    FullButUnderscore,
}

/// The smallest piece of a creole line: a run of text, an image, a bullet...
pub trait Atom: TextBlock {
    /// How far the atom sits above the line's bottom: raised for superscript, lowered for subscript.
    fn starting_altitude(&self, string_bounder: &dyn StringBounder) -> f64;
}

pub struct Stripe {
    atoms: Vec<Box<dyn Atom>>,
    cell_alignment: HorizontalAlignment,
}

pub struct Sheet {
    stripes: Vec<Stripe>,
}

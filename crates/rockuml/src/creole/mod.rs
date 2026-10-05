//! Creole, PlantUML's wiki-like text markup: bold, lists, headings, tables and more inside any label.
//!
//! A label's lines become a [`Sheet`] of [`Stripe`]s (one per visual line), each a row of [`Atom`]s that
//! [`SheetBlock1`] lays out.

mod atom_text;
mod atoms;
mod char_hidder;
mod code;
mod commands;
mod display;
mod parser;
mod sheet_block;
mod table;
mod tree;

pub(crate) use display::Display;
pub(crate) use parser::CreoleParser;
pub(crate) use sheet_block::{SheetBlock1, SheetBlock2};

use crate::klimt::font::StringBounder;
use crate::klimt::{HorizontalAlignment, TextBlock};

/// How much markup a text reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CreoleMode {
    #[default]
    Full,
    /// Full creole except `__underline__` and lists.
    FullButUnderscore,
}

/// The smallest piece of a creole line: a run of text, an image, a bullet...
pub(crate) trait Atom: TextBlock {
    /// How far the atom sits above the line's bottom: raised for superscript, lowered for subscript.
    fn starting_altitude(&self, string_bounder: &dyn StringBounder) -> f64;
}

pub(crate) struct Stripe {
    atoms: Vec<Box<dyn Atom>>,
    cell_alignment: HorizontalAlignment,
}

pub(crate) struct Sheet {
    stripes: Vec<Stripe>,
}

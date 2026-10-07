//! The bodies of entities: members of classes and objects, entries of maps, JSON data, and the text blocks
//! that draw them (PlantUML's `cucadiagram` package).

mod bodier;
mod body_enhanced;
mod member;
mod methods_or_fields_area;

pub(crate) use bodier::{Bodier, get_linked_entry};
pub(crate) use member::Member;

use crate::color::Colors;
use crate::skin::SkinParam;
use crate::skin::visibility_modifier::VisibilityModifier;
use crate::style::{Style, StyleBuilder};

/// What drawing an entity's body depends on besides the body.
pub(crate) struct BodyContext<'a> {
    pub skin: &'a SkinParam,
    /// The diagram's style rules at its end, which style visibility icons.
    pub style_builder: &'a StyleBuilder,
    /// The entity's style, which gives the text its font.
    pub style: &'a Style,
    /// The entity's own colours.
    pub colors: &'a Colors,
    /// The visibilities whose members `hide` commands leave out.
    pub hidden: &'a [VisibilityModifier],
}

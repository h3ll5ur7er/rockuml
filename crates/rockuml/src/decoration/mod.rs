//! How diagram elements and links are decorated: link ends and middles, line styles, colours and the
//! symbols of elements (PlantUML's `decoration` package, without the drawing of link extremities, which
//! lives with svek).

mod link_decor;
mod link_style;
mod link_type;
mod rainbow;
pub(crate) mod symbol;

pub(crate) use link_decor::LinkDecor;
pub(crate) use link_style::LinkStyle;
pub(crate) use link_type::LinkType;
pub(crate) use rainbow::Rainbow;

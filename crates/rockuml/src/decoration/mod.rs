//! How diagram elements and links are decorated: link ends and middles, line styles, colours and the
//! symbols of elements (PlantUML's `decoration` package, without the drawing of link extremities, which
//! lives with svek).

mod html_color_and_style;
mod link_decor;
mod link_style;
mod link_type;
mod rainbow;
pub(crate) mod symbol;
mod with_link_type;

pub(crate) use html_color_and_style::HtmlColorAndStyle;
pub(crate) use link_decor::LinkDecor;
pub(crate) use link_style::LinkStyle;
pub(crate) use link_type::LinkType;
pub(crate) use rainbow::Rainbow;
pub(crate) use with_link_type::WithLinkType;

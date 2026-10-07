//! How diagram elements and links are decorated: link ends and middles, line styles, colours and the
//! symbols of elements (PlantUML's `decoration` package, without the drawing of link extremities, which
//! lives with svek).

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        unused_imports,
        reason = "used by the Phase 5 family commands"
    )
)]
#![cfg_attr(
    test,
    allow(
        dead_code,
        unused_imports,
        reason = "used by the Phase 5 family commands"
    )
)]

mod link_decor;
mod link_style;
mod link_type;
mod rainbow;
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "drawn by the description diagrams and clusters, which are ported next"
    )
)]
pub(crate) mod symbol;

pub(crate) use link_decor::LinkDecor;
pub(crate) use link_style::LinkStyle;
pub(crate) use link_type::{LinkMiddleDecor, LinkType};
pub(crate) use rainbow::{HtmlColorAndStyle, Rainbow};

//! How links look: their end and middle decorations, line styles and colours (PlantUML's `decoration`
//! package, without the drawing of extremities, which lives with svek).

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

pub(crate) use link_decor::LinkDecor;
pub(crate) use link_style::LinkStyle;
pub(crate) use link_type::{LinkMiddleDecor, LinkType};
pub(crate) use rainbow::{HtmlColorAndStyle, Rainbow};

//! Symbols drawn as a figure above their label (PlantUML's `USymbolSimpleAbstract` and its subclasses
//! `USymbolActor`, `USymbolActorBusiness`, `USymbolBoundary`, `USymbolControl`, `USymbolEntityDomain` and
//! `USymbolInterface`).

use std::rc::Rc;

use super::SmallContent;
use crate::klimt::TextBlock;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::symbol::{Boundary, CircleInterface2, Control, EntityDomain};

/// Whatever shadow the style casts, these figures cast one of their own size.
fn figure_shadow(fashion: &Fashion) -> f64 {
    if fashion.is_shadowing() { 4.0 } else { 0.0 }
}

pub(super) fn boundary(content: SmallContent) -> Box<dyn TextBlock> {
    let fashion = &content.fashion;
    let drawing = Box::new(Boundary::new(
        fashion.with_delta_shadow(figure_shadow(fashion)),
    ));
    as_small(drawing, content)
}

pub(super) fn control(content: SmallContent) -> Box<dyn TextBlock> {
    let fashion = &content.fashion;
    let drawing = Box::new(Control::new(
        fashion.with_delta_shadow(figure_shadow(fashion)),
    ));
    as_small(drawing, content)
}

pub(super) fn entity_domain(content: SmallContent) -> Box<dyn TextBlock> {
    let fashion = &content.fashion;
    let drawing = Box::new(EntityDomain::new(
        fashion.with_delta_shadow(figure_shadow(fashion)),
    ));
    as_small(drawing, content)
}

pub(super) fn interface(content: SmallContent) -> Box<dyn TextBlock> {
    let fashion = &content.fashion;
    let drawing = Box::new(CircleInterface2::new(
        fashion.back_color.clone(),
        fashion.fore_color.clone(),
        figure_shadow(fashion),
    ));
    as_small(drawing, content)
}

/// `drawing` with the stereotype above it and the label below.
pub(super) fn as_small(drawing: Box<dyn TextBlock>, content: SmallContent) -> Box<dyn TextBlock> {
    Box::new(SmallSimple { drawing, content })
}

struct SmallSimple {
    drawing: Box<dyn TextBlock>,
    content: SmallContent,
}

impl TextBlock for SmallSimple {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_label = self.content.label.calculate_dimension(string_bounder);
        let dim_stereo = self.content.stereotype.calculate_dimension(string_bounder);
        let dim_actor = self.drawing.calculate_dimension(string_bounder);
        XDimension2D::new(
            dim_stereo.width.max(dim_actor.width).max(dim_label.width),
            dim_stereo.height + dim_actor.height + dim_label.height,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_label = self.content.label.calculate_dimension(string_bounder);
        let dim_stereo = self.content.stereotype.calculate_dimension(string_bounder);
        let dim_drawing = self.drawing.calculate_dimension(string_bounder);
        let dim_total = self.calculate_dimension(string_bounder);
        let ug = self.content.fashion.apply(ug);
        self.drawing.draw_u(&ug.translated(
            (dim_total.width - dim_drawing.width) / 2.0,
            dim_stereo.height,
        ));
        // The stencil spans the label from where the symbol is, not from where the label is.
        let label_ug = ug.with_stencil(Rc::new(RectangleStencil {
            width: dim_label.width,
        }));
        self.content.label.draw_u(&label_ug.translated(
            (dim_total.width - dim_label.width) / 2.0,
            dim_drawing.height + dim_stereo.height,
        ));
        self.content
            .stereotype
            .draw_u(&ug.translated((dim_total.width - dim_stereo.width) / 2.0, 0.0));
    }
}

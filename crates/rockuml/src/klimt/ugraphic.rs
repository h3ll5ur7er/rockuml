use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use super::clip::UClip;
use super::font::StringBounder;
use super::geom::UTranslate;
use super::group::UGroup;
use super::shape::UShape;
use super::stencil::{Stencil, StencilFrame, UHorizontalLine};
use super::url::Url;
use crate::color::HColor;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UStroke {
    pub dash_visible: f64,
    pub dash_space: f64,
    pub thickness: f64,
}

impl UStroke {
    pub const SIMPLE: Self = Self::with_thickness(1.0);

    pub const fn with_thickness(thickness: f64) -> Self {
        Self {
            dash_visible: 0.0,
            dash_space: 0.0,
            thickness,
        }
    }
}

impl fmt::Display for UStroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::java::double_to_string as java;
        write!(
            f,
            "{}-{}-{}",
            java(self.dash_visible),
            java(self.dash_space),
            java(self.thickness)
        )
    }
}

/// The drawing state shapes are drawn with.
#[derive(Clone, Debug, PartialEq)]
pub struct UParam {
    pub color: HColor,
    pub backcolor: HColor,
    pub stroke: UStroke,
    /// Where on the document drawing is limited to.
    pub clip: Option<UClip>,
}

/// Receives the shapes of one output document.
pub trait UGraphicBackend {
    fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam);

    /// Formats without groups ignore them.
    fn start_group(&mut self, _group: &UGroup) {}

    fn close_group(&mut self) {}

    /// Formats without links ignore them.
    fn start_url(&mut self, _url: &Url) {}

    fn close_url(&mut self) {}
}

/// A drawing surface positioned somewhere on a document; cheap to derive moved or restyled copies from.
#[derive(Clone)]
pub struct UGraphic {
    backend: Rc<RefCell<dyn UGraphicBackend>>,
    string_bounder: Rc<dyn StringBounder>,
    default_background: HColor,
    translate: UTranslate,
    param: UParam,
    stencil: Option<StencilFrame>,
}

impl UGraphic {
    pub fn new(
        backend: Rc<RefCell<dyn UGraphicBackend>>,
        string_bounder: Rc<dyn StringBounder>,
        default_background: HColor,
    ) -> Self {
        Self {
            backend,
            string_bounder,
            default_background,
            translate: UTranslate::default(),
            param: UParam {
                color: HColor::NONE,
                backcolor: HColor::NONE,
                stroke: UStroke::SIMPLE,
                clip: None,
            },
            stencil: None,
        }
    }

    pub fn string_bounder(&self) -> &dyn StringBounder {
        self.string_bounder.as_ref()
    }

    pub fn param(&self) -> &UParam {
        &self.param
    }

    /// What lies behind everything: the colour text must stay readable on.
    pub fn default_background(&self) -> &HColor {
        &self.default_background
    }

    #[must_use]
    pub fn translated(&self, dx: f64, dy: f64) -> Self {
        Self {
            translate: UTranslate::new(dx, dy).compose(self.translate),
            ..self.clone()
        }
    }

    /// Lines are drawn in `color`.
    #[must_use]
    pub fn with_color(&self, color: HColor) -> Self {
        let mut copy = self.clone();
        copy.param.color = color;
        copy
    }

    /// Shapes are filled with `backcolor`.
    #[must_use]
    pub fn with_backcolor(&self, backcolor: HColor) -> Self {
        let mut copy = self.clone();
        copy.param.backcolor = backcolor;
        copy
    }

    #[must_use]
    pub fn with_stroke(&self, stroke: UStroke) -> Self {
        let mut copy = self.clone();
        copy.param.stroke = stroke;
        copy
    }

    /// Drawing is limited to `clip`, given where this surface is.
    #[must_use]
    pub fn with_clip(&self, clip: UClip) -> Self {
        let mut copy = self.clone();
        copy.param.clip = Some(clip.translate(self.translate.dx, self.translate.dy));
        copy
    }

    /// Shapes drawn until the matching `close_group` belong to `group`.
    pub fn start_group(&self, group: &UGroup) {
        self.backend.borrow_mut().start_group(group);
    }

    pub fn close_group(&self) {
        self.backend.borrow_mut().close_group();
    }

    /// Shapes drawn until the matching `close_url` follow `url` when clicked.
    pub fn start_url(&self, url: &Url) {
        self.backend.borrow_mut().start_url(url);
    }

    pub fn close_url(&self) {
        self.backend.borrow_mut().close_url();
    }

    /// Separators drawn on the result span `stencil`, which is placed where this surface is.
    #[must_use]
    pub fn with_stencil(&self, stencil: Rc<dyn Stencil>) -> Self {
        Self {
            stencil: Some(StencilFrame {
                stencil,
                origin: self.translate,
            }),
            ..self.clone()
        }
    }

    /// Draws the separator across the stencil, or as a bare separator shape where there is none (which only
    /// the debug format lists).
    pub fn draw_horizontal_line(&self, line: &UHorizontalLine) {
        let Some(frame) = &self.stencil else {
            self.draw(&UShape::HorizontalLine);
            return;
        };
        let at_stencil = Self {
            translate: frame.origin,
            stencil: None,
            ..self.clone()
        };
        line.draw_line_internal(
            &at_stencil,
            frame.stencil.as_ref(),
            self.translate.dy - frame.origin.dy,
        );
    }

    pub fn draw(&self, shape: &UShape) {
        self.backend
            .borrow_mut()
            .draw(shape, self.translate, &self.param);
    }
}

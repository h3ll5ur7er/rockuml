//! Drawing surfaces (PlantUML's `UGraphic`).
//!
//! A [`UGraphic`] is a cheap value: deriving a moved or restyled copy (`apply`) never changes the original.
//! It is one of two kinds.
//!
//! - A *surface* over a [`UGraphicBackend`]: an output format (SVG, debug) or a measuring surface
//!   (`LimitFinder`). It keeps the absolute translation and the drawing state, and hands the backend
//!   primitive [`UShape`]s at absolute positions. Every diagram but activity diagrams only ever draws on
//!   surfaces.
//! - A *layer* ([`UGraphicLayer`]) over another `UGraphic`: PlantUML's `UGraphicDelegator` family, which
//!   activity diagrams stack to intercept what their tiles draw. A layer sees everything drawn on it as an
//!   [`AnyShape`], including the composite shapes activity diagrams draw with `ug.draw(...)`: tiles
//!   ([`Ftile`]), connections ([`Connection`]) and arrows ([`Snake`]). It handles some and passes the rest
//!   down; `apply` returns a new layer over the layer below with the change applied as PlantUML's layer
//!   applies it (most pass every change down; some keep the translation to themselves).
//!
//! What reaches a surface is drawn there: primitives by the backend, a tile or connection by drawing
//! itself on the surface, an arrow by drawing its lines. PlantUML's output formats fail on composite shapes
//! and its `LimitFinder` draws tiles through itself; activity diagrams always dispatch them in a layer
//! first, so surfaces only meet them when a tile is measured on its own.
//!
//! [`UGraphic::flush_ug`] ends a drawing: layers that hold shapes back (the arrows of `UGraphicForSnake`)
//! draw them then. [`UGraphic::layer`] answers PlantUML's `ug instanceof SomeLayer` for the outermost
//! layer.

use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use super::TextBlock;
use super::clip::UClip;
use super::font::StringBounder;
use super::geom::UTranslate;
use super::group::UGroup;
use super::shape::UShape;
use super::stencil::{
    HorizontalLineDrawer, Stencil, StencilFrame, UGraphicStencil, UHorizontalLine,
};
use super::url::Url;
use crate::color::HColor;
use crate::ftile::{Connection, Ftile, Snake};

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

    /// Whether the format draws the text block of a special text, as PlantUML's `AbstractUGraphic` does,
    /// rather than receiving the shape.
    fn draws_special_text(&self) -> bool {
        false
    }

    /// Formats without groups ignore them.
    fn start_group(&mut self, _group: &UGroup) {}

    fn close_group(&mut self) {}

    /// Formats without links ignore them.
    fn start_url(&mut self, _url: &Url) {}

    fn close_url(&mut self) {}
}

/// A change [`UGraphic::apply`] makes (PlantUML's `UChange`).
#[derive(Clone)]
pub(crate) enum UChange {
    Translate(UTranslate),
    /// The colour lines are drawn in (an `HColor` applied as itself).
    Color(HColor),
    /// The colour shapes are filled with (`UBackground`, written `color.bg()`).
    Background(HColor),
    Stroke(UStroke),
    /// Given where the surface is.
    Clip(UClip),
    /// Separators drawn on the result are drawn by this drawer, placed where the surface is (PlantUML wraps
    /// the surface in an `AbstractUGraphicHorizontalLine`).
    HorizontalLineDrawer(Rc<dyn HorizontalLineDrawer>),
}

impl From<UTranslate> for UChange {
    fn from(translate: UTranslate) -> Self {
        Self::Translate(translate)
    }
}

impl From<HColor> for UChange {
    fn from(color: HColor) -> Self {
        Self::Color(color)
    }
}

impl From<UStroke> for UChange {
    fn from(stroke: UStroke) -> Self {
        Self::Stroke(stroke)
    }
}

impl From<UClip> for UChange {
    fn from(clip: UClip) -> Self {
        Self::Clip(clip)
    }
}

/// Anything [`UGraphic::draw`] takes: PlantUML's `UShape`, which besides klimt's primitives includes the
/// composite shapes activity diagrams draw and layers intercept.
#[derive(Clone, Copy)]
pub(crate) enum AnyShape<'a> {
    Shape(&'a UShape),
    /// A title some formats draw as its text block (`SpecialText`).
    SpecialText(&'a dyn TextBlock),
    /// A separator across whatever contains it (`UHorizontalLine`).
    HorizontalLine(&'a UHorizontalLine<'a>),
    Ftile(&'a dyn Ftile),
    Connection(&'a dyn Connection),
    /// An arrow (`Snake`); `UGraphicForSnake` collects and merges them.
    Snake(&'a Snake),
}

impl<'a> From<&'a UShape> for AnyShape<'a> {
    fn from(shape: &'a UShape) -> Self {
        Self::Shape(shape)
    }
}

impl<'a> From<&'a dyn Ftile> for AnyShape<'a> {
    fn from(tile: &'a dyn Ftile) -> Self {
        Self::Ftile(tile)
    }
}

impl<'a> From<&'a Rc<dyn Ftile>> for AnyShape<'a> {
    fn from(tile: &'a Rc<dyn Ftile>) -> Self {
        Self::Ftile(tile.as_ref())
    }
}

impl<'a> From<&'a dyn Connection> for AnyShape<'a> {
    fn from(connection: &'a dyn Connection) -> Self {
        Self::Connection(connection)
    }
}

impl<'a> From<&'a Rc<dyn Connection>> for AnyShape<'a> {
    fn from(connection: &'a Rc<dyn Connection>) -> Self {
        Self::Connection(connection.as_ref())
    }
}

impl<'a> From<&'a Snake> for AnyShape<'a> {
    fn from(snake: &'a Snake) -> Self {
        Self::Snake(snake)
    }
}

/// A `UGraphic` that intercepts what is drawn on it before passing it on to [`Self::ug`] (PlantUML's
/// `UGraphicDelegator` and the other `UGraphic`s that wrap one). Layers are immutable; what they share
/// between the copies `apply` makes (collected arrows, label positions) lives behind an `Rc`.
///
/// To add one: implement `ug` and `apply`, override `draw` for the shapes it intercepts, and wrap it with
/// [`UGraphic::from_layer`] in its constructor so that callers hold a `UGraphic`.
pub(crate) trait UGraphicLayer: Any {
    /// The surface below (`getUg()`). Queries such as the drawing state and the string bounder go there,
    /// and so does everything the defaults below pass on.
    fn ug(&self) -> &UGraphic;

    /// The layer after `change`, as PlantUML's layer applies it; usually a copy over `self.ug().apply(change)`.
    fn apply(&self, change: UChange) -> UGraphic;

    /// Draws `shape`; `this` is the `UGraphic` holding this layer, for drawing a tile back through the layer
    /// as PlantUML's `tile.drawU(this)`.
    fn draw(&self, _this: &UGraphic, shape: AnyShape<'_>) {
        self.ug().draw(shape);
    }

    /// Draws what the layer held back (`flushUg`).
    fn flush_ug(&self) {
        self.ug().flush_ug();
    }

    fn start_group(&self, group: &UGroup) {
        self.ug().start_group(group);
    }

    fn close_group(&self) {
        self.ug().close_group();
    }

    fn start_url(&self, url: &Url) {
        self.ug().start_url(url);
    }

    fn close_url(&self) {
        self.ug().close_url();
    }
}

/// A drawing surface positioned somewhere on a document, or a layer over one; cheap to derive moved or
/// restyled copies from.
#[derive(Clone)]
pub struct UGraphic {
    kind: Kind,
}

#[derive(Clone)]
enum Kind {
    Surface(Surface),
    Layer(Rc<dyn UGraphicLayer>),
}

#[derive(Clone)]
struct Surface {
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
            kind: Kind::Surface(Surface {
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
            }),
        }
    }

    /// A `UGraphic` drawing through `layer`.
    pub(crate) fn from_layer(layer: impl UGraphicLayer) -> Self {
        Self {
            kind: Kind::Layer(Rc::new(layer)),
        }
    }

    /// The outermost layer, when it is a `T` (PlantUML's `ug instanceof T`). Layers below it do not count:
    /// a tile asks about the surface it was handed.
    pub(crate) fn layer<T: UGraphicLayer>(&self) -> Option<&T> {
        match &self.kind {
            Kind::Surface(_) => None,
            Kind::Layer(layer) => (layer.as_ref() as &dyn Any).downcast_ref(),
        }
    }

    pub fn string_bounder(&self) -> &dyn StringBounder {
        match &self.kind {
            Kind::Surface(surface) => surface.string_bounder.as_ref(),
            Kind::Layer(layer) => layer.ug().string_bounder(),
        }
    }

    pub fn param(&self) -> &UParam {
        match &self.kind {
            Kind::Surface(surface) => &surface.param,
            Kind::Layer(layer) => layer.ug().param(),
        }
    }

    /// What lies behind everything: the colour text must stay readable on.
    pub fn default_background(&self) -> &HColor {
        match &self.kind {
            Kind::Surface(surface) => &surface.default_background,
            Kind::Layer(layer) => layer.ug().default_background(),
        }
    }

    /// The surface after `change`.
    #[must_use]
    pub(crate) fn apply(&self, change: impl Into<UChange>) -> Self {
        match (&self.kind, change.into()) {
            (Kind::Surface(surface), change) => Self {
                kind: Kind::Surface(surface.apply(change)),
            },
            (Kind::Layer(_), UChange::HorizontalLineDrawer(drawer)) => {
                Self::from_layer(HorizontalLineLayer {
                    ug: self.clone(),
                    translate: UTranslate::default(),
                    drawer,
                })
            }
            (Kind::Layer(layer), change) => layer.apply(change),
        }
    }

    #[must_use]
    pub fn translated(&self, dx: f64, dy: f64) -> Self {
        self.apply(UTranslate::new(dx, dy))
    }

    /// Lines are drawn in `color`.
    #[must_use]
    pub fn with_color(&self, color: HColor) -> Self {
        self.apply(UChange::Color(color))
    }

    /// Shapes are filled with `backcolor`.
    #[must_use]
    pub fn with_backcolor(&self, backcolor: HColor) -> Self {
        self.apply(UChange::Background(backcolor))
    }

    #[must_use]
    pub fn with_stroke(&self, stroke: UStroke) -> Self {
        self.apply(stroke)
    }

    /// Drawing is limited to `clip`, given where this surface is.
    #[must_use]
    pub fn with_clip(&self, clip: UClip) -> Self {
        self.apply(clip)
    }

    /// Shapes drawn until the matching `close_group` belong to `group`.
    pub fn start_group(&self, group: &UGroup) {
        match &self.kind {
            Kind::Surface(surface) => surface.backend.borrow_mut().start_group(group),
            Kind::Layer(layer) => layer.start_group(group),
        }
    }

    pub fn close_group(&self) {
        match &self.kind {
            Kind::Surface(surface) => surface.backend.borrow_mut().close_group(),
            Kind::Layer(layer) => layer.close_group(),
        }
    }

    /// Shapes drawn until the matching `close_url` follow `url` when clicked.
    pub fn start_url(&self, url: &Url) {
        match &self.kind {
            Kind::Surface(surface) => surface.backend.borrow_mut().start_url(url),
            Kind::Layer(layer) => layer.start_url(url),
        }
    }

    pub fn close_url(&self) {
        match &self.kind {
            Kind::Surface(surface) => surface.backend.borrow_mut().close_url(),
            Kind::Layer(layer) => layer.close_url(),
        }
    }

    /// Draws what layers held back; surfaces hold nothing back (`flushUg`).
    pub(crate) fn flush_ug(&self) {
        if let Kind::Layer(layer) = &self.kind {
            layer.flush_ug();
        }
    }

    /// Separators drawn on the result span `stencil`, which is placed where this surface is.
    #[must_use]
    pub fn with_stencil(&self, stencil: Rc<dyn Stencil>) -> Self {
        self.with_horizontal_line_drawer(Rc::new(UGraphicStencil {
            stencil,
            default_stroke: None,
        }))
    }

    /// Like [`Self::with_stencil`], separators without a style of their own drawn with `default_stroke`.
    #[must_use]
    pub fn with_stencil_stroke(&self, stencil: Rc<dyn Stencil>, default_stroke: UStroke) -> Self {
        self.with_horizontal_line_drawer(Rc::new(UGraphicStencil {
            stencil,
            default_stroke: Some(default_stroke),
        }))
    }

    /// Separators drawn on the result are drawn by `drawer`, on a surface placed where this one is.
    #[must_use]
    pub fn with_horizontal_line_drawer(&self, drawer: Rc<dyn HorizontalLineDrawer>) -> Self {
        self.apply(UChange::HorizontalLineDrawer(drawer))
    }

    /// Draws the separator with the surface's drawer, or as a bare separator shape where there is none (which
    /// only the debug format lists).
    pub fn draw_horizontal_line(&self, line: &UHorizontalLine) {
        self.draw(AnyShape::HorizontalLine(line));
    }

    /// Draws `block` as PlantUML's `SpecialText`.
    pub fn draw_special_text(&self, block: &dyn TextBlock) {
        self.draw(AnyShape::SpecialText(block));
    }

    pub(crate) fn draw<'a>(&self, shape: impl Into<AnyShape<'a>>) {
        let shape = shape.into();
        match &self.kind {
            Kind::Layer(layer) => layer.draw(self, shape),
            Kind::Surface(surface) => match shape {
                AnyShape::Shape(shape) => surface.draw(shape),
                AnyShape::SpecialText(block) => {
                    if surface.backend.borrow().draws_special_text() {
                        block.draw_u(self);
                    } else {
                        surface.draw(&UShape::SpecialText);
                    }
                }
                AnyShape::HorizontalLine(line) => surface.draw_horizontal_line(line),
                AnyShape::Ftile(tile) => tile.draw_u(self),
                AnyShape::Connection(connection) => connection.draw_u(self),
                AnyShape::Snake(snake) => snake.draw_internal(self),
            },
        }
    }
}

impl Surface {
    fn apply(&self, change: UChange) -> Self {
        let mut copy = self.clone();
        match change {
            UChange::Translate(translate) => copy.translate = translate.compose(self.translate),
            UChange::Color(color) => copy.param.color = color,
            UChange::Background(backcolor) => copy.param.backcolor = backcolor,
            UChange::Stroke(stroke) => copy.param.stroke = stroke,
            UChange::Clip(clip) => {
                copy.param.clip = Some(clip.translate(self.translate.dx, self.translate.dy));
            }
            UChange::HorizontalLineDrawer(drawer) => {
                copy.stencil = Some(StencilFrame {
                    drawer,
                    origin: self.translate,
                });
            }
        }
        copy
    }

    fn draw(&self, shape: &UShape) {
        self.backend
            .borrow_mut()
            .draw(shape, self.translate, &self.param);
    }

    /// Draws the separator with the surface's drawer, or as a bare separator shape where there is none.
    fn draw_horizontal_line(&self, line: &UHorizontalLine) {
        let Some(frame) = &self.stencil else {
            self.draw(&UShape::HorizontalLine);
            return;
        };
        let at_stencil = UGraphic {
            kind: Kind::Surface(Self {
                translate: frame.origin,
                stencil: None,
                ..self.clone()
            }),
        };
        frame
            .drawer
            .draw_hline(&at_stencil, line, self.translate.dy - frame.origin.dy);
    }
}

/// A separator drawer set on a layer (PlantUML's `AbstractUGraphicHorizontalLine`): it keeps the translation
/// since it was set, draws separators at that height of the surface it was set on, and passes everything
/// else to that surface moved by the translation. Surfaces keep their drawer themselves (`StencilFrame`),
/// which measures the same height from absolute positions.
struct HorizontalLineLayer {
    ug: UGraphic,
    translate: UTranslate,
    drawer: Rc<dyn HorizontalLineDrawer>,
}

impl UGraphicLayer for HorizontalLineLayer {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    fn apply(&self, change: UChange) -> UGraphic {
        let (ug, translate) = match change {
            UChange::Translate(translate) => (self.ug.clone(), self.translate.compose(translate)),
            UChange::Clip(clip) => (
                self.ug
                    .apply(clip.translate(self.translate.dx, self.translate.dy)),
                self.translate,
            ),
            change => (self.ug.apply(change), self.translate),
        };
        UGraphic::from_layer(Self {
            ug,
            translate,
            drawer: self.drawer.clone(),
        })
    }

    fn draw(&self, _this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::HorizontalLine(line) => {
                self.drawer.draw_hline(&self.ug, line, self.translate.dy);
            }
            shape => self.ug.apply(self.translate).draw(shape),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;

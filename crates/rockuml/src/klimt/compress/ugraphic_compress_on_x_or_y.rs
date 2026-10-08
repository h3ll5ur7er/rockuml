use std::rc::Rc;

use super::CompressionMode;
use super::compression_transform::CompressionTransform;
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::{CenteredText, UShape};
use crate::klimt::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};

/// Draws what is drawn on it squeezed along one axis (`UGraphicCompressOnXorY`). It keeps the translation to
/// itself and hands the surface below transformed positions; rectangles shrink with the space they span,
/// lines are redrawn between their transformed ends, and centred titles are centred in their squeezed width.
pub(crate) struct UGraphicCompressOnXorY {
    mode: CompressionMode,
    ug: UGraphic,
    compression_transform: Rc<CompressionTransform>,
    translate: UTranslate,
}

impl UGraphicCompressOnXorY {
    pub(crate) fn create(
        mode: CompressionMode,
        ug: UGraphic,
        compression_transform: Rc<CompressionTransform>,
    ) -> UGraphic {
        UGraphic::from_layer(Self {
            mode,
            ug,
            compression_transform,
            translate: UTranslate::default(),
        })
    }

    fn ct(&self, v: f64) -> f64 {
        self.compression_transform.transform(v)
    }

    fn get_translate(&self, x: f64, y: f64) -> UTranslate {
        match self.mode {
            CompressionMode::OnX => UTranslate::new(self.ct(x), y),
            CompressionMode::OnY => UTranslate::new(x, self.ct(y)),
        }
    }

    fn draw_line(&self, x: f64, y: f64, dx: f64, dy: f64) {
        match self.mode {
            CompressionMode::OnX => self.draw_line_between(self.ct(x), y, self.ct(x + dx), y + dy),
            CompressionMode::OnY => self.draw_line_between(x, self.ct(y), x + dx, self.ct(y + dy)),
        }
    }

    /// Lines go down from their first end.
    fn draw_line_between(&self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if y1 > y2 {
            self.draw_line_between(x2, y2, x1, y1);
            return;
        }
        self.ug.apply(UTranslate::new(x1, y1)).draw(&UShape::Line {
            dx: x2 - x1,
            dy: y2 - y1,
        });
    }

    fn draw_centered_text(&self, x: f64, y: f64, centered_text: &CenteredText) {
        let real_space_width = match self.mode {
            CompressionMode::OnX => self.ct(x + centered_text.total_width) - self.ct(x),
            CompressionMode::OnY => centered_text.total_width,
        };
        let text = &centered_text.text;
        let text_width = text.calculate_dimension(self.ug.string_bounder()).width;
        let pos = (real_space_width - text_width) / 2.0;
        text.draw_u(
            &self
                .ug
                .apply(self.get_translate(x, y))
                .apply(UTranslate::new(pos, 0.0)),
        );
    }
}

impl UGraphicLayer for UGraphicCompressOnXorY {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    /// PlantUML refuses clips here; no tile clips what it draws.
    fn apply(&self, change: UChange) -> UGraphic {
        let (ug, translate) = match change {
            UChange::Translate(translate) => (self.ug.clone(), self.translate.compose(translate)),
            change => (self.ug.apply(change), self.translate),
        };
        UGraphic::from_layer(Self {
            mode: self.mode,
            ug,
            compression_transform: self.compression_transform.clone(),
            translate,
        })
    }

    fn draw(&self, _this: &UGraphic, shape: AnyShape<'_>) {
        let (x, y) = (self.translate.dx, self.translate.dy);
        match shape {
            AnyShape::Shape(UShape::Rectangle(rectangle)) => {
                let rectangle = match self.mode {
                    CompressionMode::OnX => {
                        let x2 = self.ct(x + rectangle.width);
                        rectangle.with_width(x2 - self.ct(x))
                    }
                    CompressionMode::OnY => {
                        let y2 = self.ct(y + rectangle.height);
                        rectangle.with_height(y2 - self.ct(y))
                    }
                };
                self.ug
                    .apply(self.get_translate(x, y))
                    .draw(&UShape::Rectangle(rectangle));
            }
            AnyShape::Shape(UShape::CenteredText(centered_text)) => {
                self.draw_centered_text(x, y, centered_text);
            }
            AnyShape::Shape(&UShape::Line { dx, dy }) => self.draw_line(x, y, dx, dy),
            shape => self.ug.apply(self.get_translate(x, y)).draw(shape),
        }
    }
}

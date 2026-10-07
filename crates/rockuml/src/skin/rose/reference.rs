use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::{Area, Component, TextualPart};

const CORNER_SIZE: f64 = 10.0;
const HEIGHT_FOOTER: f64 = 5.0;
const X_MARGIN: f64 = 2.0;

/// `ref over A, B : text`: a frame with a `ref` tab (PlantUML's `ComponentRoseReference`).
pub(crate) struct ComponentRoseReference {
    text: TextualPart,
    header: Box<dyn TextBlock>,
    body: Fashion,
    header_fashion: Fashion,
    position: Option<HorizontalAlignment>,
}

impl ComponentRoseReference {
    pub(crate) fn new(
        text: TextualPart,
        header: Box<dyn TextBlock>,
        body: Fashion,
        header_fashion: Fashion,
        position: Option<HorizontalAlignment>,
    ) -> Self {
        Self {
            text,
            header,
            body,
            header_fashion,
            position,
        }
    }

    fn header_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.header.calculate_dimension(string_bounder).height + 2.0
    }

    fn header_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.header.calculate_dimension(string_bounder).width + 30.0 + 15.0
    }

    /// The tab holding `ref`, its bottom right corner cut.
    fn corner(&self, width: f64, height: f64) -> Vec<USegment> {
        let round = self.body.round_corner;
        if round == 0.0 {
            return vec![
                USegment::MoveTo(0.0, 0.0),
                USegment::LineTo(width, 0.0),
                USegment::LineTo(width, height - CORNER_SIZE),
                USegment::LineTo(width - CORNER_SIZE, height),
                USegment::LineTo(0.0, height),
                USegment::LineTo(0.0, 0.0),
            ];
        }
        vec![
            USegment::MoveTo(round / 2.0, 0.0),
            USegment::LineTo(width, 0.0),
            USegment::LineTo(width, height - CORNER_SIZE),
            USegment::LineTo(width - CORNER_SIZE, height),
            USegment::LineTo(0.0, height),
            USegment::LineTo(0.0, round / 2.0),
            USegment::arc_to((round / 2.0, 0.0), round / 2.0, true),
        ]
    }
}

impl Component for ComponentRoseReference {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text
            .text_width(string_bounder)
            .max(self.header_width(string_bounder))
            + X_MARGIN * 2.0
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder) + self.header_height(string_bounder) + HEIGHT_FOOTER
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dimension = area.dimension;
        let string_bounder = ug.string_bounder();
        let header_width = self.header_width(string_bounder).trunc();
        let header_height = self.header_height(string_bounder).trunc();
        let mut rectangle = URectangle::new(
            dimension.width - X_MARGIN * 2.0,
            dimension.height - HEIGHT_FOOTER,
        );
        if self.body.round_corner != 0.0 {
            rectangle = rectangle.rounded(self.body.round_corner);
        }
        let ug = self.body.apply(ug);
        ug.translated(X_MARGIN, 0.0)
            .draw(&UShape::Rectangle(rectangle));
        let ug = self.header_fashion.apply(&ug);
        ug.translated(X_MARGIN, 0.0)
            .draw(&UShape::Path(self.corner(header_width, header_height)));
        let ug = ug.with_stroke(UStroke::SIMPLE);
        self.header.draw_u(&ug.translated(15.0, 2.0));
        let padding = self.text.padding();
        let text_x = match self.position {
            Some(HorizontalAlignment::Center) => {
                let text_width = self
                    .text
                    .text_block()
                    .calculate_dimension(string_bounder)
                    .width;
                (dimension.width - text_width) / 2.0
            }
            Some(HorizontalAlignment::Right) => {
                let text_width = self
                    .text
                    .text_block()
                    .calculate_dimension(string_bounder)
                    .width;
                dimension.width - text_width - padding.right - X_MARGIN
            }
            _ => padding.left + X_MARGIN,
        };
        self.text
            .text_block()
            .draw_u(&ug.translated(text_x, padding.top + header_height));
    }
}

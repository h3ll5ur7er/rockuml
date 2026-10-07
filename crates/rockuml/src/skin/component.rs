//! The drawable parts of a sequence diagram (PlantUML's `Component`, `AbstractComponent` and
//! `AbstractTextualComponent`).

use crate::creole::{CreoleParser, Display, SheetBlock1, SheetBlock2};
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::Style;

/// Where a component is drawn and how much room it gets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Area {
    pub dimension: XDimension2D,
    /// How far an activation level shifts a self message.
    pub delta_x1: f64,
    pub live_delta_size: f64,
    pub level: i32,
    /// How far the label of a message from the diagram's border moves.
    pub text_delta_x: f64,
}

impl Area {
    pub(crate) fn new(width: f64, height: f64) -> Self {
        Self {
            dimension: XDimension2D::new(width, height),
            ..Self::default()
        }
    }
}

/// Teoz draws twice: first the backgrounds of groups and boxes, then everything else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Context2D {
    pub is_background: bool,
}

pub(crate) trait Component {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64;

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64;

    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.preferred_width(string_bounder),
            self.preferred_height(string_bounder),
        )
    }

    fn padding_x(&self) -> f64 {
        0.0
    }

    fn padding_y(&self) -> f64 {
        0.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area);

    fn draw_background_internal(&self, _ug: &UGraphic, _area: &Area) {}

    /// Draws the background or the foreground, whichever `context` asks for, inside the padding.
    fn draw_u(&self, ug: &UGraphic, area: &Area, context: Context2D) {
        let ug = ug.translated(self.padding_x(), self.padding_y());
        if context.is_background {
            self.draw_background_internal(&ug, area);
        } else {
            self.draw_internal(&ug, area);
        }
    }
}

/// A message's arrow, which other parts of the diagram align with.
pub(crate) trait ArrowComponent: Component {
    fn start_point(&self, string_bounder: &dyn StringBounder, dimension: XDimension2D) -> XPoint2D;

    fn end_point(&self, string_bounder: &dyn StringBounder, dimension: XDimension2D) -> XPoint2D;

    /// How far below the component's top the arrow runs.
    fn y_point(&self, string_bounder: &dyn StringBounder) -> f64;

    fn pos_arrow(&self, string_bounder: &dyn StringBounder) -> f64;
}

/// The text of a component and the padding around it (PlantUML's `AbstractTextualComponent`).
pub(crate) struct TextualPart {
    text_block: Box<dyn TextBlock>,
    padding: ClockwiseTopRightBottomLeft,
}

impl TextualPart {
    pub(crate) fn new(
        text_block: Box<dyn TextBlock>,
        padding: ClockwiseTopRightBottomLeft,
    ) -> Self {
        Self {
            text_block,
            padding,
        }
    }

    pub(crate) fn text_block(&self) -> &dyn TextBlock {
        self.text_block.as_ref()
    }

    pub(crate) fn pure_text_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(string_bounder).width
    }

    /// The text's width with the horizontal padding; `pure_width` lets a component widen the text.
    pub(crate) fn text_width_from(&self, pure_width: f64) -> f64 {
        pure_width + self.padding.left + self.padding.right
    }

    pub(crate) fn text_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_width_from(self.pure_text_width(string_bounder))
    }

    pub(crate) fn text_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(string_bounder).height
            + self.padding.top
            + self.padding.bottom
    }

    pub(crate) fn padding(&self) -> ClockwiseTopRightBottomLeft {
        self.padding
    }
}

/// `Display.create0` for a component: creole text in the style's font and alignment. A single empty line
/// takes no room at all.
pub(crate) fn component_text(
    display: &Display,
    font: FontConfiguration,
    style: &Style,
) -> Box<dyn TextBlock> {
    if display.lines().len() == 1 && display.lines()[0].is_empty() {
        return Box::new(TextBlockEmpty);
    }
    let alignment = display
        .natural_alignment()
        .or_else(|| style.horizontal_alignment())
        .unwrap_or_default();
    creole_text(display.lines(), font, alignment)
}

pub(crate) fn creole_text(
    lines: &[String],
    font: FontConfiguration,
    alignment: HorizontalAlignment,
) -> Box<dyn TextBlock> {
    let sheet = CreoleParser::new(font, alignment).create_sheet(lines);
    Box::new(SheetBlock2::new(SheetBlock1::new(
        sheet,
        ClockwiseTopRightBottomLeft::none(),
    )))
}

/// Takes no room and draws nothing (`TextBlockEmpty`).
pub(crate) struct TextBlockEmpty;

impl TextBlock for TextBlockEmpty {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::default()
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}

/// Margins around a text block (`TextBlockUtils.withMargin`).
pub(crate) fn with_margin(
    block: Box<dyn TextBlock>,
    margin: ClockwiseTopRightBottomLeft,
) -> Box<dyn TextBlock> {
    Box::new(TextBlockMarged::new(block, margin))
}

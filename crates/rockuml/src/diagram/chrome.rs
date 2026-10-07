//! The warnings banner and the mainframe that `DiagramChromeFactory` puts around a diagram first.

use std::rc::Rc;

use crate::color::{Colors, HColor, XColor};
use crate::creole::Display;
use crate::klimt::big_frame::BigFrame;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::shape::{URectangle, UShape, UText};
use crate::klimt::sprite::SpriteContainer;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::creole_text;
use crate::style::Style;

/// Something the diagram's author should change, shown above the diagram (PlantUML's `Warning`).
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Warning(pub String);

const LINE_SPACING: f64 = 10.0;
const CORNER_RADIUS: f64 = 5.0;

fn warning_font() -> FontConfiguration {
    FontConfiguration::black_blue_true(UFont::monospace(10))
}

/// The warnings on a yellow banner (PlantUML's `WarningBannerBlock`).
struct WarningBannerBlock<'a> {
    warnings: &'a [Warning],
}

impl WarningBannerBlock<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let font = warning_font().font();
        let mut width: f64 = 0.0;
        let mut height = 0.0;
        for Warning(message) in self.warnings {
            let line = string_bounder.calculate_dimension(&font, message);
            width = width.max(line.width);
            height += line.height;
            height += LINE_SPACING;
        }
        if !self.warnings.is_empty() {
            height -= LINE_SPACING;
        }
        XDimension2D::new(width + 20.0, height + 10.0)
    }

    /// The banner spans `force_width` when that is wider than its text.
    fn draw_u(&self, ug: &UGraphic, force_width: f64) {
        let string_bounder = ug.string_bounder();
        let dim = self.calculate_dimension(string_bounder);
        let effective_width = dim.width.max(force_width);
        let rectangle =
            URectangle::new(effective_width - 10.0, dim.height - 5.0).rounded(CORNER_RADIUS);
        ug.with_backcolor(HColor::Simple(XColor::from_rgb(0xffffcc)))
            .with_color(HColor::Simple(XColor::from_rgb(0xffdd88)))
            .with_stroke(UStroke::with_thickness(3.0))
            .translated(3.0, 3.0)
            .draw(&UShape::Rectangle(rectangle));

        let font = warning_font();
        let mut ug_text = ug.with_color(HColor::BLACK).translated(10.0, 2.0);
        for Warning(message) in self.warnings {
            let height = string_bounder
                .calculate_dimension(&font.font(), message)
                .height;
            ug_text = ug_text.translated(0.0, height);
            ug_text.draw(&UShape::Text(UText::new(message, font.clone())));
            ug_text = ug_text.translated(0.0, LINE_SPACING);
        }
    }
}

/// A diagram below the banner of its warnings (`DiagramChromeFactory.addWarnings`).
pub(super) struct WithWarnings<'a> {
    banner: WarningBannerBlock<'a>,
    original: Box<dyn TextBlock + 'a>,
}

impl<'a> WithWarnings<'a> {
    pub(super) fn new(original: Box<dyn TextBlock + 'a>, warnings: &'a [Warning]) -> Self {
        Self {
            banner: WarningBannerBlock { warnings },
            original,
        }
    }
}

impl TextBlock for WithWarnings<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let banner = self.banner.calculate_dimension(string_bounder);
        let original = self.original.calculate_dimension(string_bounder);
        XDimension2D::new(
            banner.width.max(original.width),
            banner.height + original.height,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let total_width = self.calculate_dimension(string_bounder).width;
        self.banner.draw_u(ug, total_width);
        let banner_height = self.banner.calculate_dimension(string_bounder).height;
        self.original.draw_u(&ug.translated(0.0, banner_height));
    }
}

/// A diagram inside a big frame titled by the `mainframe` command (`DiagramChromeFactory.decorateWithFrame`).
pub(super) struct MainFrame<'a> {
    title: Box<dyn TextBlock>,
    original: Box<dyn TextBlock + 'a>,
    padding: ClockwiseTopRightBottomLeft,
    margin: ClockwiseTopRightBottomLeft,
    symbol_context: Fashion,
    string_bounder: Rc<dyn StringBounder>,
}

impl<'a> MainFrame<'a> {
    /// `style` is the mainframe's; `string_bounder` measures how far the diagram reaches.
    pub(super) fn new(
        original: Box<dyn TextBlock + 'a>,
        label: &Display,
        style: &Style,
        string_bounder: Rc<dyn StringBounder>,
        sprites: &dyn SpriteContainer,
    ) -> Self {
        let alignment = label
            .natural_alignment()
            .unwrap_or(HorizontalAlignment::Center);
        Self {
            title: creole_text(
                label.lines(),
                style.font_configuration(),
                alignment,
                0.0,
                sprites,
            ),
            original,
            padding: style.padding(),
            margin: style.margin(),
            symbol_context: style.symbol_context(&Colors::default()),
            string_bounder,
        }
    }

    fn frame(&self) -> BigFrame<'_> {
        BigFrame {
            title: self.title.as_ref(),
            original: self.original.as_ref(),
            padding: self.padding,
            symbol_context: &self.symbol_context,
            string_bounder: self.string_bounder.clone(),
        }
    }
}

impl TextBlock for MainFrame<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let frame = self.frame().calculate_dimension(string_bounder);
        let margin = self.margin;
        frame.delta(margin.left + margin.right, margin.top + margin.bottom)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let title_height = self.title.calculate_dimension(string_bounder).height;
        let padding = self.padding.inc_top(title_height + 10.0);
        let min_max = LimitFinder::min_max_of(self.original.as_ref(), self.string_bounder.clone());
        let dx = if min_max.min_x() < 0.0 {
            -min_max.min_x()
        } else {
            0.0
        };
        let dy = if min_max.min_y() < 0.0 {
            -min_max.min_y()
        } else {
            0.0
        };
        let margin = self.margin;
        self.frame().draw_u(&ug.translated(margin.left, margin.top));
        self.original.draw_u(&ug.translated(
            margin.left + (padding.left + dx),
            margin.top + (padding.top + dy),
        ));
    }
}

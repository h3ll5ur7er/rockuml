//! The image PlantUML draws instead of a diagram it cannot read (PlantUML's `PSystemError`).

use super::diagram_type::DiagramType;
use super::source::UmlSource;
use super::{Diagram, ExportSettings, NotYetPorted};
use crate::color::HColor;
use crate::creole::{CreoleMode, CreoleParser, SheetBlock1};
use crate::klimt::blocks::{Marged, RawText, Vertical, WithBackcolor};
use crate::klimt::font::{FontConfiguration, FontStyle, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{UImage, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::text::StringLocated;

/// What PlantUML's error images say they were made by.
const PLANTUML_DESCRIPTION: &str = "PlantUML version 1.2026.8 / 149874a [2026-09-05 15:59:17 UTC]";

const GREEN: HColor = HColor::Simple(crate::color::XColor::rgb(0x33, 0xFF, 0x02));
const RED: HColor = HColor::Simple(crate::color::XColor::rgb(0xFF, 0x00, 0x00));

pub struct ErrorDiagram {
    source: UmlSource,
    /// The lines read up to the error, the faulty one last.
    trace: Vec<StringLocated>,
    message: String,
}

impl ErrorDiagram {
    /// `diagram_type` names the diagram the lines were read as, which the message mentions.
    pub fn new(
        source: UmlSource,
        trace: Vec<StringLocated>,
        error: &str,
        diagram_type: Option<DiagramType>,
    ) -> Self {
        let message = match diagram_type {
            Some(diagram_type) => format!(
                "{error} (Assumed diagram type: {})",
                diagram_type.human_readable_name()
            ),
            None => error.to_owned(),
        };
        Self {
            source,
            trace,
            message,
        }
    }

    /// Where the faulty line is, and where each file around it was included from.
    fn location_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut location = self.trace.last().map(StringLocated::location);
        while let Some(current) = location {
            lines.push(format!(
                "[From {} (line {}) ]",
                current.description(),
                current.position() + 1
            ));
            location = current.parent();
        }
        lines
    }

    /// A blank line, then the lines read, the middle skipped when there are more than 40.
    fn body_lines(&self) -> Vec<String> {
        let shorten = |line: &StringLocated| {
            let text = line.text();
            let units: Vec<u16> = text.encode_utf16().collect();
            if units.len() > 120 {
                format!("{} ...", String::from_utf16_lossy(&units[..120]))
            } else {
                text.to_owned()
            }
        };
        let mut lines = vec![" ".to_owned()];
        if self.trace.len() > 40 {
            lines.extend(self.trace[..5].iter().map(shorten));
            lines.push("...".to_owned());
            lines.push(format!("... ( skipping {} lines )", self.trace.len() - 25));
            lines.push("...".to_owned());
            lines.extend(self.trace[self.trace.len() - 20..].iter().map(shorten));
        } else {
            lines.extend(self.trace.iter().map(shorten));
        }
        lines
    }
}

fn sans_serif(size: i32, color: HColor) -> FontConfiguration {
    FontConfiguration::new(UFont::new("SansSerif", UFontFace::NORMAL, size), color, 8)
}

/// Margins given as left, right, top, bottom, like PlantUML's `withMargin`.
fn margins(left: f64, right: f64, top: f64, bottom: f64) -> ClockwiseTopRightBottomLeft {
    ClockwiseTopRightBottomLeft {
        top,
        right,
        bottom,
        left,
    }
}

impl Diagram for ErrorDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(&self) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let bold = |color| sans_serif(14, color).with_style(FontStyle::Bold);
        let mut body = self.body_lines();
        let faulty = body.pop().expect("the body has the blank line at least");
        let location = WithBackcolor::new(
            Marged::new(
                RawText::new(self.location_lines(), bold(HColor::BLACK)),
                margins(1.0, 1.0, 1.0, 4.0),
            ),
            GREEN,
        );
        let header = Marged::new(
            RawText::new(
                [PLANTUML_DESCRIPTION],
                sans_serif(12, GREEN)
                    .with_style(FontStyle::Bold)
                    .with_style(FontStyle::Italic),
            ),
            margins(0.0, 2.0, 0.0, 8.0),
        );
        let wavy = bold(GREEN)
            .with_style(FontStyle::Wave)
            .with_extended_color(RED);
        // Stacked pairwise like PlantUML, which matters: a background spans the width of its own pair.
        let below: [Box<dyn TextBlock>; 3] = [
            Box::new(RawText::new(body, bold(GREEN))),
            Box::new(RawText::new([faulty], wavy)),
            Box::new(RawText::new([format!(" {}", self.message)], bold(RED))),
        ];
        let report = below
            .into_iter()
            .fold(Box::new(location) as Box<dyn TextBlock>, |above, next| {
                Box::new(Vertical::new(vec![above, next], HorizontalAlignment::Left))
            });
        let image = Vertical::new(vec![Box::new(header), report], HorizontalAlignment::Left);
        let image = Marged::new(image, ClockwiseTopRightBottomLeft::same(5.0));
        let image = WithBackcolor::new(image, HColor::BLACK);
        if self.source.lines().len() < 5 {
            let blocks: Vec<Box<dyn TextBlock>> = vec![Box::new(Welcome::new()), Box::new(image)];
            return Ok(Box::new(Vertical::new(blocks, HorizontalAlignment::Left)));
        }
        Ok(Box::new(image))
    }

    fn export_settings(&self) -> ExportSettings {
        ExportSettings::without_skin(self.source.seed())
    }

    fn is_error(&self) -> bool {
        true
    }
}

/// The help PlantUML shows above errors in very short sources, with its logo in the top right corner.
struct Welcome {
    text: SheetBlock1,
}

impl Welcome {
    const MARGIN: f64 = 5.0;
    const LOGO_WIDTH: f64 = 80.0;
    const LOGO_HEIGHT: f64 = 71.0;
    const LOGO_PADDING: f64 = 30.0;

    fn new() -> Self {
        let lines = [
            "<b>Welcome to PlantUML!",
            " ",
            "You can start with a simple UML Diagram like:",
            " ",
            "\"\"Bob->Alice: Hello\"\"",
            " ",
            "Or",
            " ",
            "\"\"class Example\"\"",
            " ",
            "You will find more information about PlantUML syntax on <u>https://plantuml.com</u>",
            " ",
            "(Details by typing \"\"license\"\" keyword)",
            " ",
        ];
        let font = sans_serif(12, HColor::BLACK);
        let sheet = CreoleParser::with_mode(
            font,
            HorizontalAlignment::Left,
            CreoleMode::FullButUnderscore,
        )
        .create_sheet(&lines);
        Self {
            text: SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none()),
        }
    }
}

impl TextBlock for Welcome {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let margins = 2.0 * Self::MARGIN;
        self.text
            .calculate_dimension(string_bounder)
            .delta(Self::LOGO_PADDING + Self::LOGO_WIDTH, 0.0)
            .delta(margins, margins)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.translated(Self::MARGIN, Self::MARGIN);
        let inner_width = self.calculate_dimension(ug.string_bounder()).width - 2.0 * Self::MARGIN;
        self.text.draw_u(&ug.with_color(HColor::BLACK));
        ug.translated(inner_width - Self::LOGO_WIDTH - 1.0, 1.0)
            .draw(&UShape::Image(UImage {
                png: crate::assets::get("images/plantuml-logo.png").expect("the logo is embedded"),
                width: Self::LOGO_WIDTH,
                height: Self::LOGO_HEIGHT,
            }));
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(HColor::WHITE)
    }
}

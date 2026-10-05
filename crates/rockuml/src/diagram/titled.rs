//! What every diagram with a skin shares: skinparams and styles, and the title drawn around it
//! (PlantUML's `TitledDiagram` and `DiagramChromeFactory`).

use super::ExportSettings;
use crate::creole::{CreoleParser, Display, SheetBlock1};
use crate::klimt::blocks::{Bordered, Decorated, Marged};
use crate::klimt::font::{FontConfiguration, UFont, UFontFace};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::ugraphic::UStroke;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

pub struct Titled {
    pub skin: SkinParam,
    /// The diagram's own style name, like `saltDiagram`, for the styles of its legend.
    diagram_style: SName,
    title: Option<Display>,
    caption: Option<Display>,
    legend: Option<(Display, HorizontalAlignment, VerticalAlignment)>,
    header: Option<(Display, HorizontalAlignment)>,
    footer: Option<(Display, HorizontalAlignment)>,
}

/// Where a legend goes: above or below the diagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Bottom,
}

/// A diagram built from commands that apply to every titled diagram.
pub trait TitledDiagram {
    fn titled(&mut self) -> &mut Titled;
}

impl Titled {
    pub fn new(diagram_style: SName) -> Self {
        Self {
            skin: SkinParam::default(),
            diagram_style,
            title: None,
            caption: None,
            legend: None,
            header: None,
            footer: None,
        }
    }

    /// A blank title is ignored.
    pub fn set_title(&mut self, title: Display) {
        if !title.is_white() {
            self.title = Some(title);
        }
    }

    pub fn set_caption(&mut self, caption: Display) {
        self.caption = Some(caption);
    }

    pub fn set_legend(
        &mut self,
        legend: Display,
        horizontal: HorizontalAlignment,
        vertical: VerticalAlignment,
    ) {
        self.legend = Some((legend, horizontal, vertical));
    }

    pub fn set_header(&mut self, header: Display, alignment: HorizontalAlignment) {
        self.header = Some((header, alignment));
    }

    pub fn set_footer(&mut self, footer: Display, alignment: HorizontalAlignment) {
        self.footer = Some((footer, alignment));
    }

    /// Where headers or footers go when the command does not say: as their style aligns text.
    pub fn default_alignment(&self, part: SName) -> HorizontalAlignment {
        self.document_style(Some(part))
            .value(PName::HorizontalAlignment)
            .as_horizontal_alignment()
            .unwrap_or_default()
    }

    fn document_style(&self, name: Option<SName>) -> Style {
        let mut names = vec![SName::Root, SName::Document];
        names.extend(name);
        self.skin
            .merged_style(&StyleSignature::of(&names))
            .expect("the skin styles the document")
    }

    /// The diagram's drawing with its legend, title, caption, header and footer around it, added in
    /// PlantUML's order.
    pub fn add_chrome<'a>(&'a self, drawing: Box<dyn TextBlock + 'a>) -> Box<dyn TextBlock + 'a> {
        let mut result = drawing;
        if let Some((legend, horizontal, vertical)) = &self.legend {
            let names = [
                SName::Root,
                SName::Document,
                self.diagram_style,
                SName::Legend,
            ];
            let style = self
                .skin
                .merged_style(&StyleSignature::of(&names))
                .expect("the skin styles the document");
            let block = Some((bordered_text(legend, &style), *horizontal));
            result = match vertical {
                VerticalAlignment::Top => Box::new(Decorated::new(result, block, None)),
                VerticalAlignment::Bottom => Box::new(Decorated::new(result, None, block)),
            };
        }
        if let Some(title) = &self.title {
            let block = bordered_text(title, &self.document_style(Some(SName::Title)));
            result = Box::new(Decorated::new(
                result,
                Some((block, HorizontalAlignment::Center)),
                None,
            ));
        }
        if let Some(caption) = &self.caption {
            let block = bordered_text(caption, &self.document_style(Some(SName::Caption)));
            result = Box::new(Decorated::new(
                result,
                None,
                Some((block, HorizontalAlignment::Center)),
            ));
        }
        let ribbon = |part: &Option<(Display, HorizontalAlignment)>, name| {
            part.as_ref()
                .filter(|(display, _)| !display.lines().is_empty())
                .map(|(display, alignment)| {
                    (
                        bordered_text(display, &self.document_style(Some(name))),
                        *alignment,
                    )
                })
        };
        let header = ribbon(&self.header, SName::Header);
        let footer = ribbon(&self.footer, SName::Footer);
        if header.is_some() || footer.is_some() {
            result = Box::new(Decorated::new(result, header, footer));
        }
        result
    }

    /// The document style's margin if it sets one, otherwise the diagram's own default.
    pub fn export_settings(&self, seed: i64, default_margin: f64) -> ExportSettings {
        let document = self.document_style(None);
        let margin = if document.has_value(PName::Margin) {
            margin_of(&document, PName::Margin)
        } else {
            ClockwiseTopRightBottomLeft::same(default_margin)
        };
        ExportSettings {
            margin,
            seed,
            svg_link_target: Some(
                self.skin
                    .value("svglinktarget")
                    .unwrap_or_else(|| "_top".to_owned()),
            ),
            preserve_aspect_ratio: self
                .skin
                .value("preserveaspectratio")
                .unwrap_or_else(|| "none".to_owned()),
        }
    }
}

/// `Style.createTextBlockBordered`: the text in the style's font, padded, bordered, then given margins.
fn bordered_text<'a>(display: &Display, style: &Style) -> Box<dyn TextBlock + 'a> {
    let alignment = display.natural_alignment().unwrap_or_else(|| {
        style
            .value(PName::HorizontalAlignment)
            .as_horizontal_alignment()
            .unwrap_or_default()
    });
    let sheet =
        CreoleParser::new(font_configuration(style), alignment).create_sheet(display.lines());
    let text = SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none());
    let bordered = Bordered::new(
        text,
        stroke(style),
        style.value(PName::LineColor).as_color(),
        style.value(PName::BackGroundColor).as_color(),
        f64::from(style.value(PName::RoundCorner).as_int()),
        margin_of(style, PName::Padding),
    );
    Box::new(Marged::new(bordered, margin_of(style, PName::Margin)))
}

/// `Style.getFontConfiguration`.
pub fn font_configuration(style: &Style) -> FontConfiguration {
    let size = match style.value(PName::FontSize).as_int_or_minus_one() {
        -1 => 14,
        size => size,
    };
    let mut face = style.value(PName::FontStyle).as_font_face();
    let weight = style.value(PName::FontWeight).as_font_face();
    if weight.weight != 400 {
        face = UFontFace {
            weight: weight.weight,
            ..face
        };
    }
    let font = UFont::new(&style.value(PName::FontName).as_string(), face, size);
    FontConfiguration::new(font, style.value(PName::FontColor).as_color(), 8)
}

/// `Style.getStroke`: the line thickness, dashed by a `visible-space` line style.
fn stroke(style: &Style) -> UStroke {
    let thickness = style.value(PName::LineThickness).as_double();
    let dash = style.value(PName::LineStyle).as_string();
    let mut lengths = dash
        .split(['-', ';', ','])
        .filter(|part| !part.is_empty())
        .map(|part| crate::java::trim(part).parse::<f64>());
    match lengths.next() {
        Some(Ok(visible)) => {
            let space = match lengths.next() {
                Some(Ok(space)) => space,
                Some(Err(_)) => return UStroke::with_thickness(thickness),
                None => visible,
            };
            UStroke {
                dash_visible: visible,
                dash_space: space,
                thickness,
            }
        }
        _ => UStroke::with_thickness(thickness),
    }
}

/// `ClockwiseTopRightBottomLeft.read`: one to four whole numbers, CSS-style; anything else is none.
fn margin_of(style: &Style, property: PName) -> ClockwiseTopRightBottomLeft {
    let text = style.value(property).as_string();
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit() || c == ' ') {
        return ClockwiseTopRightBottomLeft::none();
    }
    let numbers: Result<Vec<f64>, _> = text
        .split(' ')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<i32>().map(f64::from))
        .collect();
    let Ok(numbers) = numbers else {
        return ClockwiseTopRightBottomLeft::none();
    };
    let (top, right, bottom, left) = match numbers.as_slice() {
        [all] => (*all, *all, *all, *all),
        [vertical, horizontal] => (*vertical, *horizontal, *vertical, *horizontal),
        [top, horizontal, bottom] => (*top, *horizontal, *bottom, *horizontal),
        [top, right, bottom, left] => (*top, *right, *bottom, *left),
        _ => return ClockwiseTopRightBottomLeft::none(),
    };
    ClockwiseTopRightBottomLeft {
        top,
        right,
        bottom,
        left,
    }
}

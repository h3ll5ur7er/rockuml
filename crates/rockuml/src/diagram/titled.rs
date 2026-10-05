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

#[derive(Default)]
pub struct Titled {
    pub skin: SkinParam,
    title: Option<Display>,
}

/// A diagram built from commands that apply to every titled diagram.
pub trait TitledDiagram {
    fn titled(&mut self) -> &mut Titled;
}

impl Titled {
    /// A blank title is ignored.
    pub fn set_title(&mut self, title: Display) {
        if !title.is_white() {
            self.title = Some(title);
        }
    }

    fn document_style(&self, name: Option<SName>) -> Style {
        let mut names = vec![SName::Root, SName::Document];
        names.extend(name);
        self.skin
            .merged_style(&StyleSignature::of(&names))
            .expect("the skin styles the document")
    }

    /// The diagram's drawing with the title around it.
    pub fn add_chrome<'a>(&'a self, drawing: Box<dyn TextBlock + 'a>) -> Box<dyn TextBlock + 'a> {
        let Some(title) = &self.title else {
            return drawing;
        };
        let style = self.document_style(Some(SName::Title));
        let title_block = bordered_text(title, &style);
        Box::new(Decorated::new(
            drawing,
            Some((title_block, HorizontalAlignment::Center)),
            None,
        ))
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

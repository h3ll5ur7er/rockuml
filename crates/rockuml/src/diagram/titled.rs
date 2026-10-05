//! What every diagram with a skin shares: skinparams and styles, and the title drawn around it
//! (PlantUML's `TitledDiagram` and `DiagramChromeFactory`).

use super::scale::Scale;
use super::{DEFAULT_DPI, ExportSettings, parse_digits};
use crate::creole::{CreoleParser, Display, SheetBlock1, SheetBlock2};
use crate::klimt::blocks::{Bordered, Decorated, Decoration, Marged};
use crate::klimt::font::{FontConfiguration, UFont, UFontFace};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::ugraphic::UStroke;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};
use crate::text::LineLocation;

pub struct Titled {
    pub skin: SkinParam,
    /// The diagram's own style name, like `saltDiagram`, for the styles of its legend and background.
    diagram_style: SName,
    /// The name SVG documents announce the diagram type with, like `SALT`.
    diagram_type: &'static str,
    title: Option<Positioned>,
    caption: Option<Positioned>,
    legend: Option<(Positioned, VerticalAlignment)>,
    header: Option<Positioned>,
    footer: Option<Positioned>,
    scale: Option<Scale>,
}

/// A text around the diagram, where it goes, and the source line that wrote it (PlantUML's `DisplayPositioned`).
pub struct Positioned {
    pub display: Display,
    pub alignment: HorizontalAlignment,
    pub location: Option<LineLocation>,
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
    pub fn new(diagram_style: SName, diagram_type: &'static str) -> Self {
        Self {
            skin: SkinParam::default(),
            diagram_style,
            diagram_type,
            title: None,
            caption: None,
            legend: None,
            header: None,
            footer: None,
            scale: None,
        }
    }

    pub fn set_scale(&mut self, scale: Scale) {
        self.scale = Some(scale);
    }

    /// A blank title is ignored.
    pub fn set_title(&mut self, title: Display, location: &LineLocation) {
        if !title.is_white() {
            self.title = Some(Positioned::centered(title, location));
        }
    }

    pub fn set_caption(&mut self, caption: Display, location: &LineLocation) {
        self.caption = Some(Positioned::centered(caption, location));
    }

    pub fn set_legend(&mut self, legend: Positioned, vertical: VerticalAlignment) {
        self.legend = Some((legend, vertical));
    }

    pub fn set_header(&mut self, header: Positioned) {
        self.header = Some(header);
    }

    pub fn set_footer(&mut self, footer: Positioned) {
        self.footer = Some(footer);
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
        self.style(&names)
    }

    fn style(&self, names: &[SName]) -> Style {
        self.skin
            .merged_style(&StyleSignature::of(names))
            .expect("the skin styles the document")
    }

    /// The diagram's drawing with its legend, title, caption, header and footer around it, added in
    /// PlantUML's order.
    pub fn add_chrome<'a>(&'a self, drawing: Box<dyn TextBlock + 'a>) -> Box<dyn TextBlock + 'a> {
        let mut result = drawing;
        if let Some((legend, vertical)) = &self.legend {
            let style = self.style(&[
                SName::Root,
                SName::Document,
                self.diagram_style,
                SName::Legend,
            ]);
            let decoration = Some(legend.decoration("legend", &style));
            result = match vertical {
                VerticalAlignment::Top => Box::new(Decorated::new(result, decoration, None)),
                VerticalAlignment::Bottom => Box::new(Decorated::new(result, None, decoration)),
            };
        }
        if let Some(title) = &self.title {
            let style = self.document_style(Some(SName::Title));
            result = Box::new(Decorated::new(
                result,
                Some(title.decoration("title", &style)),
                None,
            ));
        }
        if let Some(caption) = &self.caption {
            let style = self.document_style(Some(SName::Caption));
            result = Box::new(Decorated::new(
                result,
                None,
                Some(caption.decoration("caption", &style)),
            ));
        }
        let ribbon = |part: &Option<Positioned>, name, class| {
            part.as_ref()
                .filter(|part| !part.display.lines().is_empty())
                .map(|part| part.decoration(class, &self.document_style(Some(name))))
        };
        let header = ribbon(&self.header, SName::Header, "header");
        let footer = ribbon(&self.footer, SName::Footer, "footer");
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
        let background = self.style(&[SName::Root, SName::Document, self.diagram_style]);
        ExportSettings {
            margin,
            seed,
            backcolor: Some(background.value(PName::BackGroundColor).as_color()),
            diagram_type: Some(self.diagram_type),
            scale: self.scale,
            dpi: self
                .skin
                .value("dpi")
                .as_deref()
                .and_then(parse_digits)
                .unwrap_or(DEFAULT_DPI),
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

impl Positioned {
    fn centered(display: Display, location: &LineLocation) -> Self {
        Self {
            display,
            alignment: HorizontalAlignment::Center,
            location: Some(location.clone()),
        }
    }

    fn decoration<'a>(&self, class: &str, style: &Style) -> Decoration<'a> {
        let mut group = UGroup::at(self.location.as_ref());
        group.put(UGroupType::Class, class);
        Decoration {
            block: bordered_text(&self.display, style),
            alignment: self.alignment,
            group,
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
    let text = SheetBlock2::new(SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none()));
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
    FontConfiguration::new(font, style.value(PName::FontColor).as_color(), 8).with_hyperlink_style(
        style.value(PName::HyperLinkColor).as_color(),
        stroke_of(
            style,
            PName::HyperlinkUnderlineThickness,
            PName::HyperlinkUnderlineStyle,
        ),
    )
}

/// `Style.getStroke`: the line thickness, dashed by a `visible-space` line style.
fn stroke(style: &Style) -> UStroke {
    stroke_of(style, PName::LineThickness, PName::LineStyle)
}

fn stroke_of(style: &Style, thickness: PName, line_style: PName) -> UStroke {
    let thickness = style.value(thickness).as_double();
    let dash = style.value(line_style).as_string();
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

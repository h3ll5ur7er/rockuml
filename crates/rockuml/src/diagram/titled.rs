//! What every diagram with a skin shares: skinparams and styles, and the title drawn around it
//! (PlantUML's `TitledDiagram` and `DiagramChromeFactory`).

use std::rc::Rc;

use super::chrome::{MainFrame, Warning, WithWarnings};
use super::scale::Scale;
use super::{DEFAULT_DPI, ExportSettings, UmlSource, parse_digits};
use crate::creole::{CreoleParser, Display, SheetBlock1, SheetBlock2};
use crate::klimt::blocks::{DecorateEntityImage, Decoration, TextBlockBordered, TextBlockMarged};

use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::sprite::SpriteContainer;

use crate::klimt::font::StringBounder;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};
use crate::text::LineLocation;

pub(super) struct Titled {
    pub skin: SkinParam,
    pub pragma: Pragma,
    /// The diagram's own style name, like `saltDiagram`, for the styles of its legend and background.
    diagram_style: SName,
    /// The name SVG documents announce the diagram type with, like `SALT`.
    diagram_type: &'static str,
    title: Option<Positioned>,
    caption: Option<Positioned>,
    legend: Option<(Positioned, VerticalAlignment)>,
    header: Option<Positioned>,
    footer: Option<Positioned>,
    mainframe: Option<Display>,
    scale: Option<Scale>,
    /// Without repeats, in the order they came.
    warnings: Vec<Warning>,
}

/// `!pragma` settings PlantUML knows (PlantUML's `Pragma`); others are ignored.
#[derive(Default)]
pub(super) struct Pragma {
    values: Vec<(PragmaKey, Option<String>)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PragmaKey {
    SequenceMessageSpan,
    Teoz,
}

impl PragmaKey {
    /// Names match ignoring case and anything but letters, as in PlantUML.
    fn named(name: &str) -> Option<Self> {
        let simplified: String = name
            .chars()
            .filter(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_lowercase())
            .collect();
        match simplified.as_str() {
            "sequencemessagespan" => Some(Self::SequenceMessageSpan),
            "teoz" => Some(Self::Teoz),
            _ => None,
        }
    }

    /// The value of the pragma written without one.
    fn default_value(self) -> Option<&'static str> {
        match self {
            Self::Teoz => Some("true"),
            Self::SequenceMessageSpan => None,
        }
    }
}

impl Pragma {
    pub(super) fn define(&mut self, name: &str, value: Option<&str>) {
        let Some(key) = PragmaKey::named(name) else {
            return;
        };
        let value = value.or(key.default_value()).map(str::to_owned);
        self.values.retain(|(known, _)| *known != key);
        self.values.push((key, value));
    }

    /// `true` or `on`.
    pub(super) fn is_true(&self, key: PragmaKey) -> bool {
        self.values.iter().any(|(known, value)| {
            *known == key
                && value.as_deref().is_some_and(|value| {
                    value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("on")
                })
        })
    }
}

/// A text around the diagram, where it goes, and the source line that wrote it (PlantUML's `DisplayPositioned`).
pub(super) struct Positioned {
    pub display: Display,
    pub alignment: HorizontalAlignment,
    pub location: Option<LineLocation>,
}

/// Where a legend goes: above or below the diagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VerticalAlignment {
    Top,
    Bottom,
}

/// A diagram built from commands that apply to every titled diagram.
pub(super) trait TitledDiagram {
    fn titled(&mut self) -> &mut Titled;
}

impl Titled {
    /// The skin draws the images of `source`.
    pub(super) fn new(
        diagram_style: SName,
        diagram_type: &'static str,
        source: &UmlSource,
    ) -> Self {
        Self {
            skin: SkinParam::new(source),
            pragma: Pragma::default(),
            diagram_style,
            diagram_type,
            title: None,
            caption: None,
            legend: None,
            header: None,
            footer: None,
            mainframe: None,
            scale: None,
            warnings: Vec::new(),
        }
    }

    pub(super) fn add_warning(&mut self, warning: Warning) {
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }

    pub(super) fn set_mainframe(&mut self, label: Display) {
        self.mainframe = Some(label);
    }

    pub(super) fn set_scale(&mut self, scale: Scale) {
        self.scale = Some(scale);
    }

    /// A blank title is ignored.
    pub(super) fn set_title(&mut self, title: Display, location: &LineLocation) {
        if !title.is_white() {
            self.title = Some(Positioned::centered(title, location));
        }
    }

    pub(super) fn set_caption(&mut self, caption: Display, location: &LineLocation) {
        self.caption = Some(Positioned::centered(caption, location));
    }

    pub(super) fn set_legend(&mut self, legend: Positioned, vertical: VerticalAlignment) {
        self.legend = Some((legend, vertical));
    }

    pub(super) fn set_header(&mut self, header: Positioned) {
        self.header = Some(header);
    }

    pub(super) fn set_footer(&mut self, footer: Positioned) {
        self.footer = Some(footer);
    }

    /// Where headers or footers go when the command does not say: as their style aligns text.
    pub(super) fn default_alignment(&self, part: SName) -> HorizontalAlignment {
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

    /// The diagram's drawing with its warnings, mainframe, legend, title, caption, header and footer around
    /// it, added in PlantUML's order. `string_bounder` measures the drawing for its mainframe.
    pub(super) fn add_chrome<'a>(
        &'a self,
        drawing: Box<dyn TextBlock + 'a>,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Box<dyn TextBlock + 'a> {
        self.add_chrome_titled(drawing, string_bounder, self.title.as_ref())
    }

    /// Like [`Self::add_chrome`], with another title, as the pages of sequence diagrams have.
    pub(super) fn add_chrome_titled<'a>(
        &'a self,
        drawing: Box<dyn TextBlock + 'a>,
        string_bounder: &Rc<dyn StringBounder>,
        title: Option<&'a Positioned>,
    ) -> Box<dyn TextBlock + 'a> {
        let mut result = drawing;
        if !self.warnings.is_empty() {
            result = Box::new(WithWarnings::new(result, &self.warnings));
        }
        if let Some(label) = &self.mainframe {
            let style = self.document_style(Some(SName::Mainframe));
            result = Box::new(MainFrame::new(
                result,
                label,
                &style,
                string_bounder.clone(),
                &self.skin,
            ));
        }
        if let Some((legend, vertical)) = &self.legend {
            let style = self.style(&[
                SName::Root,
                SName::Document,
                self.diagram_style,
                SName::Legend,
            ]);
            let decoration = Some(legend.decoration("legend", &style, &self.skin));
            result = match vertical {
                VerticalAlignment::Top => {
                    Box::new(DecorateEntityImage::new(result, decoration, None))
                }
                VerticalAlignment::Bottom => {
                    Box::new(DecorateEntityImage::new(result, None, decoration))
                }
            };
        }
        if let Some(title) = title {
            let style = self.document_style(Some(SName::Title));
            result = Box::new(DecorateEntityImage::new(
                result,
                Some(title.decoration("title", &style, &self.skin)),
                None,
            ));
        }
        if let Some(caption) = &self.caption {
            let style = self.document_style(Some(SName::Caption));
            result = Box::new(DecorateEntityImage::new(
                result,
                None,
                Some(caption.decoration("caption", &style, &self.skin)),
            ));
        }
        let ribbon = |part: &Option<Positioned>, name, class| {
            part.as_ref()
                .filter(|part| !part.display.lines().is_empty())
                .map(|part| part.decoration(class, &self.document_style(Some(name)), &self.skin))
        };
        let header = ribbon(&self.header, SName::Header, "header");
        let footer = ribbon(&self.footer, SName::Footer, "footer");
        if header.is_some() || footer.is_some() {
            result = Box::new(DecorateEntityImage::new(result, header, footer));
        }
        result
    }

    /// The document style's margin if it sets one, otherwise the diagram's own default.
    pub(super) fn export_settings(&self, seed: i64, default_margin: f64) -> ExportSettings {
        let document = self.document_style(None);
        let margin = if document.has_value(PName::Margin) {
            document.margin()
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

    fn decoration<'a>(
        &self,
        class: &str,
        style: &Style,
        sprites: &dyn SpriteContainer,
    ) -> Decoration<'a> {
        let mut group = UGroup::at(self.location.as_ref());
        group.put(UGroupType::Class, class);
        Decoration {
            block: bordered_text(&self.display, style, sprites),
            alignment: self.alignment,
            group,
        }
    }
}

/// `Style.createTextBlockBordered`: the text in the style's font, padded, bordered, then given margins.
fn bordered_text<'a>(
    display: &Display,
    style: &Style,
    sprites: &dyn SpriteContainer,
) -> Box<dyn TextBlock + 'a> {
    let alignment = display.natural_alignment().unwrap_or_else(|| {
        style
            .value(PName::HorizontalAlignment)
            .as_horizontal_alignment()
            .unwrap_or_default()
    });
    let sheet = CreoleParser::new(style.font_configuration(), alignment, sprites)
        .create_sheet(display.lines());
    let text = SheetBlock2::new(SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none()));
    let bordered = TextBlockBordered::new(
        text,
        style.stroke(),
        style.value(PName::LineColor).as_color(),
        style.value(PName::BackGroundColor).as_color(),
        f64::from(style.value(PName::RoundCorner).as_int()),
        style.padding(),
    );
    Box::new(TextBlockMarged::new(bordered, style.margin()))
}

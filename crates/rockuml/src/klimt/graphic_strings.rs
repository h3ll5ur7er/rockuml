//! Lines of creole text drawn black on white, with an optional image beside or below them: the text of
//! PlantUML's reference cards and messages (PlantUML's `GraphicStrings`).

use super::blocks::TextBlockRaw;
use super::font::{FontConfiguration, FontStyle, StringBounder, UFont, UFontFace};
use super::geom::XDimension2D;
use super::image::PortableImage;
use super::shape::{UImage, UShape};
use super::sprite::SpriteContainerEmpty;
use super::ugraphic::UGraphic;
use super::{HorizontalAlignment, TextBlock};
use crate::color::HColor;
use crate::creole::{CreoleMode, Display};

/// Where the image goes (PlantUML's `GraphicPosition`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphicPosition {
    BackgroundCornerBottomRight,
    BackgroundCornerTopRight,
}

pub(crate) struct GraphicStrings {
    text: Box<dyn TextBlock>,
    color: HColor,
    background: HColor,
    image: Option<(PortableImage, GraphicPosition)>,
}

impl GraphicStrings {
    const MARGIN: f64 = 5.0;
    const IMAGE_PADDING: f64 = 30.0;

    /// `createBlackOnWhite`.
    pub(crate) fn create_black_on_white(strings: &[impl AsRef<str>]) -> Self {
        Self::new(strings, CreoleMode::Full, None)
    }

    /// `createBlackOnWhite` with an image or without, which reads no `__underline__`.
    pub(crate) fn create_black_on_white_with(
        strings: &[impl AsRef<str>],
        image: Option<(PortableImage, GraphicPosition)>,
    ) -> Self {
        Self::new(strings, CreoleMode::FullButUnderscore, image)
    }

    /// `createForError` without red: the lines in bold green `SansSerif` 14 on black, read without creole, as
    /// PlantUML reports what went wrong outside the diagram.
    pub(crate) fn create_for_error(strings: &[&str]) -> Self {
        let color = HColor::MY_GREEN;
        let font = FontConfiguration::new(
            UFont::new("SansSerif", UFontFace::NORMAL, 14),
            color.clone(),
            8,
        )
        .with_style(FontStyle::Bold);
        Self {
            text: Box::new(TextBlockRaw::new(strings.iter().copied(), font)),
            color,
            background: HColor::BLACK,
            image: None,
        }
    }

    fn new(
        strings: &[impl AsRef<str>],
        mode: CreoleMode,
        image: Option<(PortableImage, GraphicPosition)>,
    ) -> Self {
        let text = Display::create(strings.iter().map(AsRef::as_ref)).create0(
            &sans_serif12(HColor::BLACK),
            HorizontalAlignment::Left,
            &SpriteContainerEmpty,
            0.0,
            mode,
        );
        Self {
            text: Box::new(text),
            color: HColor::BLACK,
            background: HColor::WHITE,
            image,
        }
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim = self.text.calculate_dimension(string_bounder);
        match &self.image {
            None => dim,
            Some((image, _)) => dim.delta(Self::IMAGE_PADDING + image.width() as f64, 0.0),
        }
    }
}

/// `GraphicStrings.sansSerif12`.
fn sans_serif12(color: HColor) -> FontConfiguration {
    FontConfiguration::new(UFont::new("SansSerif", UFontFace::NORMAL, 12), color, 8)
}

impl TextBlock for GraphicStrings {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let margins = 2.0 * Self::MARGIN;
        self.calculate_dimension_internal(string_bounder)
            .delta(margins, margins)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.translated(Self::MARGIN, Self::MARGIN);
        let size = self.calculate_dimension_internal(ug.string_bounder());
        self.text.draw_u(&ug.with_color(self.color.clone()));
        let Some((image, position)) = &self.image else {
            return;
        };
        let (width, height) = (image.width() as f64, image.height() as f64);
        let (x, y) = match position {
            GraphicPosition::BackgroundCornerBottomRight => {
                (size.width - width, size.height - height)
            }
            GraphicPosition::BackgroundCornerTopRight => (size.width - width - 1.0, 1.0),
        };
        ug.translated(x, y)
            .draw(&UShape::Image(UImage::new(image.clone())));
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.background.clone())
    }
}

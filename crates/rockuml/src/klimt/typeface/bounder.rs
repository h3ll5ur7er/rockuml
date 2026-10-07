use std::sync::Arc;

use super::FontRegistry;
use crate::klimt::font::{StringBounder, UFont};
use crate::klimt::geom::XDimension2D;

/// Measures text with real fonts, as PlantUML does for SVG and PNG.
pub(crate) struct StringBounderFonts {
    fonts: Arc<FontRegistry>,
}

impl StringBounderFonts {
    pub(crate) fn new(fonts: Arc<FontRegistry>) -> Self {
        Self { fonts }
    }
}

impl StringBounder for StringBounderFonts {
    fn calculate_dimension(&self, font: &UFont, text: &str) -> XDimension2D {
        if text.is_empty() {
            return XDimension2D::default();
        }
        let resolved = self.fonts.font_for(font, text);
        let width: f64 = text
            .chars()
            .map(|c| {
                resolved
                    .advance(c)
                    .unwrap_or_else(|| resolved.missing_glyph_advance(c))
            })
            .sum();
        let (ascent, descent, leading) = resolved.vertical_metrics();
        let size = font.size_2d();
        XDimension2D::new(width * size, (ascent + descent + leading) * size)
    }

    fn descent(&self, font: &UFont, text: &str) -> f64 {
        let (_, descent, _) = self.fonts.font_for(font, text).vertical_metrics();
        descent * font.size_2d()
    }

    fn shared(&self) -> std::rc::Rc<dyn StringBounder> {
        std::rc::Rc::new(Self::new(self.fonts.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::font::UFontFace;

    fn measure(family: &str, face: UFontFace, text: &str) -> XDimension2D {
        StringBounderFonts::new(Arc::new(FontRegistry::default()))
            .calculate_dimension(&UFont::new(family, face, 14), text)
    }

    /// Measured with PlantUML's JDK on Windows, where Java maps its logical fonts to Arial, Times New Roman
    /// and Courier New.
    #[test]
    fn text_measures_as_java_measures_it_on_windows() {
        let sans = measure("SansSerif", UFontFace::NORMAL, "Hello world");
        assert_eq!((sans.width, sans.height), (69.248046875, 17.609375));
        let bold = measure("SansSerif", UFontFace::BOLD, "This is a second line");
        assert_eq!(bold.width, 136.9306640625);
        let serif = measure("Serif", UFontFace::NORMAL, "This is a second line");
        assert_eq!((serif.width, serif.height), (114.310546875, 17.74609375));
        let mono = measure("Monospaced", UFontFace::NORMAL, "Hello world");
        assert_eq!((mono.width, mono.height), (92.4150390625, 18.279296875));
        let arial = measure("Arial", UFontFace::NORMAL, "Hello world");
        assert_eq!((arial.width, arial.height), (69.248046875, 16.0986328125));
    }

    #[test]
    fn neither_kerning_nor_ligatures_apply() {
        assert_eq!(
            measure("SansSerif", UFontFace::NORMAL, "AV").width,
            18.67578125
        );
        assert_eq!(measure("SansSerif", UFontFace::NORMAL, "fi").width, 7.0);
    }

    #[test]
    fn logical_fonts_measure_other_scripts_one_em_wide() {
        assert_eq!(
            measure("SansSerif", UFontFace::NORMAL, "\u{4E2D}\u{6587}").width,
            28.0
        );
    }

    #[test]
    fn fallback_lists_use_the_first_family_that_has_every_glyph() {
        let listed = measure("Liberation Mono, Arial", UFontFace::NORMAL, "Hello world");
        assert_eq!(
            listed.width,
            measure("Courier New", UFontFace::NORMAL, "Hello world").width
        );
        let skipped = measure("Liberation Mono, SansSerif", UFontFace::NORMAL, "\u{4E2D}");
        assert_eq!(skipped.width, 14.0);
    }

    #[test]
    fn empty_text_has_no_size() {
        assert_eq!(
            measure("SansSerif", UFontFace::NORMAL, ""),
            XDimension2D::default()
        );
    }
}

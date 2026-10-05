//! PlantUML's `debug` output format: a plain-text listing of every drawn shape, with text measured by a
//! seeded pseudo-random metric instead of real fonts so that the output is the same everywhere.

use super::font::{StringBounder, UFont};
use super::geom::{UTranslate, XDimension2D};
use super::shape::{UShape, UText};
use super::ugraphic::{UGraphicBackend, UParam};
use crate::color::HColor;
use crate::java::{self, Random};

pub struct StringBounderDebug;

impl StringBounder for StringBounderDebug {
    fn calculate_dimension(&self, font: &UFont, text: &str) -> XDimension2D {
        let factor = 0.8 + 0.5 * Random::new(java::string_seed(text)).next_double();
        let size = font.size_2d();
        let utf16_length = text.encode_utf16().count() as f64;
        XDimension2D::new(size * utf16_length * factor, size)
    }
}

/// What the debug document says about the image before listing its shapes.
pub struct DebugHeader {
    pub dimension: XDimension2D,
    pub scale_factor: f64,
    pub seed: i64,
    pub svg_link_target: Option<String>,
    pub hover_path_color_rgb: Option<String>,
    pub preserve_aspect_ratio: String,
}

#[derive(Default)]
pub struct UGraphicDebug {
    lines: Vec<String>,
}

impl UGraphicDebug {
    /// The document, with lines ending in `\n` on every platform as in PlantUML.
    pub fn into_document(self, header: &DebugHeader) -> String {
        let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| "null".to_owned());
        let header_lines = [
            "DPI: 96".to_owned(),
            format!(
                "dimension: {}",
                point(header.dimension.width, header.dimension.height)
            ),
            format!(
                "scaleFactor: {}",
                java::format_fixed(header.scale_factor, 4)
            ),
            format!("seed: {}", header.seed),
            format!("svgLinkTarget: {}", optional(&header.svg_link_target)),
            format!(
                "hoverPathColorRGB: {}",
                optional(&header.hover_path_color_rgb)
            ),
            format!("preserveAspectRatio: {}", header.preserve_aspect_ratio),
            String::new(),
        ];
        header_lines
            .into_iter()
            .chain(self.lines)
            .flat_map(|line| [line, "\n".to_owned()])
            .collect()
    }

    fn out_text(&mut self, text: &UText, at: UTranslate) {
        self.lines.extend([
            "TEXT:".to_owned(),
            format!("  text: {}", text.text),
            format!("  position: {}", point(at.dx, at.dy)),
            format!("  orientation: {}", text.orientation),
            format!("  font: {}", text.font.to_string_debug()),
            format!("  color: {}", color_to_string(Some(text.font.color()))),
            format!(
                "  extendedColor: {}",
                color_to_string(text.font.extended_color())
            ),
            String::new(),
        ]);
    }
}

impl UGraphicBackend for UGraphicDebug {
    fn draw(&mut self, shape: &UShape, at: UTranslate, _param: &UParam) {
        match shape {
            UShape::Text(text) => self.out_text(text, at),
        }
    }
}

fn point(x: f64, y: f64) -> String {
    format!(
        "[ {} ; {} ]",
        java::format_fixed(x, 4),
        java::format_fixed(y, 4)
    )
}

fn color_to_string(color: Option<&HColor>) -> String {
    match color {
        None => "NULL_COLOR".to_owned(),
        Some(color) if color.is_transparent() => "NULL_COLOR".to_owned(),
        Some(HColor::Simple(color)) => format!("{:x}", color.argb()),
        Some(other) => unimplemented!("debug output of {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Widths observed in PlantUML's debug output for Serif 14.
    #[test]
    fn text_is_measured_like_plantuml() {
        let bounder = StringBounderDebug;
        let font = UFont::serif(14);
        let hello = bounder.calculate_dimension(&font, "Hello world");
        assert_eq!(java::format_fixed(hello.height, 4), "14.0000");
        assert_eq!(bounder.descent(&font, "x"), 14.0 / 4.5);
        let tab = bounder.calculate_dimension(&font, "        ");
        assert_eq!(java::format_fixed(tab.width, 4), "133.5337");
    }
}

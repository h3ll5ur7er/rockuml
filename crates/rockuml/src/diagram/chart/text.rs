//! The texts charts draw: creole blocks, numbers as PlantUML prints them, and the labels of values.

use crate::color::HColor;
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::java::{format_fixed, format_scientific};
use crate::klimt::font::{FontConfiguration, UFont, UFontFace};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;

/// `Display.getWithNewlines(pragma, text).create(font, alignment, skinParam)`.
pub(super) fn text_block(
    skin: &SkinParam,
    text: &str,
    font: &FontConfiguration,
    alignment: HorizontalAlignment,
) -> SheetBlock2 {
    Display::with_newlines(text).create0(font, alignment, skin, 0.0, CreoleMode::Full)
}

/// `FontConfiguration.create(font, color, color, null)`: links in the text's own colour.
pub(super) fn font_configuration(font: UFont, color: &HColor) -> FontConfiguration {
    FontConfiguration::new(font, color.clone(), 8)
        .with_hyperlink_style(color.clone(), crate::klimt::ugraphic::UStroke::SIMPLE)
}

/// Tiny values in scientific notation, whole ones without decimals, others with `decimals` of them
/// (`formatValue` with two, `formatAxisValue` with one).
pub(super) fn format_value(value: f64, decimals: usize) -> String {
    if value.abs() < 0.01 && value != 0.0 {
        return format_scientific(value, 2);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let whole = value as i64;
    #[allow(clippy::cast_precision_loss)]
    if value == whole as f64 {
        return whole.to_string();
    }
    format_fixed(value, decimals)
}

/// A value's label, bold and centred over (`x`, `y`) (`drawLabel` of the bar, area and scatter renderers).
pub(super) fn draw_value_label(ug: &UGraphic, skin: &SkinParam, value: f64, x: f64, y: f64) {
    let block = value_label(skin, value, HorizontalAlignment::Center);
    let dimension = block.calculate_dimension(ug.string_bounder());
    block.draw_u(&ug.translated(x - dimension.width / 2.0, y - dimension.height - 2.0));
}

pub(super) fn value_label(
    skin: &SkinParam,
    value: f64,
    alignment: HorizontalAlignment,
) -> SheetBlock2 {
    let font = UFont::new("SansSerif", UFontFace::BOLD, 10);
    text_block(
        skin,
        &format_value(value, 2),
        &font_configuration(font, &HColor::BLACK),
        alignment,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_print_whole_tiny_or_with_decimals() {
        assert_eq!(format_value(40.0, 2), "40");
        assert_eq!(format_value(-35.0, 2), "-35");
        assert_eq!(format_value(18.25, 2), "18.25");
        assert_eq!(format_value(33.333, 2), "33.33");
        assert_eq!(format_value(0.005, 2), "5.00e-03");
        assert_eq!(format_value(0.0, 2), "0");
        assert_eq!(format_value(0.25, 1), "0.3");
    }
}

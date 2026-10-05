//! Text measured from a fixed table of character widths, the same for every font (PlantUML's
//! `StringBounderFromWidthTable`): what deterministic SVG uses, so that it is identical on every machine.

use std::sync::LazyLock;

use super::font::{StringBounder, UFont};
use super::geom::XDimension2D;
use super::width_table_data::SANS_SERIF;

/// The size the table's widths are given for.
const REFERENCE_SIZE: f64 = 16.0;
const WIDTH_BEYOND_BMP: f64 = 16.0;
const WIDTH_BEYOND_TABLE: f64 = 13.0;

pub struct StringBounderFromWidthTable;

impl StringBounder for StringBounderFromWidthTable {
    fn calculate_dimension(&self, font: &UFont, text: &str) -> XDimension2D {
        let size = font.size_2d();
        let width: f64 = text.chars().map(char_width).sum();
        XDimension2D::new(width * size / REFERENCE_SIZE, size)
    }
}

fn char_width(c: char) -> f64 {
    static BLOCKS: LazyLock<Vec<UnicodeBlock>> = LazyLock::new(|| {
        SANS_SERIF
            .iter()
            .map(|data| UnicodeBlock::new(data))
            .collect()
    });
    let code_point = u32::from(c);
    if code_point >= 0xFFFF {
        return WIDTH_BEYOND_BMP;
    }
    BLOCKS
        .get((code_point >> 8) as usize)
        .map_or(WIDTH_BEYOND_TABLE, |block| block.width(code_point as u8))
}

enum UnicodeBlock {
    Uniform(u8),
    PerCharacter(Vec<u8>),
}

impl UnicodeBlock {
    fn new(data: &[u8]) -> Self {
        match data {
            [width] => Self::Uniform(*width),
            _ if data.len() < 256 => Self::PerCharacter(
                data.chunks(2)
                    .flat_map(|run| std::iter::repeat_n(run[1], usize::from(run[0])))
                    .collect(),
            ),
            _ => Self::PerCharacter(data.to_vec()),
        }
    }

    fn width(&self, low_byte: u8) -> f64 {
        let tenths = match self {
            Self::Uniform(width) => *width,
            Self::PerCharacter(widths) => widths[usize::from(low_byte)],
        };
        f64::from(tenths) / 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::font::UFontFace;

    fn width(text: &str, size: i32) -> f64 {
        StringBounderFromWidthTable
            .calculate_dimension(&UFont::new("Serif", UFontFace::NORMAL, size), text)
            .width
    }

    #[test]
    fn widths_match_plantuml() {
        assert!((width("Hello world", 14) - 65.538).abs() < 5e-4);
        assert!((width("This is a second line", 14) - 111.475).abs() < 5e-4);
    }

    #[test]
    fn run_length_encoded_and_uniform_blocks_are_read() {
        assert!(width("\u{4E00}", 16) > 0.0);
        assert!((width("\u{1F600}", 16) - WIDTH_BEYOND_BMP).abs() < 1e-9);
    }

    #[test]
    fn height_is_the_font_size() {
        let dimension = StringBounderFromWidthTable
            .calculate_dimension(&UFont::new("Serif", UFontFace::NORMAL, 12), "x");
        assert!((dimension.height - 12.0).abs() < 1e-9);
    }
}

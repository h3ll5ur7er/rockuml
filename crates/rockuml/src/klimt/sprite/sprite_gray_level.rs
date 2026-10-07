use super::sprite_monochrome::SpriteMonochrome;
use crate::url_code;

/// How many gray levels a sprite's text encodes, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpriteGrayLevel {
    Gray16,
    Gray8,
    Gray4,
}

impl SpriteGrayLevel {
    pub(crate) fn get(levels: u32) -> Option<Self> {
        match levels {
            4 => Some(Self::Gray4),
            8 => Some(Self::Gray8),
            16 => Some(Self::Gray16),
            _ => None,
        }
    }

    fn nb_color(self) -> usize {
        match self {
            Self::Gray16 => 16,
            Self::Gray8 => 8,
            Self::Gray4 => 4,
        }
    }

    /// One character per pixel, or per two (8 levels) or three (4 levels) pixels stacked vertically. Sixteen
    /// levels take their size from the text.
    pub(crate) fn build_sprite(
        self,
        width: usize,
        height: usize,
        strings: &[String],
    ) -> SpriteMonochrome {
        match self {
            Self::Gray16 => build_sprite16(strings),
            Self::Gray8 => build_stacked(width, height, strings, 8, 2),
            Self::Gray4 => build_stacked(width, height, strings, 4, 3),
        }
    }

    /// One byte per pixel, row by row, deflated and written in PlantUML's URL alphabet. PlantUML fails on
    /// too few pixels and on levels beyond the sprite's.
    pub(crate) fn build_sprite_z(
        self,
        width: usize,
        height: usize,
        compressed: &str,
    ) -> Option<SpriteMonochrome> {
        let compressed = url_code::decode_6bit(compressed).ok()?;
        let pixels = url_code::inflate_prefix(&compressed, width * height).ok()?;
        let mut result = SpriteMonochrome::new(width, height, self.nb_color());
        let mut levels = pixels.iter().map(|&level| usize::from(level));
        for line in 0..height {
            for col in 0..width {
                let level = levels.next().filter(|&level| level < self.nb_color())?;
                result.set_gray(col, line, level);
            }
        }
        Some(result)
    }
}

fn build_sprite16(strings: &[String]) -> SpriteMonochrome {
    let width = strings.first().map_or(0, |first| first.chars().count());
    let mut result = SpriteMonochrome::new(width, strings.len(), 16);
    for (line, text) in strings.iter().enumerate() {
        for (col, c) in text.chars().enumerate() {
            if let Some(level) = c.to_digit(16) {
                result.set_gray(col, line, level as usize);
            }
        }
    }
    result
}

/// Each character's 6 bits give `stacked` pixels of `levels` levels, the top one in the high bits.
fn build_stacked(
    width: usize,
    height: usize,
    strings: &[String],
    levels: usize,
    stacked: usize,
) -> SpriteMonochrome {
    let mut result = SpriteMonochrome::new(width, height, levels);
    for (line, text) in strings.iter().enumerate() {
        for (col, c) in text.chars().enumerate().take(width) {
            let value = usize::from(url_code::sextet(c));
            for row in 0..stacked {
                let shift = levels.pow((stacked - 1 - row) as u32);
                result.set_gray(col, line * stacked + row, value / shift % levels);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|&text| text.to_owned()).collect()
    }

    fn first_column(sprite: &SpriteMonochrome) -> Vec<usize> {
        (0..sprite.height())
            .map(|y| sprite.get_gray(0, y))
            .collect()
    }

    #[test]
    fn sixteen_levels_take_their_size_from_the_text() {
        let sprite = SpriteGrayLevel::Gray16.build_sprite(9, 9, &lines(&["0F0", "a"]));
        assert_eq!((sprite.width(), sprite.height()), (3, 2));
        assert_eq!(first_column(&sprite), [0, 10]);
    }

    #[test]
    fn eight_levels_stack_two_pixels_per_character() {
        // 'z' is 61.
        let sprite = SpriteGrayLevel::Gray8.build_sprite(1, 2, &lines(&["z"]));
        assert_eq!(first_column(&sprite), [7, 5]);
    }

    #[test]
    fn four_levels_stack_three_pixels_per_character() {
        let sprite = SpriteGrayLevel::Gray4.build_sprite(1, 3, &lines(&["z"]));
        assert_eq!(first_column(&sprite), [3, 3, 1]);
    }

    #[test]
    fn compressed_sprites_inflate_one_byte_per_pixel() {
        // Levels 0, 1, 2 and 3, deflated and encoded.
        let sprite = SpriteGrayLevel::Gray4
            .build_sprite_z(2, 2, "Os1aOWO0")
            .unwrap();
        assert_eq!(first_column(&sprite), [0, 2]);
        assert_eq!(sprite.get_gray(1, 1), 3);
    }

    #[test]
    fn compressed_sprites_need_a_valid_level_for_every_pixel() {
        let compressed = |levels: &[u8]| url_code::encode_6bit(&crate::deflate::deflate(levels));
        let build =
            |levels: &[u8]| SpriteGrayLevel::Gray4.build_sprite_z(2, 1, &compressed(levels));
        assert!(build(&[0, 3]).is_some());
        assert!(build(&[0]).is_none(), "too few pixels");
        assert!(build(&[0, 4]).is_none(), "a level beyond the sprite's");
    }
}

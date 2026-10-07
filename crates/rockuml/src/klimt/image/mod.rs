//! Raster images as PlantUML holds them (`PortableImage`): Java ARGB pixels, not premultiplied.

mod bilinear;

use std::io::Cursor;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PortableImage {
    width: usize,
    height: usize,
    /// Row by row, as Java's `getRGB` returns them: alpha in the top byte.
    pixels: Vec<u32>,
}

impl PortableImage {
    /// Transparent, like a new `BufferedImage`.
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
        }
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn get_rgb(&self, x: usize, y: usize) -> u32 {
        self.pixels[y * self.width + x]
    }

    pub(crate) fn set_rgb(&mut self, x: usize, y: usize, argb: u32) {
        self.pixels[y * self.width + x] = argb;
    }

    /// The pixels of a PNG file, or `None` if it is not one.
    pub(crate) fn from_png(data: &[u8]) -> Option<Self> {
        let mut decoder = png::Decoder::new(Cursor::new(data));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().ok()?;
        let mut buffer = vec![0; reader.output_buffer_size()?];
        let frame = reader.next_frame(&mut buffer).ok()?;
        let samples = frame.color_type.samples();
        let pixels = buffer[..frame.buffer_size()]
            .chunks_exact(samples)
            .map(|sample| {
                let [red, green, blue, alpha] = match *sample {
                    [gray] => [gray, gray, gray, 255],
                    [gray, alpha] => [gray, gray, gray, alpha],
                    [red, green, blue] => [red, green, blue, 255],
                    [red, green, blue, alpha] => [red, green, blue, alpha],
                    _ => unreachable!("normalised PNGs have one to four 8-bit samples"),
                };
                u32::from_be_bytes([alpha, red, green, blue])
            })
            .collect();
        Some(Self {
            width: frame.width as usize,
            height: frame.height as usize,
            pixels,
        })
    }

    /// The image as a PNG file.
    pub(crate) fn to_png(&self) -> Vec<u8> {
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let rgba: Vec<u8> = self
            .pixels
            .iter()
            .flat_map(|argb| {
                let [alpha, red, green, blue] = argb.to_be_bytes();
                [red, green, blue, alpha]
            })
            .collect();
        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(&rgba))
            .expect("encoding to memory succeeds");
        png
    }

    /// `PortableImageAwt.scale` with bilinear interpolation: Java 2D's `AffineTransformOp` into an image of
    /// the rounded scaled size.
    #[must_use]
    pub(crate) fn scale(&self, factor: f64) -> Self {
        if factor == 1.0 {
            return self.clone();
        }
        let width = (self.width as f64 * factor).round() as usize;
        let height = (self.height as f64 * factor).round() as usize;
        bilinear::scale(self, factor, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_files_round_trip() {
        let mut image = PortableImage::new(2, 1);
        image.set_rgb(1, 0, 0x80ff_0010);
        let decoded = PortableImage::from_png(&image.to_png()).unwrap();
        assert_eq!(decoded, image);
    }

    #[test]
    fn scaled_size_is_rounded() {
        let image = PortableImage::new(16, 10);
        let scaled = image.scale(14.0 / 13.0);
        assert_eq!((scaled.width(), scaled.height()), (17, 11));
    }
}

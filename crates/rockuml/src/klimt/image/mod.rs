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

    /// The pixels of a PNG, GIF or JPEG file, told apart by their content as Java's readers do (`SImageIO.read`).
    pub(crate) fn read(data: &[u8]) -> Option<Self> {
        if data.starts_with(b"\x89PNG") {
            Self::from_png(data)
        } else if data.starts_with(b"GIF8") {
            Self::from_gif(data)
        } else if data.starts_with(b"\xFF\xD8") {
            Self::from_jpeg(data)
        } else {
            None
        }
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
                argb(red, green, blue, alpha)
            })
            .collect();
        Some(Self {
            width: frame.width as usize,
            height: frame.height as usize,
            pixels,
        })
    }

    /// The first frame, placed on the logical screen, which transparent pixels leave see-through.
    fn from_gif(data: &[u8]) -> Option<Self> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(data).ok()?;
        let mut image = Self::new(usize::from(decoder.width()), usize::from(decoder.height()));
        let frame = decoder.read_next_frame().ok()??;
        let (left, top) = (usize::from(frame.left), usize::from(frame.top));
        for (index, &[red, green, blue, alpha]) in
            frame.buffer.as_chunks::<4>().0.iter().enumerate()
        {
            let x = left + index % usize::from(frame.width);
            let y = top + index / usize::from(frame.width);
            if x < image.width && y < image.height {
                image.set_rgb(x, y, argb(red, green, blue, alpha));
            }
        }
        Some(image)
    }

    fn from_jpeg(data: &[u8]) -> Option<Self> {
        use zune_jpeg::zune_core::bytestream::ZCursor;
        use zune_jpeg::zune_core::colorspace::ColorSpace;
        use zune_jpeg::zune_core::options::DecoderOptions;

        let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
        let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(data), options);
        let rgb = decoder.decode().ok()?;
        let info = decoder.info()?;
        Some(Self {
            width: usize::from(info.width),
            height: usize::from(info.height),
            pixels: rgb
                .as_chunks::<3>()
                .0
                .iter()
                .map(|&[red, green, blue]| argb(red, green, blue, 0xFF))
                .collect(),
        })
    }

    /// The image drawn over a new transparent one, as PlantUML reads image files and URLs
    /// (`SecurityUtils.readRasterImage`): Java 2D's source-over drops the colour of see-through pixels and
    /// rounds translucent ones through premultiplied alpha.
    #[must_use]
    pub(crate) fn drawn_on_transparent(&self) -> Self {
        let pixels = self
            .pixels
            .iter()
            .map(|&pixel| {
                let [alpha, red, green, blue] = pixel.to_be_bytes();
                match alpha {
                    0 => 0,
                    0xFF => pixel,
                    _ => {
                        let round_trip = |channel| div8(alpha, mul8(alpha, channel));
                        argb(round_trip(red), round_trip(green), round_trip(blue), alpha)
                    }
                }
            })
            .collect();
        Self { pixels, ..*self }
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

/// `a × b / 255`, rounded as Java 2D's `mul8table`.
fn mul8(a: u8, b: u8) -> u8 {
    let increment = u64::from(a) * 0x01_0101;
    ((u64::from(b) * increment + (1 << 23)) >> 24) as u8
}

/// `b × 255 / a`, rounded and capped at 255 as Java 2D's `div8table`.
fn div8(a: u8, b: u8) -> u8 {
    if b >= a {
        return 0xFF;
    }
    let increment = ((0xFF_u64 << 24) + u64::from(a) / 2) / u64::from(a);
    ((u64::from(b) * increment + (1 << 23)) >> 24) as u8
}

fn argb(red: u8, green: u8, blue: u8, alpha: u8) -> u32 {
    u32::from_be_bytes([alpha, red, green, blue])
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
    fn drawing_on_transparent_rounds_through_premultiplied_alpha() {
        let mut image = PortableImage::new(3, 1);
        image.set_rgb(0, 0, 0x00FF_FFFF);
        image.set_rgb(1, 0, 0x034A_07C8);
        image.set_rgb(2, 0, 0xFF12_3456);
        let drawn = image.drawn_on_transparent();
        assert_eq!(
            [
                drawn.get_rgb(0, 0),
                drawn.get_rgb(1, 0),
                drawn.get_rgb(2, 0)
            ],
            [0, 0x0355_00AA, 0xFF12_3456]
        );
    }

    #[test]
    fn scaled_size_is_rounded() {
        let image = PortableImage::new(16, 10);
        let scaled = image.scale(14.0 / 13.0);
        assert_eq!((scaled.width(), scaled.height()), (17, 11));
    }
}

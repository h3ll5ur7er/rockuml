//! PNG images: the SVG document, rasterised with the fonts its text was measured with.

use std::io::Write;
use std::sync::Arc;

use flate2::Crc;
use flate2::write::ZlibEncoder;
use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::fontdb::{self, Database};
use resvg::usvg::{FontFamily, FontResolver, ImageHrefResolver, Options, Tree};

use super::typeface::FontRegistry;
use crate::color::HColor;

/// `size` is the image's size in pixels; `metadata` the diagram source PlantUML embeds in its PNGs.
pub(crate) fn rasterize(
    svg: &str,
    (width, height): (u32, u32),
    background: &HColor,
    fonts: &Arc<FontRegistry>,
    metadata: &str,
) -> Vec<u8> {
    let mut pixmap = Pixmap::new(width.max(1), height.max(1))
        .expect("images are at most PLANTUML_LIMIT_SIZE wide");
    if let HColor::Simple(color) = background {
        pixmap.fill(Color::from_rgba8(
            color.red,
            color.green,
            color.blue,
            color.alpha,
        ));
    }
    let options = Options {
        fontdb: Arc::new(font_database(fonts)),
        font_resolver: font_resolver(fonts.clone()),
        image_href_resolver: embedded_images_only(),
        ..Options::default()
    };
    let tree = Tree::from_str(svg, &options).expect("rockuml writes valid SVG");
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    let png = pixmap.encode_png().expect("encoding to memory succeeds");
    with_text_chunk(&png, "plantuml", metadata)
}

/// The engine reads no files: images come embedded in the document or not at all.
fn embedded_images_only() -> ImageHrefResolver<'static> {
    ImageHrefResolver {
        resolve_data: ImageHrefResolver::default_data_resolver(),
        resolve_string: Box::new(|_, _| None),
    }
}

fn font_database(fonts: &FontRegistry) -> Database {
    let mut database = Database::new();
    for data in fonts.files() {
        database.load_font_source(fontdb::Source::Binary(data));
    }
    database
}

/// Draws text with the font Java would choose for each family name, so that it fits the measured width.
fn font_resolver(fonts: Arc<FontRegistry>) -> FontResolver<'static> {
    FontResolver {
        select_font: Box::new(move |font, database| {
            let drawn: Vec<String> = font
                .families()
                .iter()
                .map(|family| match family {
                    FontFamily::Serif => fonts.drawn_family("Serif"),
                    FontFamily::SansSerif | FontFamily::Cursive | FontFamily::Fantasy => {
                        fonts.drawn_family("SansSerif")
                    }
                    FontFamily::Monospace => fonts.drawn_family("Monospaced"),
                    FontFamily::Named(name) => fonts.drawn_family(name),
                })
                .collect();
            let families: Vec<fontdb::Family> = drawn
                .iter()
                .map(|name| fontdb::Family::Name(name))
                .collect();
            database.query(&fontdb::Query {
                families: &families,
                weight: fontdb::Weight(font.weight()),
                style: match font.style() {
                    resvg::usvg::FontStyle::Normal => fontdb::Style::Normal,
                    resvg::usvg::FontStyle::Italic => fontdb::Style::Italic,
                    resvg::usvg::FontStyle::Oblique => fontdb::Style::Oblique,
                },
                stretch: fontdb::Stretch::Normal,
            })
        }),
        select_fallback: FontResolver::default_fallback_selector(),
    }
}

/// The PNG with a compressed `iTXt` chunk before its end, as PlantUML stores the diagram source.
fn with_text_chunk(png: &[u8], keyword: &str, text: &str) -> Vec<u8> {
    /// Length, type and checksum.
    const CHUNK_OVERHEAD: usize = 12;
    const IEND_CHUNK_LENGTH: usize = CHUNK_OVERHEAD;
    let mut compressed = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    compressed
        .write_all(text.as_bytes())
        .expect("compressing to memory succeeds");
    let mut data = Vec::from(keyword.as_bytes());
    // Null separator, compressed, zlib, then empty language tag and translated keyword.
    data.extend([0, 1, 0, 0, 0]);
    data.extend(compressed.finish().expect("compressing to memory succeeds"));

    let mut chunk = Vec::with_capacity(data.len() + CHUNK_OVERHEAD);
    chunk.extend(
        u32::try_from(data.len())
            .expect("metadata fits a chunk")
            .to_be_bytes(),
    );
    chunk.extend(b"iTXt");
    chunk.extend(&data);
    let mut crc = Crc::new();
    crc.update(&chunk[4..]);
    chunk.extend(crc.sum().to_be_bytes());

    let end = png.len() - IEND_CHUNK_LENGTH;
    [&png[..end], &chunk, &png[end..]].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rasterized(svg: &str) -> Pixmap {
        let png = rasterize(
            svg,
            (20, 10),
            &HColor::WHITE,
            &Arc::new(FontRegistry::default()),
            "src",
        );
        Pixmap::decode_png(&png).unwrap()
    }

    #[test]
    fn the_image_has_the_given_size_and_background() {
        let pixmap =
            rasterized(r#"<svg xmlns="http://www.w3.org/2000/svg" width="5px" height="5px"/>"#);
        assert_eq!((pixmap.width(), pixmap.height()), (20, 10));
        assert_eq!(pixmap.pixel(19, 9).unwrap().red(), 255);
    }

    #[test]
    fn text_is_drawn_with_the_embedded_fonts() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><text x="0" y="9" font-size="12" font-family="Helvetica">Ww</text></svg>"#;
        let pixmap = rasterized(svg);
        assert!(pixmap.pixels().iter().any(|pixel| pixel.red() < 128));
    }

    #[test]
    fn the_source_is_stored_in_a_text_chunk() {
        let png = rasterize(
            r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
            (1, 1),
            &HColor::WHITE,
            &Arc::new(FontRegistry::default()),
            "@startuml",
        );
        let at = png.windows(4).position(|window| window == b"iTXt").unwrap();
        assert_eq!(&png[at + 4..at + 13], b"plantuml\0");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }
}

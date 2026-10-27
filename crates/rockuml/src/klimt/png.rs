//! PNG images: the SVG document, rasterised with the fonts its text was measured with.

use std::io::Write;
use std::sync::Arc;

use flate2::Crc;
use flate2::write::ZlibEncoder;
use resvg::tiny_skia::{Color, IntRect, Paint, Pixmap, PixmapPaint, Rect, Transform};
use resvg::usvg::fontdb::{self, Database};
use resvg::usvg::{FontFamily, FontResolver, ImageHrefResolver, Options, Tree};

use super::typeface::FontRegistry;
use crate::color::XColor;

/// `size` is the image's size in pixels, filled with `background` before the drawing; `metadata` the diagram source
/// PlantUML embeds in its PNGs.
pub(crate) fn rasterize(
    svg: &str,
    size: (u32, u32),
    background: XColor,
    fonts: &Arc<FontRegistry>,
    metadata: Option<&str>,
) -> Vec<u8> {
    encode(&render(svg, size, background, fonts), metadata)
}

/// Like [`rasterize`], cut into the pieces `split` asks for, across then down (PlantUML's `PngSplitter`), each
/// encoded only when it is asked for. `None` where PlantUML throws making the first piece (see
/// [`PageSplit::can_cut`]).
pub(crate) fn rasterize_pieces(
    svg: &str,
    size: (u32, u32),
    background: XColor,
    fonts: &Arc<FontRegistry>,
    metadata: Option<&str>,
    split: PageSplit,
    limit: u32,
) -> Option<impl Iterator<Item = Vec<u8>> + use<>> {
    if !split.can_cut(size, limit) {
        return None;
    }
    let pieces = Pieces {
        image: render(svg, size, background, fonts),
        split,
        next: 0,
    };
    let metadata = metadata.map(str::to_owned);
    Some(pieces.map(move |piece| encode(&piece, metadata.as_deref())))
}

fn render(
    svg: &str,
    (width, height): (u32, u32),
    background: XColor,
    fonts: &Arc<FontRegistry>,
) -> Pixmap {
    let mut pixmap = Pixmap::new(width.max(1), height.max(1))
        .expect("images are at most PLANTUML_LIMIT_SIZE wide");
    pixmap.fill(color(background));
    let options = Options {
        fontdb: Arc::new(font_database(fonts)),
        font_resolver: font_resolver(fonts.clone()),
        image_href_resolver: embedded_images_only(),
        ..Options::default()
    };
    let tree = Tree::from_str(svg, &options).expect("rockuml writes valid SVG");
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    pixmap
}

fn encode(pixmap: &Pixmap, metadata: Option<&str>) -> Vec<u8> {
    let png = pixmap.encode_png().expect("encoding to memory succeeds");
    match metadata {
        Some(metadata) => with_text_chunk(&png, "plantuml", metadata),
        None => png,
    }
}

fn color(color: XColor) -> Color {
    Color::from_rgba8(color.red, color.green, color.blue, color.alpha)
}

/// `page 2x3`: an image cut into 2 pieces across and 3 down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PageSplit {
    pub(crate) horizontal: u32,
    pub(crate) vertical: u32,
    pub(crate) param: SplitParam,
}

impl PageSplit {
    /// Whether a `width` × `height` image can be cut, its pieces being all of one size. PlantUML throws on a
    /// piece less than a pixel long (`getSubimage`) or a frame Java's `int`s make empty or too large to
    /// allocate; rockuml refuses frames beyond `limit` too, where Java runs out of memory sooner or later.
    fn can_cut(&self, (width, height): (u32, u32), limit: u32) -> bool {
        let (_, piece_width) = segment(width, self.horizontal, 0);
        let (_, piece_height) = segment(height, self.vertical, 0);
        piece_width > 0
            && piece_height > 0
            && self
                .param
                .framed_size(piece_width, piece_height, limit)
                .is_some()
    }
}

/// The pieces of an image, cut and framed when they are asked for, so that only one is held at a time.
struct Pieces {
    image: Pixmap,
    split: PageSplit,
    /// Across then down: piece `i` across and `j` down is number `i * vertical + j`.
    next: u64,
}

impl Iterator for Pieces {
    type Item = Pixmap;

    fn next(&mut self) -> Option<Pixmap> {
        let PageSplit {
            horizontal,
            vertical,
            param,
        } = self.split;
        if self.next == u64::from(horizontal) * u64::from(vertical) {
            return None;
        }
        let i = u32::try_from(self.next / u64::from(vertical)).expect("below horizontal");
        let j = u32::try_from(self.next % u64::from(vertical)).expect("below vertical");
        self.next += 1;
        let (x, width) = segment(self.image.width(), horizontal, i);
        let (y, height) = segment(self.image.height(), vertical, j);
        let piece = IntRect::from_xywh(x as i32, y as i32, width, height)
            .and_then(|rect| self.image.clone_rect(rect))
            .expect("can_cut checked that pieces lie inside the image");
        Some(param.framed(piece))
    }
}

/// Where piece `index` of `count` starts along `total` pixels, and how long it is (PlantUML's `PngSegment`).
fn segment(total: u32, count: u32, index: u32) -> (u32, u32) {
    let length = f64::from(total) / f64::from(count);
    ((length * f64::from(index)) as u32, length as u32)
}

/// How the pieces of a cut image are framed: a margin, painted or transparent, and a border inside it
/// (PlantUML's `SplitParam`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SplitParam {
    border: Option<XColor>,
    external: Option<XColor>,
    margin: i32,
}

impl SplitParam {
    pub(crate) fn new(border: Option<XColor>, external: Option<XColor>, margin: i32) -> Self {
        // The border is drawn in the margin, which must be there for it.
        let margin = if border.is_some() && margin == 0 {
            1
        } else {
            margin
        };
        Self {
            border,
            external,
            margin,
        }
    }

    fn is_set(&self) -> bool {
        self.margin > 0
    }

    /// The size of a framed `width` × `height` piece, which Java computes in `int`s that may wrap; `None`
    /// where PlantUML cannot make that image, or it exceeds `limit`.
    fn framed_size(&self, width: u32, height: u32, limit: u32) -> Option<(u32, u32)> {
        if !self.is_set() {
            return Some((width, height));
        }
        let framed = |length: u32| {
            let framed = (length as i32).wrapping_add(self.margin.wrapping_mul(2));
            u32::try_from(framed)
                .ok()
                .filter(|&framed| framed > 0 && framed <= limit)
        };
        Some((framed(width)?, framed(height)?))
    }

    fn framed(&self, piece: Pixmap) -> Pixmap {
        if !self.is_set() {
            return piece;
        }
        let margin = self.margin;
        let (width, height) = (piece.width(), piece.height());
        let (framed_width, framed_height) = self
            .framed_size(width, height, u32::MAX)
            .expect("can_cut checked the frame");
        let mut framed = Pixmap::new(framed_width, framed_height).expect("the frame has pixels");
        if let Some(external) = self.external {
            framed.fill(color(external));
        }
        // Only a margin that wrapped the frame's size round is beyond it; Java then draws nothing that shows,
        // and tiny-skia cannot place anything that far.
        if i64::from(margin) > i64::from(framed_width.max(framed_height)) {
            return framed;
        }
        framed.draw_pixmap(
            margin,
            margin,
            piece.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        if let Some(border) = self.border {
            let mut paint = Paint::default();
            paint.set_color(color(border));
            paint.anti_alias = false;
            // Java's drawRect outlines the pixels around the piece.
            let (left, top) = ((margin - 1) as f32, (margin - 1) as f32);
            let (outer_width, outer_height) = ((width + 2) as f32, (height + 2) as f32);
            let lines = [
                Rect::from_xywh(left, top, outer_width, 1.0),
                Rect::from_xywh(left, top + outer_height - 1.0, outer_width, 1.0),
                Rect::from_xywh(left, top, 1.0, outer_height),
                Rect::from_xywh(left + outer_width - 1.0, top, 1.0, outer_height),
            ];
            for line in lines.into_iter().flatten() {
                framed.fill_rect(line, &paint, Transform::identity(), None);
            }
        }
        framed
    }
}

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
            XColor::rgb(255, 255, 255),
            &Arc::new(FontRegistry::default()),
            Some("src"),
        );
        Pixmap::decode_png(&png).unwrap()
    }

    #[test]
    fn pieces_split_the_image_evenly_and_drop_the_rest() {
        assert_eq!(segment(255, 2, 0), (0, 127));
        assert_eq!(segment(255, 2, 1), (127, 127));
    }

    fn cut(size: (u32, u32), split: PageSplit, limit: u32) -> Option<Vec<Pixmap>> {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#;
        let fonts = Arc::new(FontRegistry::default());
        let white = XColor::rgb(255, 255, 255);
        let pieces = rasterize_pieces(svg, size, white, &fonts, None, split, limit)?;
        Some(
            pieces
                .map(|png| Pixmap::decode_png(&png).unwrap())
                .collect(),
        )
    }

    fn split(horizontal: u32, vertical: u32, param: SplitParam) -> PageSplit {
        PageSplit {
            horizontal,
            vertical,
            param,
        }
    }

    #[test]
    fn pieces_go_across_then_down() {
        let param = SplitParam::new(None, Some(XColor::rgb(255, 0, 0)), 2);
        let pieces = cut((20, 10), split(2, 3, param), 4096).unwrap();
        assert_eq!(pieces.len(), 6);
        let sizes: Vec<_> = pieces
            .iter()
            .map(|piece| (piece.width(), piece.height()))
            .collect();
        assert_eq!(sizes, [(14, 7); 6]);
        assert_eq!(pieces[0].pixel(0, 0).unwrap().red(), 255);
        assert_eq!(pieces[0].pixel(2, 2).unwrap().green(), 255);
    }

    /// PlantUML's `getSubimage` throws on a piece without width or height.
    #[test]
    fn pieces_less_than_a_pixel_long_cannot_be_cut() {
        let none = SplitParam::default();
        assert!(cut((20, 10), split(21, 1, none), 4096).is_none());
        assert!(cut((20, 10), split(1, 11, none), 4096).is_none());
        assert!(cut((20, 10), split(1_000_000, 1_000_000, none), 4096).is_none());
        assert!(cut((20, 10), split(20, 10, none), 4096).is_some());
    }

    /// Java adds twice the margin in `int`s, which wrap: the largest margin shrinks the piece by 2 pixels and
    /// draws it outside the image.
    #[test]
    fn frames_take_java_s_int_arithmetic() {
        let param = SplitParam::new(None, None, i32::MAX);
        let pieces = cut((20, 10), split(2, 1, param), 4096).unwrap();
        let sizes: Vec<_> = pieces
            .iter()
            .map(|piece| (piece.width(), piece.height()))
            .collect();
        assert_eq!(sizes, [(8, 8); 2]);
        assert!(pieces[0].pixels().iter().all(|pixel| pixel.alpha() == 0));
        let param = SplitParam::new(None, None, i32::MAX / 2 + 1);
        assert!(cut((20, 10), split(2, 1, param), 4096).is_none());
    }

    #[test]
    fn frames_beyond_the_image_size_limit_are_refused() {
        let param = SplitParam::new(None, Some(XColor::rgb(255, 0, 0)), 30_000);
        assert!(cut((20, 10), split(2, 1, param), 4096).is_none());
        let param = SplitParam::new(None, None, 3);
        assert!(cut((20, 10), split(2, 1, param), 16).is_some());
        assert!(cut((20, 10), split(2, 1, param), 15).is_none());
    }

    #[test]
    fn a_border_gets_a_margin_to_be_drawn_in() {
        let piece = Pixmap::new(3, 2).unwrap();
        let framed = SplitParam::new(Some(XColor::rgb(255, 0, 0)), None, 0).framed(piece);
        assert_eq!((framed.width(), framed.height()), (5, 4));
        assert_eq!(framed.pixel(0, 0).unwrap().red(), 255);
        assert_eq!(framed.pixel(2, 2).unwrap().alpha(), 0);
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
            XColor::rgb(255, 255, 255),
            &Arc::new(FontRegistry::default()),
            Some("@startuml"),
        );
        let at = png.windows(4).position(|window| window == b"iTXt").unwrap();
        assert_eq!(&png[at + 4..at + 13], b"plantuml\0");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }
}

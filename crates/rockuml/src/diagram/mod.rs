//! Diagrams: recognising a block's diagram type, building the diagram, and exporting it.

mod activity;
pub(crate) mod activity3;
mod board;
mod bpm;
mod builder;
mod cards;
mod chart;
mod chen;
mod chrome;
mod class;
mod common_commands;
mod creole;
pub(crate) mod cuca;
mod cuca_commands;
mod definition;
mod description;
mod diagram_type;
mod donors;
mod dot;
mod ebnf;
mod emoji_list;
mod error;
mod files;
mod flow;
mod gantt;
mod git;
mod hcl;
mod help;
mod json;
mod mindmap;
mod nwdiag;
mod openiconic;
mod packetdiag;
mod regexdiagram;
mod salt;
mod scale;
mod sequence;
mod source;
mod sprite_list;
mod state;
#[cfg(test)]
mod test_support;
mod timing;
mod titled;
mod ug_diagram;
mod wbs;
mod wire;
mod yaml;

pub(crate) use source::{BASE64_TAG_REPLACEMENT, BASE64_TAG_START};
pub(crate) use titled::entity_image_legend;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::color::{ColorMapper, HColor};
use crate::host::{Host, IsolatedHost};
use crate::klimt::TextBlock;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::png::{self, PageSplit};
use crate::klimt::svg::{SvgOption, UGraphicSvg};
use crate::klimt::typeface::{FontRegistry, StringBounderFonts};
use crate::klimt::ugraphic::{UGraphic, UGraphicBackend};
use crate::klimt::width_table::StringBounderFromWidthTable;
use crate::metadata::Metadata;
use crate::preproc::PreprocessedBlock;
use crate::text::StringLocated;
use creole::CreoleDiagram;
use diagram_type::DiagramType;
use error::ErrorDiagram;
use scale::Scale;
pub use source::UmlSource;

/// Names a part of PlantUML that rockuml does not have yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotYetPorted(pub &'static str);

impl std::fmt::Display for NotYetPorted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not ported yet", self.0)
    }
}

pub trait Diagram {
    /// The source the diagram was built from, as it was prepared for building.
    fn source(&self) -> &UmlSource;

    /// Everything the diagram draws on one of its pages, laid out for `string_bounder`.
    fn text_block(
        &self,
        page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted>;

    /// Like [`Diagram::text_block`], when `fonts` are the fonts rockuml draws with, which the font list shows.
    fn text_block_with_fonts(
        &self,
        page: usize,
        string_bounder: &Rc<dyn StringBounder>,
        _fonts: &FontRegistry,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        self.text_block(page, string_bounder)
    }

    /// How many images the diagram makes: `newpage` breaks diagrams into pages.
    fn page_count(&self) -> usize {
        1
    }

    /// How a page is placed on its image, and the image-wide options.
    fn export_settings(&self, page: usize) -> ExportSettings;

    /// Why the diagram is an error image instead of what its source describes.
    fn error(&self) -> Option<DiagramError> {
        None
    }

    /// Whether the diagram is an error image instead of what its source describes.
    fn is_error(&self) -> bool {
        self.error().is_some()
    }

    /// The image of a diagram that asks a program for it instead of drawing it (PlantUML's
    /// `DirectOsDiagram.exportDiagramNow`); `None` for the others, and where the host runs no such program.
    fn export_diagram_now(&self, _format: ImageFormat, _host: &dyn Host) -> Option<DirectImage> {
        None
    }
}

/// What a diagram that asks a program for its image exports.
pub enum DirectImage {
    /// The program's output, written as it is.
    Bytes(Vec<u8>),
    /// Why there is no such output, drawn without the diagram's source: the image reports a crash.
    Drawing(Box<dyn TextBlock>),
}

/// What an error image reports, as PlantUML's `PSystemError` tells it to the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagramError {
    /// The faulty line's position in its file, counted from zero.
    pub line: i32,
    pub message: String,
}

/// The resolution diagrams are drawn for unless `skinparam dpi` says otherwise.
const DEFAULT_DPI: u32 = 96;

/// How a diagram is placed on the image, and the image-wide options the skin decides.
pub struct ExportSettings {
    pub(super) margin: ClockwiseTopRightBottomLeft,
    pub(super) seed: i64,
    /// The skin's background; without a skin, the drawing's own or white.
    pub(super) backcolor: Option<HColor>,
    /// The type name SVG documents announce, for diagrams that have a skin.
    pub(super) diagram_type: Option<&'static str>,
    pub(super) scale: Option<Scale>,
    /// Images are drawn for this resolution; it scales PNGs and SVGs alike.
    pub(super) dpi: u32,
    pub(super) svg_link_target: Option<String>,
    pub(super) preserve_aspect_ratio: String,
    /// How SVG and PNG paint colours.
    pub(super) color_mapper: ColorMapper,
    /// How `page` cuts the PNG files the command line writes.
    pub(super) split: Option<PageSplit>,
}

impl ExportSettings {
    /// For diagrams without a skin: no margin and default options.
    fn without_skin(seed: i64) -> Self {
        Self {
            margin: ClockwiseTopRightBottomLeft::none(),
            seed,
            backcolor: None,
            diagram_type: None,
            scale: None,
            dpi: DEFAULT_DPI,
            svg_link_target: None,
            preserve_aspect_ratio: "none".to_owned(),
            color_mapper: ColorMapper::Identity,
            split: None,
        }
    }
}

type Create = fn(UmlSource, &[StringLocated]) -> Result<Box<dyn Diagram>, NotYetPorted>;

/// The diagram of a block. `host` supplies the files and URLs its images name.
pub fn create(
    block: &PreprocessedBlock,
    host: &dyn Host,
) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let (diagram_type, mut source) = prepare(block);
    if block.failed() {
        // PlantUML's PSystemErrorPreprocessor: its source is what was read up to the error.
        let lines = block.located_lines();
        let read = lines.iter().map(|line| line.text().to_owned()).collect();
        let message = lines
            .last()
            .and_then(StringLocated::preprocessor_error)
            .unwrap_or_default();
        return Ok(Box::new(ErrorDiagram::new(
            uml_source(diagram_type, lines, read),
            lines.to_vec(),
            message,
            None,
        )));
    }
    // Known first, so that diagrams rockuml cannot draw read no images.
    let create: Create = match diagram_type {
        Some(DiagramType::Creole) => |source, _| Ok(CreoleDiagram::create(source)),
        Some(DiagramType::Salt) => |source, _| Ok(salt::SaltDiagram::create(source)),
        Some(DiagramType::Uml) => builder::create_uml,
        Some(DiagramType::ChenEer) => |source, _| builder::create_chen(source),
        Some(DiagramType::MindMap) => |source, _| builder::create_mindmap(source),
        Some(DiagramType::Wbs) => |source, _| builder::create_wbs(source),
        Some(DiagramType::NwDiag) => |source, _| builder::create_nwdiag(source),
        Some(DiagramType::Packet) => |source, _| builder::create_packet(source),
        Some(DiagramType::Gantt) => |source, _| builder::create_gantt(source),
        Some(DiagramType::Json) => |source, _| Ok(json::create_json(source)),
        Some(DiagramType::Yaml) => |source, _| Ok(json::create_yaml(source)),
        Some(DiagramType::Hcl) => |source, _| Ok(json::create_hcl(source)),
        Some(DiagramType::Files) => |source, _| Ok(files::create(source)),
        Some(DiagramType::Board) => |source, _| builder::create_board(source),
        Some(DiagramType::Bpm) => |source, _| builder::create_bpm(source),
        Some(DiagramType::Chart) => |source, _| builder::create_chart(source),
        Some(DiagramType::Git) => |source, _| git::create(source),
        Some(DiagramType::Ebnf) => |source, _| builder::create_ebnf(source),
        Some(DiagramType::Regex) => |source, _| builder::create_regex(source),
        Some(DiagramType::Definition) => |source, _| Ok(definition::create(source)),
        Some(DiagramType::Wire) => |source, _| builder::create_wire(source),
        Some(DiagramType::Flow) => |source, _| builder::create_flow(source),
        Some(DiagramType::Dot) => |source, _| dot::create(source),
        _ => return Err(NotYetPorted("this diagram type")),
    };
    source.read_named_files(block.directory(), host);
    source.note_current_time(host);
    create(source, block.located_lines())
}

/// The diagram's source encoded as in a PlantUML server URL.
pub fn encoded_url(block: &PreprocessedBlock) -> String {
    // The URL encodes the source alone, so no image is read.
    let source = match create(block, &IsolatedHost) {
        Ok(diagram) => diagram.source().plain_string(),
        Err(_) => prepare(block).1.plain_string(),
    };
    crate::url_code::encode(&source)
}

fn prepare(block: &PreprocessedBlock) -> (Option<DiagramType>, UmlSource) {
    let lines = block.located_lines();
    let raw_lines = block.raw_lines().to_vec();
    let diagram_type = DiagramType::of_start_line(lines.first().map_or("", StringLocated::text));
    (diagram_type, uml_source(diagram_type, lines, raw_lines))
}

fn uml_source(
    diagram_type: Option<DiagramType>,
    lines: &[StringLocated],
    raw_lines: Vec<String>,
) -> UmlSource {
    let mut source = if diagram_type == Some(DiagramType::Uml) {
        UmlSource::with_continuations_joined(lines, raw_lines)
    } else {
        UmlSource::new(lines.to_vec(), raw_lines)
    };
    source.patch_base64();
    source
}

/// The image formats diagrams are exported to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    /// PlantUML's `debug` format, which lists every drawn shape.
    Debug,
    /// SVG with text measured with fonts.
    Svg,
    /// SVG with text measured by a fixed width table instead of fonts, identical on every machine.
    DeterministicSvg,
    Png,
}

/// The image's bytes, with the diagram's source embedded. `fonts` measure the text of formats that use
/// fonts, and draw it in PNG.
///
/// # Panics
///
/// If `page` is not below the diagram's page count, like an index out of bounds.
pub fn export(
    diagram: &dyn Diagram,
    page: usize,
    format: ImageFormat,
    fonts: &Arc<FontRegistry>,
    host: &dyn Host,
) -> Result<Vec<u8>, NotYetPorted> {
    export_with(diagram, page, format, Metadata::Embedded, fonts, host)
}

/// Like [`export`], with or without the diagram's source in SVG and PNG images.
///
/// # Panics
///
/// If `page` is not below the diagram's page count, like an index out of bounds.
pub fn export_with(
    diagram: &dyn Diagram,
    page: usize,
    format: ImageFormat,
    metadata: Metadata,
    fonts: &Arc<FontRegistry>,
    host: &dyn Host,
) -> Result<Vec<u8>, NotYetPorted> {
    export_image(diagram, page, format, metadata, fonts, host).map(|image| image.data)
}

/// An exported image and the size PlantUML reports for it (`ImageData`): the drawing with its margins,
/// before any scale, truncated to whole pixels.
pub struct ExportedImage {
    pub data: Vec<u8>,
    pub width: i32,
    pub height: i32,
    /// Whether the image reports that a program the diagram ran failed (PlantUML's `FileImageData.CRASH`).
    pub crashed: bool,
}

/// Like [`export_with`], with the image's size.
///
/// # Panics
///
/// If `page` is not below the diagram's page count, like an index out of bounds.
pub fn export_image(
    diagram: &dyn Diagram,
    page: usize,
    format: ImageFormat,
    metadata: Metadata,
    fonts: &Arc<FontRegistry>,
    host: &dyn Host,
) -> Result<ExportedImage, NotYetPorted> {
    let (mut exported, width, height) =
        export_pieces(diagram, page, format, metadata, fonts, host, false)?;
    Ok(ExportedImage {
        data: exported.files.remove(0),
        width,
        height,
        crashed: exported.crashed,
    })
}

/// The files PlantUML's command line writes for a page: its image, or for PNG the pieces `page 2x3` cuts
/// it into, across then down.
pub struct ExportedFiles {
    pub files: Vec<Vec<u8>>,
    /// Like [`ExportedImage::crashed`].
    pub crashed: bool,
}

impl ExportedFiles {
    fn single(file: Vec<u8>) -> Self {
        Self {
            files: vec![file],
            crashed: false,
        }
    }
}

/// The [`ExportedFiles`] of a page.
///
/// # Panics
///
/// If `page` is not below the diagram's page count, like an index out of bounds.
pub fn export_files(
    diagram: &dyn Diagram,
    page: usize,
    format: ImageFormat,
    metadata: Metadata,
    fonts: &Arc<FontRegistry>,
    host: &dyn Host,
) -> Result<ExportedFiles, NotYetPorted> {
    export_pieces(diagram, page, format, metadata, fonts, host, true).map(|(exported, ..)| exported)
}

/// The image, or its pieces when `cut` and the diagram asks for them; with the size of the whole.
fn export_pieces(
    diagram: &dyn Diagram,
    page: usize,
    format: ImageFormat,
    metadata: Metadata,
    fonts: &Arc<FontRegistry>,
    host: &dyn Host,
    cut: bool,
) -> Result<(ExportedFiles, i32, i32), NotYetPorted> {
    let direct_drawing = match diagram.export_diagram_now(format, host) {
        Some(DirectImage::Bytes(data)) => return Ok((ExportedFiles::single(data), 0, 0)),
        Some(DirectImage::Drawing(drawing)) => Some(drawing),
        None => None,
    };
    let crashed = direct_drawing.is_some();
    let source_metadata =
        (metadata == Metadata::Embedded && !crashed).then(|| diagram.source().metadata());
    let settings = diagram.export_settings(page);
    let string_bounder = string_bounder(format, fonts);
    let text_block = match direct_drawing {
        Some(drawing) => drawing,
        None => diagram.text_block_with_fonts(page, &string_bounder, fonts)?,
    };
    let margin = settings.margin;
    let dimension = text_block
        .calculate_dimension(string_bounder.as_ref())
        .delta(margin.left + margin.right, margin.top + margin.bottom);
    let scale_factor = settings
        .scale
        .map_or(1.0, |scale| scale.factor(dimension.width, dimension.height))
        * f64::from(settings.dpi)
        / f64::from(DEFAULT_DPI);
    let backcolor = settings
        .backcolor
        .clone()
        .or_else(|| text_block.backcolor())
        .unwrap_or(HColor::WHITE);
    let draw = |backend: Rc<RefCell<dyn UGraphicBackend>>, default_background: HColor| {
        let ug = UGraphic::new(backend, string_bounder.clone(), default_background);
        text_block.draw_u(&ug.translated(margin.left, margin.top));
    };
    let svg = |rasterized: bool| {
        let option = SvgOption {
            min_dim: dimension,
            backcolor: backcolor.clone(),
            scale: scale_factor,
            preserve_aspect_ratio: settings.preserve_aspect_ratio.clone(),
            root_attributes: settings
                .diagram_type
                .map(|name| ("data-diagram-type".to_owned(), name.to_owned()))
                .into_iter()
                .collect(),
            link_target: settings.svg_link_target.clone(),
            color_mapper: settings.color_mapper,
        };
        let output = Rc::new(RefCell::new(UGraphicSvg::new(
            settings.seed,
            option,
            string_bounder.clone(),
            (format != ImageFormat::DeterministicSvg).then(|| fonts.clone()),
            rasterized,
        )));
        draw(output.clone(), backcolor.clone());
        let encoded = source_metadata.as_deref().map(crate::url_code::encode);
        output.borrow_mut().take_document(encoded.as_deref())
    };

    let files = match format {
        ImageFormat::Debug => {
            let render_date = crate::tim::java_date_string(host.current_time_millis(), host);
            let output = Rc::new(RefCell::new(UGraphicDebug::new(render_date)));
            draw(output.clone(), HColor::WHITE);
            let header = DebugHeader {
                dimension,
                scale_factor,
                seed: settings.seed,
                svg_link_target: settings.svg_link_target.clone(),
                hover_path_color_rgb: None,
                preserve_aspect_ratio: settings.preserve_aspect_ratio.clone(),
            };
            vec![output.borrow().document(&header).into_bytes()]
        }
        ImageFormat::Svg | ImageFormat::DeterministicSvg => vec![svg(false).into_bytes()],
        ImageFormat::Png => {
            let limit = image_size_limit(host);
            // A gradient is painted by the drawing itself.
            let png_back_color = match backcolor {
                HColor::Simple(_) => backcolor.to_color(settings.color_mapper),
                _ => HColor::NONE.as_xcolor(),
            };
            let size = (
                ((dimension.width * scale_factor) as u32).min(limit),
                ((dimension.height * scale_factor) as u32).min(limit),
            );
            let metadata = source_metadata.as_deref();
            match settings.split.as_ref().filter(|_| cut) {
                Some(split) => {
                    png::rasterize_pieces(&svg(true), size, png_back_color, fonts, metadata, split)
                }
                None => vec![png::rasterize(
                    &svg(true),
                    size,
                    png_back_color,
                    fonts,
                    metadata,
                )],
            }
        }
    };
    let (width, height) = (dimension.width as i32, dimension.height as i32);
    Ok((ExportedFiles { files, crashed }, width, height))
}

/// How text is measured for `format`.
fn string_bounder(format: ImageFormat, fonts: &Arc<FontRegistry>) -> Rc<dyn StringBounder> {
    match format {
        ImageFormat::Debug => Rc::new(StringBounderDebug),
        ImageFormat::Svg | ImageFormat::Png => Rc::new(StringBounderFonts::new(fonts.clone())),
        ImageFormat::DeterministicSvg => Rc::new(StringBounderFromWidthTable),
    }
}

/// PlantUML crops images to `PLANTUML_LIMIT_SIZE` pixels each way. rockuml also refuses limits that would make
/// a single image need gigabytes of memory.
fn image_size_limit(host: &dyn Host) -> u32 {
    const DEFAULT_LIMIT: u32 = 4096;
    const LARGEST_LIMIT: u32 = 16_384;
    host.getenv("PLANTUML_LIMIT_SIZE")
        .as_deref()
        .and_then(parse_digits)
        .unwrap_or(DEFAULT_LIMIT)
        .min(LARGEST_LIMIT)
}

/// A setting written as digits only, as PlantUML accepts numeric settings.
fn parse_digits(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

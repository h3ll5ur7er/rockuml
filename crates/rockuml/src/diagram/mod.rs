//! Diagrams: recognising a block's diagram type, building the diagram, and exporting it.

mod activity3;
mod builder;
mod chen;
mod chrome;
mod class;
mod common_commands;
mod creole;
pub(crate) mod cuca;
mod cuca_commands;
mod description;
mod diagram_type;
mod error;
mod salt;
mod scale;
mod sequence;
mod source;
mod state;
mod titled;
mod unported;

pub(crate) use source::{BASE64_TAG_REPLACEMENT, BASE64_TAG_START};
pub(crate) use titled::entity_image_legend;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::color::HColor;
use crate::host::{Host, IsolatedHost};
use crate::klimt::TextBlock;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::png;
use crate::klimt::svg::{SvgOption, UGraphicSvg};
use crate::klimt::typeface::{FontRegistry, StringBounderFonts};
use crate::klimt::ugraphic::{UGraphic, UGraphicBackend};
use crate::klimt::width_table::StringBounderFromWidthTable;
use crate::preproc::PreprocessedBlock;
use crate::text::StringLocated;
use creole::CreoleDiagram;
use diagram_type::DiagramType;
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

    /// How many images the diagram makes: sequence diagrams break into pages with `newpage`.
    fn page_count(&self) -> usize {
        1
    }

    fn export_settings(&self) -> ExportSettings;

    /// Whether the diagram is an error image instead of what its source describes.
    fn is_error(&self) -> bool {
        false
    }
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
    // Known first, so that diagrams rockuml cannot draw read no images.
    let create: Create = match diagram_type {
        Some(DiagramType::Creole) => |source, _| Ok(CreoleDiagram::create(source)),
        Some(DiagramType::Salt) => |source, _| Ok(salt::SaltDiagram::create(source)),
        Some(DiagramType::Uml) => builder::create_uml,
        Some(DiagramType::ChenEer) => |source, _| builder::create_chen(source),
        _ => return Err(NotYetPorted("this diagram type")),
    };
    source.read_image_files(block.directory(), host);
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
    let mut source = if diagram_type == Some(DiagramType::Uml) {
        UmlSource::with_continuations_joined(lines, raw_lines)
    } else {
        UmlSource::new(lines.to_vec(), raw_lines)
    };
    source.patch_base64();
    (diagram_type, source)
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

/// The image's bytes. `fonts` measure the text of formats that use fonts, and draw it in PNG.
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
    let settings = diagram.export_settings();
    let string_bounder: Rc<dyn StringBounder> = match format {
        ImageFormat::Debug => Rc::new(StringBounderDebug),
        ImageFormat::Svg | ImageFormat::Png => Rc::new(StringBounderFonts::new(fonts.clone())),
        ImageFormat::DeterministicSvg => Rc::new(StringBounderFromWidthTable),
    };
    let text_block = diagram.text_block(page, &string_bounder)?;
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
        };
        let output = Rc::new(RefCell::new(UGraphicSvg::new(
            settings.seed,
            option,
            string_bounder.clone(),
            (format != ImageFormat::DeterministicSvg).then(|| fonts.clone()),
            rasterized,
        )));
        draw(output.clone(), backcolor.clone());
        let metadata = crate::url_code::encode(&diagram.source().metadata());
        output.borrow_mut().take_document(Some(&metadata))
    };

    Ok(match format {
        ImageFormat::Debug => {
            let render_date = crate::tim::java_date_string(host.current_time_millis(), host);
            let output = Rc::new(RefCell::new(UGraphicDebug::new(render_date)));
            draw(output.clone(), HColor::WHITE);
            output
                .borrow()
                .document(&DebugHeader {
                    dimension,
                    scale_factor,
                    seed: settings.seed,
                    svg_link_target: settings.svg_link_target.clone(),
                    hover_path_color_rgb: None,
                    preserve_aspect_ratio: settings.preserve_aspect_ratio.clone(),
                })
                .into_bytes()
        }
        ImageFormat::Svg | ImageFormat::DeterministicSvg => svg(false).into_bytes(),
        ImageFormat::Png => {
            let limit = image_size_limit(host);
            png::rasterize(
                &svg(true),
                (
                    ((dimension.width * scale_factor) as u32).min(limit),
                    ((dimension.height * scale_factor) as u32).min(limit),
                ),
                &backcolor,
                fonts,
                &diagram.source().metadata(),
            )
        }
    })
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

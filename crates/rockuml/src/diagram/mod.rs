//! Diagrams: recognising a block's diagram type, building the diagram, and exporting it.

mod common_commands;
mod creole;
mod diagram_type;
mod error;
mod salt;
mod source;
mod titled;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::color::HColor;
use crate::host::Host;
use crate::klimt::TextBlock;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::svg::{SvgOption, UGraphicSvg};
use crate::klimt::typeface::{FontRegistry, StringBounderFonts};
use crate::klimt::ugraphic::{UGraphic, UGraphicBackend};
use crate::klimt::width_table::StringBounderFromWidthTable;
use crate::preproc::PreprocessedBlock;
use crate::text::StringLocated;
use creole::CreoleDiagram;
use diagram_type::DiagramType;
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

    /// Everything the diagram draws.
    fn text_block(&self) -> Result<Box<dyn TextBlock + '_>, NotYetPorted>;

    fn export_settings(&self) -> ExportSettings;
}

/// How a diagram is placed on the image, and the image-wide options the skin decides.
pub struct ExportSettings {
    pub(super) margin: ClockwiseTopRightBottomLeft,
    pub(super) seed: i64,
    /// The skin's background; without a skin, the drawing's own or white.
    pub(super) backcolor: Option<HColor>,
    /// The type name SVG documents announce, for diagrams that have a skin.
    pub(super) diagram_type: Option<&'static str>,
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
            svg_link_target: None,
            preserve_aspect_ratio: "none".to_owned(),
        }
    }
}

pub fn create(block: &PreprocessedBlock) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let (diagram_type, source) = prepare(block);
    match diagram_type {
        Some(DiagramType::Creole) => Ok(CreoleDiagram::create(source)),
        Some(DiagramType::Salt) => Ok(salt::SaltDiagram::create(source)),
        _ => Err(NotYetPorted("this diagram type")),
    }
}

/// The diagram's source encoded as in a PlantUML server URL.
pub fn encoded_url(block: &PreprocessedBlock) -> String {
    let source = match create(block) {
        Ok(diagram) => diagram.source().plain_string(),
        Err(_) => prepare(block).1.plain_string(),
    };
    crate::url_code::encode(&source)
}

fn prepare(block: &PreprocessedBlock) -> (Option<DiagramType>, UmlSource) {
    let lines = block.located_lines();
    let raw_lines = block.raw_lines().to_vec();
    let diagram_type = DiagramType::of_start_line(lines.first().map_or("", StringLocated::text));
    let source = if diagram_type == Some(DiagramType::Uml) {
        UmlSource::with_continuations_joined(lines, raw_lines)
    } else {
        UmlSource::new(lines.to_vec(), raw_lines)
    };
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
}

/// `fonts` measure the text of formats that use fonts.
pub fn export(
    diagram: &dyn Diagram,
    format: ImageFormat,
    fonts: &Arc<FontRegistry>,
    host: &dyn Host,
) -> Result<String, NotYetPorted> {
    let settings = diagram.export_settings();
    let text_block = diagram.text_block()?;
    let string_bounder: Rc<dyn StringBounder> = match format {
        ImageFormat::Debug => Rc::new(StringBounderDebug),
        ImageFormat::Svg => Rc::new(StringBounderFonts::new(fonts.clone())),
        ImageFormat::DeterministicSvg => Rc::new(StringBounderFromWidthTable),
    };
    let margin = settings.margin;
    let dimension = text_block
        .calculate_dimension(string_bounder.as_ref())
        .delta(margin.left + margin.right, margin.top + margin.bottom);
    let draw = |backend: Rc<RefCell<dyn UGraphicBackend>>, default_background: HColor| {
        let ug = UGraphic::new(backend, string_bounder.clone(), default_background);
        text_block.draw_u(&ug.translated(margin.left, margin.top));
    };

    Ok(match format {
        ImageFormat::Debug => {
            let render_date = crate::tim::java_date_string(host.current_time_millis(), host);
            let output = Rc::new(RefCell::new(UGraphicDebug::new(render_date)));
            draw(output.clone(), HColor::WHITE);
            output.borrow().document(&DebugHeader {
                dimension,
                scale_factor: 1.0,
                seed: settings.seed,
                svg_link_target: settings.svg_link_target,
                hover_path_color_rgb: None,
                preserve_aspect_ratio: settings.preserve_aspect_ratio,
            })
        }
        ImageFormat::Svg | ImageFormat::DeterministicSvg => {
            let backcolor = settings
                .backcolor
                .or_else(|| text_block.backcolor())
                .unwrap_or(HColor::WHITE);
            let option = SvgOption {
                min_dim: dimension,
                backcolor: Some(backcolor.to_svg()),
                preserve_aspect_ratio: settings.preserve_aspect_ratio,
                root_attributes: settings
                    .diagram_type
                    .map(|name| ("data-diagram-type".to_owned(), name.to_owned()))
                    .into_iter()
                    .collect(),
            };
            let output = Rc::new(RefCell::new(UGraphicSvg::new(
                settings.seed,
                option,
                string_bounder.clone(),
            )));
            draw(output.clone(), backcolor);
            let metadata = crate::url_code::encode(&diagram.source().metadata());
            output.borrow_mut().take_document(Some(&metadata))
        }
    })
}

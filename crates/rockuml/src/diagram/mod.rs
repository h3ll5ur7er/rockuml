//! Diagrams: recognising a block's diagram type, building the diagram, and exporting it.

mod creole;
mod diagram_type;
mod source;

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::ugraphic::UGraphic;
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
    fn text_block(&self) -> Box<dyn TextBlock + '_>;

    fn export_settings(&self) -> ExportSettings;
}

/// How a diagram is placed on the image, and the image-wide options the skin decides.
pub struct ExportSettings {
    margin: ClockwiseTopRightBottomLeft,
    seed: i64,
    svg_link_target: Option<String>,
    preserve_aspect_ratio: String,
}

impl ExportSettings {
    /// For diagrams without a skin: no margin and default options.
    fn without_skin(seed: i64) -> Self {
        Self {
            margin: ClockwiseTopRightBottomLeft::none(),
            seed,
            svg_link_target: None,
            preserve_aspect_ratio: "none".to_owned(),
        }
    }
}

pub fn create(block: &PreprocessedBlock) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let (diagram_type, source) = prepare(block.located_lines());
    match diagram_type {
        Some(DiagramType::Creole) => Ok(Box::new(CreoleDiagram::create(source)?)),
        _ => Err(NotYetPorted("this diagram type")),
    }
}

/// The diagram's source encoded as in a PlantUML server URL.
pub fn encoded_url(block: &PreprocessedBlock) -> String {
    let source = match create(block) {
        Ok(diagram) => diagram.source().plain_string(),
        Err(_) => prepare(block.located_lines()).1.plain_string(),
    };
    crate::url_code::encode(&source)
}

fn prepare(lines: &[StringLocated]) -> (Option<DiagramType>, UmlSource) {
    let diagram_type = DiagramType::of_start_line(lines.first().map_or("", StringLocated::text));
    let source = if diagram_type == Some(DiagramType::Uml) {
        UmlSource::with_continuations_joined(lines)
    } else {
        UmlSource::new(lines.to_vec())
    };
    (diagram_type, source)
}

/// The diagram in PlantUML's `debug` format, which lists every drawn shape.
pub fn export_debug(diagram: &dyn Diagram) -> String {
    let settings = diagram.export_settings();
    let text_block = diagram.text_block();
    let string_bounder = Rc::new(StringBounderDebug);
    let margin = settings.margin;
    let dimension = text_block
        .calculate_dimension(string_bounder.as_ref())
        .delta(margin.left + margin.right, margin.top + margin.bottom);

    let output = Rc::new(RefCell::new(UGraphicDebug::default()));
    let ug = UGraphic::new(
        output.clone(),
        string_bounder as Rc<dyn StringBounder>,
        HColor::WHITE,
    );
    text_block.draw_u(&ug.translated(margin.left, margin.top));

    output.take().into_document(&DebugHeader {
        dimension,
        scale_factor: 1.0,
        seed: settings.seed,
        svg_link_target: settings.svg_link_target,
        hover_path_color_rgb: None,
        preserve_aspect_ratio: settings.preserve_aspect_ratio,
    })
}

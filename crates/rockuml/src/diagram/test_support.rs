//! Helpers for the tests of diagrams.

use std::rc::Rc;
use std::sync::Arc;

use super::{Diagram, DiagramError, ImageFormat, NotYetPorted};
use crate::host::IsolatedHost;
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::font::StringBounder;
use crate::klimt::typeface::FontRegistry;
use crate::preproc::{PreprocessorEnvironment, Source};

/// Why the first diagram of `text` is an error image, if it is one.
pub(crate) fn error_of(text: &str) -> Option<DiagramError> {
    first_diagram(text).unwrap().error()
}

/// The part of PlantUML rockuml lacks to draw the first diagram of `text`, if any.
pub(crate) fn not_ported_of(text: &str) -> Option<NotYetPorted> {
    let string_bounder: Rc<dyn StringBounder> = Rc::new(StringBounderDebug);
    let diagram = first_diagram(text);
    diagram
        .and_then(|diagram| diagram.text_block(0, &string_bounder).map(drop))
        .err()
}

/// The `debug` description of the first diagram of `text`, drawn with `fonts`.
pub(crate) fn debug_of(text: &str, fonts: FontRegistry) -> String {
    let image = super::export(
        first_diagram(text).unwrap().as_ref(),
        0,
        ImageFormat::Debug,
        &Arc::new(fonts),
        &IsolatedHost,
    )
    .unwrap();
    String::from_utf8(image).unwrap()
}

fn first_diagram(text: &str) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let source = Source {
        text,
        description: "t",
        directory: std::path::PathBuf::new(),
        environment: PreprocessorEnvironment::default(),
    };
    let block = crate::preproc::preprocess(&source, &IsolatedHost).remove(0);
    super::create(&block, &IsolatedHost)
}

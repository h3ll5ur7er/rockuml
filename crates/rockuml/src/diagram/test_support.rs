//! Helpers for the tests of diagrams.

use std::sync::Arc;

use super::{Diagram, DiagramError, ImageFormat};
use crate::host::IsolatedHost;
use crate::klimt::typeface::FontRegistry;
use crate::preproc::{PreprocessorEnvironment, Source};

/// The first diagram of `text`.
fn diagram_of(text: &str) -> Box<dyn Diagram> {
    let source = Source {
        text,
        description: "t",
        directory: std::path::PathBuf::new(),
        environment: PreprocessorEnvironment::default(),
    };
    let block = crate::preproc::preprocess(&source, &IsolatedHost).remove(0);
    super::create(&block, &IsolatedHost).unwrap()
}

/// Why the first diagram of `text` is an error image, if it is one.
pub(crate) fn error_of(text: &str) -> Option<DiagramError> {
    diagram_of(text).error()
}

/// The `debug` description of the first diagram of `text`, drawn with `fonts`.
pub(crate) fn debug_of(text: &str, fonts: FontRegistry) -> String {
    let image = super::export(
        diagram_of(text).as_ref(),
        0,
        ImageFormat::Debug,
        &Arc::new(fonts),
        &IsolatedHost,
    )
    .unwrap();
    String::from_utf8(image).unwrap()
}

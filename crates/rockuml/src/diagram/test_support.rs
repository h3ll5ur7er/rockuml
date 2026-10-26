//! Helpers for the tests of diagrams.

use std::rc::Rc;

use super::{Diagram, DiagramError, NotYetPorted};
use crate::host::IsolatedHost;
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::font::StringBounder;
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

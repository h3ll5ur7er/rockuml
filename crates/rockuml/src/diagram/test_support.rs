//! Helpers for the tests of diagrams.

use super::DiagramError;
use crate::host::IsolatedHost;
use crate::preproc::{PreprocessorEnvironment, Source};

/// Why the first diagram of `text` is an error image, if it is one.
pub(crate) fn error_of(text: &str) -> Option<DiagramError> {
    let source = Source {
        text,
        description: "t",
        directory: std::path::PathBuf::new(),
        environment: PreprocessorEnvironment::default(),
    };
    let block = crate::preproc::preprocess(&source, &IsolatedHost).remove(0);
    super::create(&block, &IsolatedHost).unwrap().error()
}

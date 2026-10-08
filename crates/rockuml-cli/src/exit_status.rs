//! What a run found, and the exit status that tells it (PlantUML's `ExitStatus`).

use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) const OK: u8 = 0;
/// rockuml's own: some diagrams need parts of PlantUML that are not ported yet.
pub(crate) const NOT_PORTED: u8 = 1;
pub(crate) const ERROR_50_NO_FILE_FOUND: u8 = 50;
pub(crate) const ERROR_100_NO_DIAGRAM_FOUND: u8 = 100;
pub(crate) const ERROR_200_SOME_DIAGRAMS_HAVE_ERROR: u8 = 200;
/// A command line PlantUML cannot parse.
pub(crate) const CLI_PARSING_ERROR: u8 = 42;

/// Shared by the threads that process the inputs.
#[derive(Default)]
pub(crate) struct ExitStatus {
    has_files: AtomicBool,
    has_blocks: AtomicBool,
    has_errors: AtomicBool,
    not_ported: AtomicBool,
}

impl ExitStatus {
    pub(crate) fn goes_has_files(&self) {
        self.has_files.store(true, Ordering::SeqCst);
    }

    pub(crate) fn goes_has_blocks(&self) {
        self.has_blocks.store(true, Ordering::SeqCst);
    }

    pub(crate) fn goes_has_errors(&self) {
        self.has_errors.store(true, Ordering::SeqCst);
    }

    pub(crate) fn goes_not_ported(&self) {
        self.not_ported.store(true, Ordering::SeqCst);
    }

    pub(crate) fn has_errors(&self) -> bool {
        self.has_errors.load(Ordering::SeqCst)
    }

    pub(crate) fn no_diagram_found(&self) -> bool {
        !self.has_blocks.load(Ordering::SeqCst)
    }

    /// Diagrams rockuml could not render at all matter more than those PlantUML would reject.
    pub(crate) fn exit_code(&self) -> u8 {
        if self.not_ported.load(Ordering::SeqCst) {
            NOT_PORTED
        } else if self.has_errors() {
            ERROR_200_SOME_DIAGRAMS_HAVE_ERROR
        } else if !self.has_files.load(Ordering::SeqCst) {
            ERROR_50_NO_FILE_FOUND
        } else if self.no_diagram_found() {
            ERROR_100_NO_DIAGRAM_FOUND
        } else {
            OK
        }
    }

    /// What `--help` lists.
    pub(crate) fn exit_codes() -> [(u8, &'static str); 5] {
        [
            (OK, "Success"),
            (
                NOT_PORTED,
                "Some diagrams use parts of PlantUML not ported yet",
            ),
            (ERROR_50_NO_FILE_FOUND, "No file found"),
            (ERROR_100_NO_DIAGRAM_FOUND, "No diagram found in file(s)"),
            (
                ERROR_200_SOME_DIAGRAMS_HAVE_ERROR,
                "Some diagrams have syntax errors",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_outrank_missing_files_and_diagrams() {
        let status = ExitStatus::default();
        assert_eq!(status.exit_code(), ERROR_50_NO_FILE_FOUND);
        status.goes_has_files();
        assert_eq!(status.exit_code(), ERROR_100_NO_DIAGRAM_FOUND);
        status.goes_has_blocks();
        assert_eq!(status.exit_code(), OK);
        status.goes_has_errors();
        assert_eq!(status.exit_code(), ERROR_200_SOME_DIAGRAMS_HAVE_ERROR);
        status.goes_not_ported();
        assert_eq!(status.exit_code(), NOT_PORTED);
    }
}

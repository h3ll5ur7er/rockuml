//! Standard output and error, buffered per task so that work done in parallel prints in input order.

use std::io::{self, Write};

/// PlantUML writes text with the platform's line separator.
pub(crate) const LINE_SEPARATOR: &str = if cfg!(windows) { "\r\n" } else { "\n" };

#[derive(Default)]
pub(crate) struct Console {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Console {
    /// Text whose lines end with `\n`, written with the platform's line separator.
    pub(crate) fn print(&mut self, text: &str) {
        self.stdout
            .extend(text.replace('\n', LINE_SEPARATOR).into_bytes());
    }

    pub(crate) fn println(&mut self, line: &str) {
        self.stdout.extend(line.as_bytes());
        self.stdout.extend(LINE_SEPARATOR.as_bytes());
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) {
        self.stdout.extend(bytes);
    }

    pub(crate) fn error(&mut self, line: &str) {
        self.stderr.extend(line.as_bytes());
        self.stderr.extend(LINE_SEPARATOR.as_bytes());
    }

    /// Writes what was buffered. A reader that closed the pipe (`rockuml ... | head`) simply wants no
    /// more output.
    pub(crate) fn flush(&mut self) {
        for (buffer, mut target) in [
            (
                &mut self.stdout,
                Box::new(io::stdout().lock()) as Box<dyn Write>,
            ),
            (&mut self.stderr, Box::new(io::stderr().lock())),
        ] {
            if !buffer.is_empty() {
                let _ = target.write_all(buffer).and_then(|()| target.flush());
                buffer.clear();
            }
        }
    }
}

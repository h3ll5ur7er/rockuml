//! `-pipe`: diagrams read from standard input, their images written to standard output (PlantUML's `Pipe`).

use std::io::{self, BufRead, Read};
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use rockuml::diagram::Diagram;
use rockuml::preproc::{PreprocessedBlock, PreprocessorEnvironment, Source};

use crate::charset::Charset;
use crate::cli_flag::CliFlag;
use crate::console::Console;
use crate::crash::{self, Unrendered};
use crate::exit_status::ExitStatus;
use crate::file_format::FileFormat;
use crate::run::Settings;

/// Processes each diagram as soon as it has been read, so that a program can send one diagram, read its
/// image, then send the next.
pub(crate) fn manage_pipe(settings: &Settings, status: &ExitStatus, console: &mut Console) {
    let options = &settings.options;
    let no_stderr = options.is_true(CliFlag::Pipenostderr);
    let no_error_image = no_stderr || options.is_true(CliFlag::NoErrorImage);
    let mut reader = DiagramReader {
        lines: lines_of_stdin(settings.charset),
        format: Ok(settings.format),
    };
    let mut source = reader.read_single_diagram(true);
    while let Some(text) = source {
        let blocks = match crash::catch(|| preprocess(&text, settings)) {
            Ok(blocks) => blocks,
            Err(message) => {
                console.error(&format!("rockuml: {STDIN}: crashed: {message}"));
                status.goes_has_errors();
                source = reader.read_single_diagram(false);
                continue;
            }
        };
        let diagrams: Vec<_> = blocks
            .iter()
            .map(|block| crash::render(|| rockuml::diagram::create(block, &settings.host)))
            .collect();
        for diagram in &diagrams {
            status.goes_has_blocks();
            let is_error = match diagram {
                Ok(diagram) => diagram.is_error(),
                Err(unrendered) => matches!(unrendered, Unrendered::Crashed(_)),
            };
            if is_error {
                status.goes_has_errors();
            }
        }
        status.goes_has_files();
        if options.is_true(CliFlag::ComputeUrl) {
            for block in &blocks {
                console.println(&rockuml::diagram::encoded_url(block));
            }
        } else {
            let format = reader.format.clone();
            generate_diagram(
                settings,
                format,
                &blocks,
                &diagrams,
                no_stderr,
                no_error_image,
                status,
                console,
            );
        }
        console.flush();
        source = reader.read_single_diagram(false);
    }
}

/// How the reports of diagrams read from standard input name their missing images.
const STDIN: &str = "<stdin>";

type Created = Result<Box<dyn Diagram>, Unrendered>;

#[expect(clippy::too_many_arguments, reason = "mirrors Pipe.generateDiagram")]
fn generate_diagram(
    settings: &Settings,
    format: Result<FileFormat, String>,
    blocks: &[PreprocessedBlock],
    diagrams: &[Created],
    no_stderr: bool,
    no_error_image: bool,
    status: &ExitStatus,
    console: &mut Console,
) {
    let image = match format {
        Ok(format) => output_image(settings, format, blocks, diagrams, status, console),
        Err(not_ported) => {
            console.error(&format!("rockuml: {not_ported}"));
            status.goes_not_ported();
            None
        }
    };
    let first_error = diagrams
        .first()
        .and_then(|diagram| diagram.as_ref().ok())
        .and_then(|diagram| diagram.error());
    if let Some(error) = &first_error {
        let report = format!("ERROR\n{}\n{}\n", error.line, error.message);
        if no_stderr {
            console.print(&report);
        } else {
            for line in report.lines() {
                console.error(line);
            }
        }
    }
    if let Some(image) = image
        && !(no_error_image && image.is_error)
    {
        console.write(&image.bytes);
    }
    if let Some(delimiter) = settings.options.get_string(CliFlag::Pipedelimitor) {
        console.println(delimiter);
    }
}

struct Image {
    bytes: Vec<u8>,
    is_error: bool,
}

/// Image number `--pipe-image-index` of the diagrams, counting every page (`outputImage`).
fn output_image(
    settings: &Settings,
    format: FileFormat,
    blocks: &[PreprocessedBlock],
    diagrams: &[Created],
    status: &ExitStatus,
    console: &mut Console,
) -> Option<Image> {
    if format == FileFormat::Preproc {
        let first = blocks.first()?;
        let bytes = first
            .lines()
            .flat_map(|line| [line, "\n"])
            .collect::<String>()
            .into_bytes();
        return Some(Image {
            bytes,
            is_error: false,
        });
    }
    let mut num_image = settings.options.image_index();
    for diagram in diagrams {
        let diagram = match diagram {
            Ok(diagram) => diagram,
            Err(unrendered) => {
                unrendered.report(STDIN, status, console);
                return None;
            }
        };
        if num_image < diagram.page_count() {
            let image_format = format
                .image_format()
                .expect("drawing formats export images");
            let exported = crash::render(|| {
                rockuml::diagram::export_with(
                    diagram.as_ref(),
                    num_image,
                    image_format,
                    settings.options.metadata(),
                    &settings.fonts,
                    &settings.host,
                )
            });
            return match exported {
                Ok(_) if format == FileFormat::Null => None,
                Ok(bytes) => Some(Image {
                    bytes,
                    is_error: diagram.is_error(),
                }),
                Err(unrendered) => {
                    unrendered.report(STDIN, status, console);
                    None
                }
            };
        }
        num_image -= diagram.page_count();
    }
    console.error(&format!("numImage is too big = {num_image}"));
    None
}

fn preprocess(text: &str, settings: &Settings) -> Vec<PreprocessedBlock> {
    let options = &settings.options;
    let file_dir = options.get_string(CliFlag::FileDir);
    let source = Source {
        text,
        description: "string",
        directory: file_dir.map_or_else(
            || std::env::current_dir().unwrap_or_default(),
            PathBuf::from,
        ),
        environment: PreprocessorEnvironment {
            filename: options.get_string(CliFlag::Filename).map(str::to_owned),
            dirpath: file_dir.map(|directory| directory.replace('\\', "/")),
            defines: options.defines(),
            config: options.config().to_vec(),
            ..PreprocessorEnvironment::default()
        },
    };
    rockuml::preproc::preprocess(&source, &settings.host)
}

/// Standard input's lines without their line ends. UTF-16 input is read whole, as its line ends cannot be
/// found byte by byte.
fn lines_of_stdin(charset: Charset) -> Box<dyn Iterator<Item = String>> {
    if matches!(
        charset,
        Charset::Utf16 | Charset::Utf16BigEndian | Charset::Utf16LittleEndian
    ) {
        let mut bytes = Vec::new();
        let _ = io::stdin().lock().read_to_end(&mut bytes);
        let text = charset.decode(&bytes);
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        return Box::new(lines.into_iter());
    }
    let mut stdin = io::stdin().lock();
    Box::new(std::iter::from_fn(move || {
        let mut line = Vec::new();
        match stdin.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => {
                let end = line.strip_suffix(b"\n").unwrap_or(&line);
                let end = end.strip_suffix(b"\r").unwrap_or(end);
                Some(charset.decode(end))
            }
        }
    }))
}

struct DiagramReader {
    lines: Box<dyn Iterator<Item = String>>,
    /// The format `@@@format` lines switch to; `Err` names one that is not ported.
    format: Result<FileFormat, String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    NoContent,
    StartMarkNotFound,
    StartMarkFound,
    Complete,
}

impl DiagramReader {
    /// The next diagram from its `@start` line to its `@end` line, which is added if missing. Text before
    /// the first `@start` line makes a `@startuml` diagram when `unmarked_allowed`.
    fn read_single_diagram(&mut self, unmarked_allowed: bool) -> Option<String> {
        static START: LazyLock<Regex> =
            LazyLock::new(|| Regex::new("^@start([A-Za-z]*)$").unwrap());
        let mut state = State::NoContent;
        let mut expected_end = String::new();
        let mut text = String::new();
        while state != State::Complete {
            let Some(line) = self.lines.next() else {
                break;
            };
            if line.starts_with("@@@format ") {
                self.manage_format(&line);
                continue;
            }
            if state == State::NoContent && !line.trim_matches(|c| c <= ' ').is_empty() {
                state = State::StartMarkNotFound;
            }
            if state == State::StartMarkNotFound && line.starts_with("@start") {
                text.clear();
                state = State::StartMarkFound;
                expected_end = START.captures(&line).map_or_else(
                    || "@end".to_owned(),
                    |captures| format!("@end{}", &captures[1]),
                );
            } else if state == State::StartMarkFound && line.starts_with(&expected_end) {
                state = State::Complete;
            }
            if state != State::NoContent {
                text.push_str(&line);
                text.push('\n');
            }
        }
        match state {
            State::NoContent => None,
            State::StartMarkNotFound => {
                unmarked_allowed.then(|| format!("@startuml\n{text}@enduml\n"))
            }
            State::StartMarkFound => Some(text + &expected_end),
            State::Complete => Some(text),
        }
    }

    fn manage_format(&mut self, line: &str) {
        let line = line.to_lowercase();
        if line.contains("png") {
            self.format = Ok(FileFormat::Png);
        } else if line.contains("svg") {
            self.format = Ok(FileFormat::Svg);
        } else if line.contains("atxt") || line.contains("utxt") {
            self.format = Err("txt output is not ported yet".to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagrams(input: &str) -> Vec<String> {
        let lines: Vec<String> = input.lines().map(str::to_owned).collect();
        let mut reader = DiagramReader {
            lines: Box::new(lines.into_iter()),
            format: Ok(FileFormat::Png),
        };
        let mut result = Vec::new();
        let mut unmarked_allowed = true;
        while let Some(diagram) = reader.read_single_diagram(unmarked_allowed) {
            result.push(diagram);
            unmarked_allowed = false;
        }
        result
    }

    #[test]
    fn diagrams_run_from_start_to_end_line() {
        assert_eq!(
            diagrams("\n@startuml\nA -> B\n@enduml\n@startmindmap\n* x\n@endmindmap\n"),
            [
                "@startuml\nA -> B\n@enduml\n",
                "@startmindmap\n* x\n@endmindmap\n"
            ]
        );
    }

    #[test]
    fn missing_marks_are_added() {
        assert_eq!(diagrams("A -> B"), ["@startuml\nA -> B\n@enduml\n"]);
        assert_eq!(
            diagrams("@startuml\nA -> B"),
            ["@startuml\nA -> B\n@enduml"]
        );
    }

    #[test]
    fn format_lines_switch_the_format() {
        let lines = vec!["@@@format svg".to_owned()];
        let mut reader = DiagramReader {
            lines: Box::new(lines.into_iter()),
            format: Ok(FileFormat::Png),
        };
        assert_eq!(reader.read_single_diagram(true), None);
        assert_eq!(reader.format, Ok(FileFormat::Svg));
    }
}

mod fonts;
mod naming;
mod options;
mod system_host;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use naming::OutputNamer;
use options::{Command, OutputFormat, RenderOptions};
use rockuml::diagram::{ImageFormat, NotYetPorted};
use rockuml::fonts::FontRegistry;
use rockuml::preproc::{PreprocessedBlock, PreprocessorEnvironment, Source};
use system_host::SystemHost;

/// PlantUML writes text output with the platform's line separator.
const LINE_SEPARATOR: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// PlantUML's exit status when at least one diagram has errors.
const DIAGRAM_ERROR_STATUS: u8 = 200;

/// Deeply nested diagrams recurse deeply; the main thread's stack is only 1 MiB on Windows.
const STACK_SIZE: usize = 64 * 1024 * 1024;

fn main() -> ExitCode {
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(run)
        .expect("a thread can be started")
        .join()
        .unwrap_or(ExitCode::FAILURE)
}

fn run() -> ExitCode {
    let result = match options::parse(std::env::args().skip(1)) {
        Ok(Command::Version) => print_version(),
        Ok(Command::DecodeUrl(codes)) => decode_urls(&codes),
        Ok(Command::Render(options)) => render_all(&options),
        Err(error) => Err(error),
    };
    match result {
        Ok(ExitStatus::Success) => ExitCode::SUCCESS,
        Ok(ExitStatus::DiagramErrors) => {
            eprintln!("Some diagram description contains errors");
            ExitCode::from(DIAGRAM_ERROR_STATUS)
        }
        Ok(ExitStatus::SomeNotRendered) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("rockuml: {error}");
            ExitCode::FAILURE
        }
    }
}

enum ExitStatus {
    Success,
    DiagramErrors,
    /// Some diagrams need parts of PlantUML that rockuml does not have yet.
    SomeNotRendered,
}

fn print_version() -> Result<ExitStatus, String> {
    let version = format!(
        "rockuml {} (PlantUML {} compatible){LINE_SEPARATOR}",
        env!("CARGO_PKG_VERSION"),
        rockuml::PLANTUML_VERSION
    );
    write_stdout(&version)?;
    Ok(ExitStatus::Success)
}

/// PlantUML wraps the decoded source, which already has its own start and end lines, in another pair.
fn decode_urls(codes: &[String]) -> Result<ExitStatus, String> {
    for code in codes {
        let source =
            rockuml::url_code::decode(code).map_err(|_| format!("not a PlantUML code: {code}"))?;
        write_stdout(&format!(
            "@startuml{LINE_SEPARATOR}{source}{LINE_SEPARATOR}@enduml{LINE_SEPARATOR}"
        ))?;
    }
    Ok(ExitStatus::Success)
}

fn render_all(options: &RenderOptions) -> Result<ExitStatus, String> {
    let fonts = if options.format.measures_with_fonts() {
        fonts::load(&options.fonts)?
    } else {
        FontRegistry::default()
    };
    let fonts = Arc::new(fonts);
    let mut outcome = Outcome::default();
    for file in &options.files {
        let blocks = preprocess_file(file)?;
        if blocks.iter().any(PreprocessedBlock::failed)
            && options.format != OutputFormat::EncodedUrl
        {
            outcome.diagram_errors = true;
        }
        outcome.merge(write_outputs(file, &blocks, options, &fonts)?);
    }
    Ok(outcome.status())
}

/// What happened to the diagrams of a run.
#[derive(Clone, Copy, Default)]
struct Outcome {
    diagram_errors: bool,
    not_rendered: bool,
}

impl Outcome {
    fn merge(&mut self, other: Outcome) {
        self.diagram_errors |= other.diagram_errors;
        self.not_rendered |= other.not_rendered;
    }

    /// Diagrams rockuml could not render at all matter more than those it rendered as error images.
    fn status(self) -> ExitStatus {
        if self.not_rendered {
            ExitStatus::SomeNotRendered
        } else if self.diagram_errors {
            ExitStatus::DiagramErrors
        } else {
            ExitStatus::Success
        }
    }
}

fn preprocess_file(file: &Path) -> Result<Vec<PreprocessedBlock>, String> {
    let bytes =
        fs::read(file).map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let file_name = file_name(file);
    let source = Source {
        text: &text,
        description: &file_name,
        directory: std::path::absolute(file)
            .ok()
            .and_then(|absolute| absolute.parent().map(Path::to_path_buf))
            .unwrap_or_default(),
        environment: environment_of(file, &file_name),
    };
    Ok(rockuml::preproc::preprocess(&source, &SystemHost))
}

/// Blocks that cannot be rendered are reported and skipped.
fn write_outputs(
    file: &Path,
    blocks: &[PreprocessedBlock],
    options: &RenderOptions,
    fonts: &Arc<FontRegistry>,
) -> Result<Outcome, String> {
    if options.format == OutputFormat::EncodedUrl {
        for block in blocks {
            write_stdout(&format!(
                "{}{LINE_SEPARATOR}",
                rockuml::diagram::encoded_url(block)
            ))?;
        }
        return Ok(Outcome::default());
    }
    let mut outcome = Outcome::default();
    let output_directory = output_directory(file, options.output_directory.as_deref());
    fs::create_dir_all(&output_directory)
        .map_err(|error| format!("cannot create {}: {error}", output_directory.display()))?;
    let mut namer = OutputNamer::new(&file_name(file), options.format.suffix());
    for block in blocks {
        let name_from_diagram = block.output_name();
        let pages: Vec<Vec<u8>> = match options.format {
            OutputFormat::Preprocessed => vec![
                block
                    .lines()
                    .flat_map(|line| [line, LINE_SEPARATOR])
                    .collect::<String>()
                    .into_bytes(),
            ],
            format => {
                let image_format = format
                    .image_format()
                    .ok_or_else(|| format!("{format:?} output is not implemented yet"))?;
                match render(block, image_format, fonts) {
                    Ok(Rendered { pages, is_error }) => {
                        outcome.diagram_errors |= is_error;
                        pages
                    }
                    Err(not_ported) => {
                        let output = output_directory
                            .join(namer.next_names(name_from_diagram.as_deref(), 1).remove(0));
                        eprintln!("rockuml: {}: {not_ported}", output.display());
                        outcome.not_rendered = true;
                        continue;
                    }
                }
            }
        };
        let names = namer.next_names(name_from_diagram.as_deref(), pages.len());
        for (name, content) in names.into_iter().zip(pages) {
            let output = output_directory.join(name);
            fs::write(&output, content)
                .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
        }
    }
    Ok(outcome)
}

struct Rendered {
    /// One image per page.
    pages: Vec<Vec<u8>>,
    /// The image shows the diagram's errors instead of the diagram.
    is_error: bool,
}

fn render(
    block: &PreprocessedBlock,
    format: ImageFormat,
    fonts: &Arc<FontRegistry>,
) -> Result<Rendered, NotYetPorted> {
    let diagram = rockuml::diagram::create(block)?;
    let pages = (0..diagram.page_count())
        .map(|page| rockuml::diagram::export(diagram.as_ref(), page, format, fonts, &SystemHost))
        .collect::<Result<_, _>>()?;
    Ok(Rendered {
        pages,
        is_error: diagram.is_error(),
    })
}

/// A reader that closed the pipe (`rockuml ... | head`) simply wants no more output.
fn write_stdout(text: &str) -> Result<(), String> {
    match io::stdout().lock().write_all(text.as_bytes()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => {
            Err(format!("cannot write output: {error}"))
        }
        _ => Ok(()),
    }
}

fn file_name(file: &Path) -> String {
    file.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// What `%filename()` and `%filedate()` report. `%dirpath()` stays empty: PlantUML only reveals the
/// directory under its INSECURE security profile.
fn environment_of(file: &Path, file_name: &str) -> PreprocessorEnvironment {
    let modified = fs::metadata(file)
        .and_then(|metadata| metadata.modified())
        .ok();
    PreprocessorEnvironment {
        filename: Some(file_name.to_owned()),
        filedate: modified.map(|time| {
            rockuml::preproc::java_date_string(system_host::millis_since_epoch(time), &SystemHost)
        }),
        ..PreprocessorEnvironment::default()
    }
}

/// Like PlantUML, a relative `-o` directory is taken relative to the input file.
fn output_directory(input: &Path, requested: Option<&Path>) -> PathBuf {
    let input_directory = input.parent().unwrap_or(Path::new("")).to_path_buf();
    match requested {
        Some(directory) if directory.is_absolute() => directory.to_path_buf(),
        Some(directory) => input_directory.join(directory),
        None => input_directory,
    }
}

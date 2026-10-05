mod naming;
mod options;
mod system_host;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use naming::OutputNamer;
use options::{Command, OutputFormat, RenderOptions};
use rockuml::preproc::{PreprocessedBlock, PreprocessorEnvironment, Source};
use system_host::SystemHost;

/// PlantUML writes text output with the platform's line separator.
const LINE_SEPARATOR: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// PlantUML's exit status when at least one diagram has errors.
const DIAGRAM_ERROR_STATUS: u8 = 200;

fn main() -> ExitCode {
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
    let mut status = ExitStatus::Success;
    for file in &options.files {
        let blocks = preprocess_file(file)?;
        if blocks.iter().any(PreprocessedBlock::failed)
            && options.format != OutputFormat::EncodedUrl
        {
            status = ExitStatus::DiagramErrors;
        }
        if !write_outputs(file, &blocks, options)? {
            status = ExitStatus::SomeNotRendered;
        }
    }
    Ok(status)
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

/// Whether every block could be rendered; those that cannot are reported and skipped.
fn write_outputs(
    file: &Path,
    blocks: &[PreprocessedBlock],
    options: &RenderOptions,
) -> Result<bool, String> {
    if options.format == OutputFormat::EncodedUrl {
        for block in blocks {
            write_stdout(&format!(
                "{}{LINE_SEPARATOR}",
                rockuml::diagram::encoded_url(block)
            ))?;
        }
        return Ok(true);
    }
    let mut all_rendered = true;
    let output_directory = output_directory(file, options.output_directory.as_deref());
    fs::create_dir_all(&output_directory)
        .map_err(|error| format!("cannot create {}: {error}", output_directory.display()))?;
    let mut namer = OutputNamer::new(&file_name(file), options.format.suffix());
    for block in blocks {
        let output = output_directory.join(namer.next_name(block.output_name().as_deref()));
        let content = match options.format {
            OutputFormat::Preprocessed => block
                .lines()
                .flat_map(|line| [line, LINE_SEPARATOR])
                .collect::<String>(),
            OutputFormat::Debug => match rockuml::diagram::create(block) {
                Ok(diagram) => rockuml::diagram::export_debug(diagram.as_ref()),
                Err(not_ported) => {
                    eprintln!("rockuml: {}: {not_ported}", output.display());
                    all_rendered = false;
                    continue;
                }
            },
            format => return Err(format!("{format:?} output is not implemented yet")),
        };
        fs::write(&output, content)
            .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    }
    Ok(all_rendered)
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

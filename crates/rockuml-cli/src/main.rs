mod naming;
mod options;
mod system_host;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use naming::OutputNamer;
use options::{Command, OutputFormat, RenderOptions};
use rockuml::preproc::{PreprocessorEnvironment, Source};
use system_host::SystemHost;

/// PlantUML writes text output with the platform's line separator.
const LINE_SEPARATOR: &str = if cfg!(windows) { "\r\n" } else { "\n" };

fn main() -> ExitCode {
    match options::parse(std::env::args().skip(1)) {
        Ok(Command::Version) => {
            println!(
                "rockuml {} (PlantUML {} compatible)",
                env!("CARGO_PKG_VERSION"),
                rockuml::PLANTUML_VERSION
            );
            ExitCode::SUCCESS
        }
        Ok(Command::Render(options)) => match render_all(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("rockuml: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("rockuml: {error}");
            ExitCode::FAILURE
        }
    }
}

fn render_all(options: &RenderOptions) -> Result<(), String> {
    for file in &options.files {
        render_file(file, options)?;
    }
    Ok(())
}

fn render_file(file: &Path, options: &RenderOptions) -> Result<(), String> {
    let source =
        fs::read(file).map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let source = String::from_utf8_lossy(&source);
    let file_name = file
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let output_directory = output_directory(file, options.output_directory.as_deref());
    fs::create_dir_all(&output_directory)
        .map_err(|error| format!("cannot create {}: {error}", output_directory.display()))?;

    let source = Source {
        text: &source,
        description: &file_name,
        directory: std::path::absolute(file)
            .ok()
            .and_then(|absolute| absolute.parent().map(Path::to_path_buf))
            .unwrap_or_default(),
        environment: environment_of(file, &file_name),
    };
    let mut namer = OutputNamer::new(&file_name, options.format.suffix());
    for block in rockuml::preproc::preprocess(&source, &SystemHost) {
        let output = output_directory.join(namer.next_name(block.output_name().as_deref()));
        let content = match options.format {
            OutputFormat::Preprocessed => block
                .lines()
                .flat_map(|line| [line, LINE_SEPARATOR])
                .collect::<String>(),
            format => return Err(format!("{format:?} output is not implemented yet")),
        };
        fs::write(&output, content)
            .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    }
    Ok(())
}

/// What `%filename()` and `%filedate()` report. `%dirpath()` stays empty: PlantUML only reveals the
/// directory under its INSECURE security profile.
fn environment_of(file: &Path, file_name: &str) -> PreprocessorEnvironment {
    let modified = fs::metadata(file).and_then(|metadata| metadata.modified()).ok();
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

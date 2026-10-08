//! What a command line does, in PlantUML's order (`Run.main`): parse it, answer immediate requests such as
//! `--help`, then read standard input (`-pipe`) or the input files.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, mpsc};
use std::time::Instant;

use regex::Regex;
use rockuml::fonts::FontRegistry;
use rockuml::host::Host;

use crate::charset::Charset;
use crate::cli_flag::{CliFlag, Support};
use crate::cli_options::{CliOptions, DEFAULT_CONFIG_VARIABLE};
use crate::cli_parsed::CliParsed;
use crate::console::Console;
use crate::exit_status::{self, ExitStatus};
use crate::file_format::FileFormat;
use crate::source_file_reader::{self, SourceFileReader};
use crate::system_host::SystemHost;
use crate::{file_group, fonts, help_print, pipe};

/// Deeply nested diagrams recurse deeply; a thread's stack is only 1 MiB on Windows.
pub(crate) const STACK_SIZE: usize = 64 * 1024 * 1024;

/// What every input is processed with.
pub(crate) struct Settings {
    pub(crate) options: CliOptions,
    pub(crate) format: FileFormat,
    pub(crate) charset: Charset,
    pub(crate) fonts: Arc<FontRegistry>,
    pub(crate) host: SystemHost,
}

/// Why a command line cannot run.
enum Failure {
    /// PlantUML's `CliParsingException`.
    Parsing(String),
    /// A flag, format or value rockuml does not offer.
    Usage(String),
}

/// Runs the command line and returns the exit status.
pub(crate) fn main(arguments: Vec<String>) -> u8 {
    let start = Instant::now();
    let mut console = Console::default();
    let status = match run(arguments, &mut console, start) {
        Ok(status) => status,
        Err(Failure::Parsing(message)) => {
            console.error(&message);
            exit_status::CLI_PARSING_ERROR
        }
        Err(Failure::Usage(message)) => {
            console.error(&format!("rockuml: {message}"));
            exit_status::NOT_PORTED
        }
    };
    console.flush();
    status
}

fn run(arguments: Vec<String>, console: &mut Console, start: Instant) -> Result<u8, Failure> {
    if arguments.is_empty() {
        console.print(&help_print::help());
        return Ok(exit_status::OK);
    }
    let flags = CliParsed::parse(arguments).map_err(|error| Failure::Parsing(error.0))?;
    reject_unavailable_flags(&flags)?;
    let default_config = std::env::var(DEFAULT_CONFIG_VARIABLE)
        .ok()
        .filter(|name| !name.is_empty());
    let options = CliOptions::new(flags, default_config.as_deref())
        .map_err(|error| Failure::Parsing(error.0))?;

    if let Some(text) = immediate_action(&options) {
        console.print(&text);
        return Ok(exit_status::OK);
    }
    if options.is_true(CliFlag::EncodeSprite) {
        encode_sprite(options.remaining_args(), console)?;
        return Ok(exit_status::OK);
    }
    let settings = Settings::new(options)?;
    let options = &settings.options;
    if options.is_true(CliFlag::Pipe) {
        let status = ExitStatus::default();
        console.flush();
        pipe::manage_pipe(&settings, &status, console);
        print_duration(options, start, console);
        return Ok(status.exit_code());
    }
    if options.is_true(CliFlag::DecodeUrl) {
        decode_urls(options.remaining_args(), console)?;
        return Ok(exit_status::OK);
    }
    if options.is_true(CliFlag::RetrieveMetadata) {
        for file in input_files(options) {
            extract_metadata(&file, console);
        }
        return Ok(exit_status::OK);
    }

    let files = input_files(options);
    if files.is_empty() {
        console.error("No file found");
        return Ok(exit_status::ERROR_50_NO_FILE_FOUND);
    }
    let status = ExitStatus::default();
    for _ in 0..options.loops() {
        if options.is_true(CliFlag::ComputeUrl) {
            compute_url(&files, &settings, console);
            print_duration(options, start, console);
            return Ok(exit_status::OK);
        } else if options.is_true(CliFlag::FailFast2) && check_error(&files, &settings, &status) {
            // The inputs have errors, so none is processed.
        } else {
            console.flush();
            process_inputs_in_parallel(&files, &settings, &status);
        }
    }
    print_duration(options, start, console);
    if status.has_errors() {
        console.error("Some diagram description contains errors");
    }
    if status.no_diagram_found() {
        console.error("No diagram found");
    }
    Ok(status.exit_code())
}

fn reject_unavailable_flags(flags: &CliParsed) -> Result<(), Failure> {
    for flag in flags.flags() {
        match flag.support() {
            Support::Ported | Support::Ignored => {}
            Support::NotPorted => {
                return Err(Failure::Usage(format!("{} is not ported yet", flag.flag())));
            }
            Support::Dropped => {
                return Err(Failure::Usage(format!(
                    "{} is a PlantUML option rockuml leaves out",
                    flag.flag()
                )));
            }
        }
    }
    if let Some(option) = flags
        .remaining_args()
        .iter()
        .find(|argument| is_unknown_option(argument))
    {
        return Err(Failure::Usage(format!("unknown option: {option}")));
    }
    Ok(())
}

/// PlantUML takes arguments it does not know for files, so mistyped options go unnoticed; rockuml rejects
/// them unless such a file exists.
fn is_unknown_option(argument: &str) -> bool {
    argument.len() > 1 && argument.starts_with('-') && !Path::new(argument).exists()
}

/// `--help`, `--version` and the like: the first in PlantUML's order that is given once.
fn immediate_action(options: &CliOptions) -> Option<String> {
    let given = |flag| options.flags.count(flag) == 1;
    [
        (CliFlag::Help, help_print::help as fn() -> String),
        (CliFlag::HelpMore, help_print::help_more),
        (CliFlag::Version, help_print::version),
        (CliFlag::Author, help_print::author),
        (CliFlag::License, help_print::license),
    ]
    .into_iter()
    .find(|&(flag, _)| given(flag))
    .map(|(_, text)| text())
}

impl Settings {
    fn new(options: CliOptions) -> Result<Self, Failure> {
        let format = options.file_format().map_err(Failure::Usage)?;
        let charset = options.charset().map_err(Failure::Usage)?;
        let fonts = if format.measures_with_fonts() {
            fonts::load(&options.fonts()).map_err(Failure::Usage)?
        } else {
            FontRegistry::default()
        };
        let host = SystemHost::with_limit_size(options.limit_size());
        Ok(Self {
            options,
            format,
            charset,
            fonts: Arc::new(fonts),
            host,
        })
    }
}

fn input_files(options: &CliOptions) -> Vec<PathBuf> {
    let excludes = options.excludes();
    options
        .remaining_args()
        .iter()
        .flat_map(|pattern| file_group::files(pattern, &excludes))
        .collect()
}

/// Runs `task` on each file with `--threads` threads, printing each file's output in input order.
fn process_in_parallel(
    files: &[PathBuf],
    threads: usize,
    task: &(dyn Fn(&Path, &mut Console) + Sync),
) {
    let next = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..threads.min(files.len()) {
            let sender = sender.clone();
            let next = &next;
            std::thread::Builder::new()
                .stack_size(STACK_SIZE)
                .spawn_scoped(scope, move || {
                    loop {
                        let index = next.fetch_add(1, Ordering::SeqCst);
                        let Some(file) = files.get(index) else {
                            break;
                        };
                        let mut console = Console::default();
                        task(file, &mut console);
                        if sender.send((index, console)).is_err() {
                            break;
                        }
                    }
                })
                .expect("a thread can be started");
        }
        drop(sender);
        let mut finished = BTreeMap::new();
        let mut next_to_print = 0;
        for (index, console) in receiver {
            finished.insert(index, console);
            while let Some(mut console) = finished.remove(&next_to_print) {
                console.flush();
                next_to_print += 1;
            }
        }
    });
}

fn process_inputs_in_parallel(files: &[PathBuf], settings: &Settings, status: &ExitStatus) {
    let options = &settings.options;
    process_in_parallel(files, options.nb_threads(), &|file, console| {
        status.goes_has_files();
        if status.has_errors() && options.is_failfast_or_failfast2() {
            return;
        }
        manage_file_internal(file, settings, status, console);
    });
}

fn manage_file_internal(
    file: &Path,
    settings: &Settings,
    status: &ExitStatus,
    console: &mut Console,
) {
    let mut reader = match SourceFileReader::new(file, settings) {
        Ok(reader) => reader,
        Err(error) => {
            console.error(&format!("rockuml: {error}"));
            return;
        }
    };
    reader.update_status(status);
    if settings.options.is_true(CliFlag::CheckOnly) {
        return;
    }
    if settings.format == FileFormat::Preproc {
        reader.extract_preprocessing_source(console);
        return;
    }
    reader.generate_images(status, console);
}

/// `--check-before-run`: whether any input has an error; then none is processed.
fn check_error(files: &[PathBuf], settings: &Settings, status: &ExitStatus) -> bool {
    let has_error = AtomicBool::new(false);
    process_in_parallel(files, settings.options.nb_threads(), &|file, console| {
        if has_error.load(Ordering::SeqCst) {
            return;
        }
        match SourceFileReader::new(file, settings) {
            Ok(reader) => {
                status.goes_has_files();
                if reader.has_error() {
                    has_error.store(true, Ordering::SeqCst);
                    status.goes_has_errors();
                }
            }
            Err(error) => console.error(&format!("rockuml: {error}")),
        }
    });
    has_error.load(Ordering::SeqCst)
}

/// `-encodeurl`: each diagram's URL code.
fn compute_url(files: &[PathBuf], settings: &Settings, console: &mut Console) {
    for file in files {
        match source_file_reader::preprocess_file(file, settings) {
            Ok(blocks) => {
                for block in &blocks {
                    console.println(&rockuml::diagram::encoded_url(block));
                }
            }
            Err(error) => console.error(&format!("rockuml: {error}")),
        }
    }
}

/// PlantUML wraps the decoded source, which already has its own start and end lines, in another pair.
fn decode_urls(codes: &[String], console: &mut Console) -> Result<(), Failure> {
    for code in codes {
        let source = rockuml::url_code::decode(code)
            .map_err(|_| Failure::Usage(format!("not a PlantUML code: {code}")))?;
        console.println("@startuml");
        console.println(&source);
        console.println("@enduml");
    }
    Ok(())
}

/// `--extract-source`: the source an SVG or PNG image carries, or `null` for an image without one.
fn extract_metadata(file: &Path, console: &mut Console) {
    const SEPARATOR: &str = "------------------------";
    console.println(SEPARATOR);
    console.println(&file.display().to_string());
    console.println("");
    let content = fs::read(file).unwrap_or_default();
    if file.to_string_lossy().ends_with(".svg") {
        if let Some(source) = rockuml::metadata::from_svg(&String::from_utf8_lossy(&content)) {
            console.println(&source);
        }
    } else {
        console.println(
            rockuml::metadata::from_png(&content)
                .as_deref()
                .unwrap_or("null"),
        );
    }
    console.println(SEPARATOR);
}

/// `--sprite [4|8|16] <image>`: the sprite definition drawing the image in that many grays.
fn encode_sprite(arguments: &[String], console: &mut Console) -> Result<(), Failure> {
    static LEVEL: LazyLock<Regex> = LazyLock::new(|| Regex::new("^(4|8|16)z?$").unwrap());
    let (level, path) = match arguments {
        [level, path, ..] if LEVEL.is_match(level) => (level.as_str(), path),
        [path, ..] => ("16", path),
        [] => {
            return Err(Failure::Usage(
                "--encode-sprite needs an image file".to_owned(),
            ));
        }
    };
    if level.ends_with('z') {
        return Err(Failure::Usage(
            "compressed sprites (4z, 8z, 16z) are not ported yet".to_owned(),
        ));
    }
    let lowercase = path.to_lowercase();
    let (file_name, data) = if lowercase.starts_with("http://") || lowercase.starts_with("https://")
    {
        let file_name = path.rsplit('/').next().unwrap_or_default().to_owned();
        (file_name, SystemHost::default().read_url(path))
    } else {
        (
            source_file_reader::file_name(Path::new(path)),
            fs::read(path).ok(),
        )
    };
    let data = data.ok_or_else(|| Failure::Usage(format!("cannot read {path}")))?;
    let grays = level.parse().expect("the level is a number");
    let sprite = rockuml::sprite::encode(&data, &sprite_name(&file_name), grays)
        .ok_or_else(|| Failure::Usage(format!("{path} is not an image rockuml reads")))?;
    console.println(&sprite);
    Ok(())
}

/// The letters, digits and underscores the image's file name starts with, or `test`.
fn sprite_name(file_name: &str) -> String {
    let name: String = file_name
        .chars()
        .take_while(|&c| c.is_alphabetic() || c.is_ascii_digit() || c == '_')
        .collect();
    if name.is_empty() {
        "test".to_owned()
    } else {
        name
    }
}

fn print_duration(options: &CliOptions, start: Instant, console: &mut Console) {
    if options.is_true(CliFlag::Duration) {
        let seconds = start.elapsed().as_millis() as f64 / 1000.0;
        let mut text = seconds.to_string();
        if !text.contains('.') {
            text.push_str(".0");
        }
        console.error(&format!("Duration = {text} seconds"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprites_are_named_after_their_file() {
        assert_eq!(sprite_name("my_icon2.png"), "my_icon2");
        assert_eq!(sprite_name("caf\u{e9}-1.png"), "caf\u{e9}");
        assert_eq!(sprite_name(".png"), "test");
    }

    #[test]
    fn options_that_are_no_files_are_unknown() {
        assert!(is_unknown_option("--bogus"));
        assert!(!is_unknown_option("-"));
        assert!(!is_unknown_option("diagram.puml"));
    }
}

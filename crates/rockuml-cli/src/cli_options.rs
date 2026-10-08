//! The settings a command line makes (PlantUML's `CliOptions`), including the configuration lines it adds to
//! every diagram.

use std::fs;
use std::path::{Path, PathBuf};

use rockuml::metadata::Metadata;

use crate::charset::Charset;
use crate::cli_flag::CliFlag;
use crate::cli_parsed::{CliParsed, CliParsingException};
use crate::file_format::FileFormat;
use crate::file_group;

/// Names a configuration file read before any other (`PLANTUML_DEFAULT_CONFIG_FILENAME`).
pub(crate) const DEFAULT_CONFIG_VARIABLE: &str = "PLANTUML_DEFAULT_CONFIG_FILENAME";

/// A define that sets PlantUML's image size limit instead of a preprocessor variable.
pub(crate) const LIMIT_SIZE: &str = "PLANTUML_LIMIT_SIZE";

pub(crate) struct CliOptions {
    pub(crate) flags: CliParsed,
    config: Vec<String>,
}

impl CliOptions {
    /// Reads the configuration files the command line names; `default_config` is the file
    /// `PLANTUML_DEFAULT_CONFIG_FILENAME` names.
    pub(crate) fn new(
        flags: CliParsed,
        default_config: Option<&str>,
    ) -> Result<Self, CliParsingException> {
        let mut config = Vec::new();
        if let Some(file_name) = default_config {
            init_include(&mut config, file_name)?;
        }
        for (file_name, _) in flags.get_map(CliFlag::Include, CliFlag::IncludeLong) {
            init_include(&mut config, &file_name)?;
        }
        for theme in flags.get_list(CliFlag::Theme) {
            config.push(format!("!theme {theme}"));
        }
        for file_name in flags.get_list(CliFlag::Config) {
            add_file_in_config(&mut config, Path::new(file_name))?;
        }
        for (key, value) in flags.get_map(CliFlag::Pragma, CliFlag::PragmaLong) {
            config.push(format!("!pragma {key} {}", java_string(value.as_deref())));
        }
        for (key, value) in flags.get_map(CliFlag::Skinparam, CliFlag::SkinparamLong) {
            config.push(format!(
                "skinparamlocked {key} {}",
                java_string(value.as_deref())
            ));
        }
        Ok(Self { flags, config })
    }

    pub(crate) fn is_true(&self, flag: CliFlag) -> bool {
        self.flags.is_true(flag)
    }

    pub(crate) fn get_string(&self, flag: CliFlag) -> Option<&str> {
        self.flags.get_string(flag)
    }

    pub(crate) fn remaining_args(&self) -> &[String] {
        self.flags.remaining_args()
    }

    /// Lines added after every `@start` line.
    pub(crate) fn config(&self) -> &[String] {
        &self.config
    }

    /// `-D` definitions, a definition without value being empty.
    pub(crate) fn defines(&self) -> Vec<(String, String)> {
        self.flags
            .get_map(CliFlag::Define, CliFlag::DefineLong)
            .into_iter()
            .map(|(key, value)| (key, value.unwrap_or_default()))
            .collect()
    }

    /// `-DPLANTUML_LIMIT_SIZE=...`, which PlantUML turns into a system property.
    pub(crate) fn limit_size(&self) -> Option<String> {
        self.defines()
            .into_iter()
            .find(|(key, _)| key == LIMIT_SIZE)
            .map(|(_, value)| value)
    }

    /// `-f` wins over flags such as `-tsvg`; of those, PlantUML takes the first in its order that is given
    /// once. PNG is the default.
    pub(crate) fn file_format(&self) -> Result<FileFormat, String> {
        if let Some(name) = self.get_string(CliFlag::Format) {
            return FileFormat::from_cli(name);
        }
        Ok(self
            .flags
            .flags()
            .filter(|&flag| self.flags.count(flag) == 1)
            .find_map(FileFormat::of_flag)
            .unwrap_or(FileFormat::Png))
    }

    pub(crate) fn charset(&self) -> Result<Charset, String> {
        Charset::for_name(self.get_string(CliFlag::Charset).unwrap_or("UTF-8"))
    }

    pub(crate) fn metadata(&self) -> Metadata {
        if self.is_true(CliFlag::NoMetadata) {
            Metadata::Omitted
        } else {
            Metadata::Embedded
        }
    }

    pub(crate) fn output_dir(&self) -> Option<&str> {
        self.get_string(CliFlag::OutputDir)
            .map(eventually_remove_starting_and_ending_double_quote)
    }

    pub(crate) fn excludes(&self) -> Vec<String> {
        self.flags
            .get_list(CliFlag::Exclude)
            .map(str::to_owned)
            .collect()
    }

    /// `auto`, a number that is not one, or none at all mean one per processor.
    pub(crate) fn nb_threads(&self) -> usize {
        self.get_string(CliFlag::NbThread)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
            .max(1)
    }

    pub(crate) fn loops(&self) -> usize {
        self.get_string(CliFlag::Loop)
            .and_then(|value| value.parse().ok())
            .unwrap_or(1)
    }

    pub(crate) fn image_index(&self) -> usize {
        self.get_string(CliFlag::PipeImageIndex)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    pub(crate) fn is_failfast_or_failfast2(&self) -> bool {
        self.is_true(CliFlag::FailFast) || self.is_true(CliFlag::FailFast2)
    }

    /// `--http-server[:port[:address]][:stop]`: the port, 8080 unless given (`getPicowebPort`).
    pub(crate) fn picoweb_port(&self) -> Result<u16, String> {
        match self.flags.get_list(CliFlag::Picoweb).next() {
            None => Ok(8080),
            Some(port) => port
                .parse()
                .map_err(|_| format!("--http-server: not a port: {port}")),
        }
    }

    /// The address to listen on, every interface unless given (`getPicowebBindAddress`).
    pub(crate) fn picoweb_bind_address(&self) -> Option<&str> {
        self.flags.get_list(CliFlag::Picoweb).nth(1)
    }

    /// Whether `/stopserver` stops the server (`getPicowebEnableStop`).
    pub(crate) fn picoweb_enable_stop(&self) -> bool {
        self.flags
            .get_list(CliFlag::Picoweb)
            .any(|part| part.eq_ignore_ascii_case("stop"))
    }

    pub(crate) fn fonts(&self) -> Vec<PathBuf> {
        self.flags
            .get_list(CliFlag::Font)
            .map(PathBuf::from)
            .collect()
    }
}

/// A `-I` file, or every file of a `-I` pattern with `*`.
fn init_include(config: &mut Vec<String>, file_name: &str) -> Result<(), CliParsingException> {
    if file_name.contains('*') {
        for file in file_group::files(file_name, &[]) {
            add_file_in_config(config, &file)?;
        }
        Ok(())
    } else {
        add_file_in_config(config, Path::new(file_name))
    }
}

fn add_file_in_config(config: &mut Vec<String>, file: &Path) -> Result<(), CliParsingException> {
    let absolute = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    if file.is_dir() {
        return Err(CliParsingException(format!(
            "Cannot have a directory here {}",
            absolute.display()
        )));
    }
    let text = fs::read(file)
        .map_err(|_| CliParsingException(format!("Cannot read {}", absolute.display())))?;
    config.extend(String::from_utf8_lossy(&text).lines().map(str::to_owned));
    Ok(())
}

/// How Java's string concatenation writes a missing value.
fn java_string(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

fn eventually_remove_starting_and_ending_double_quote(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(words: &[&str]) -> CliOptions {
        let flags = CliParsed::parse(words.iter().map(|&word| word.to_owned())).unwrap();
        CliOptions::new(flags, None).unwrap()
    }

    #[test]
    fn themes_pragmas_and_skin_parameters_become_configuration_lines() {
        let options = options(&[
            "--theme",
            "plain",
            "-Pteoz=true",
            "-SArrowColor=red",
            "--skinparam",
            "a=b",
        ]);
        assert_eq!(
            options.config(),
            [
                "!theme plain",
                "!pragma teoz true",
                "skinparamlocked ArrowColor red",
                "skinparamlocked a b"
            ]
        );
    }

    #[test]
    fn configuration_files_are_read_line_by_line() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("c.cfg");
        fs::write(&file, "skinparam a b\r\nskinparam c d\n").unwrap();
        let options = options(&["--config", file.to_str().unwrap()]);
        assert_eq!(options.config(), ["skinparam a b", "skinparam c d"]);
        let flags = CliParsed::parse(["--config".to_owned(), "missing.cfg".to_owned()]).unwrap();
        let error = CliOptions::new(flags, None).err().unwrap();
        assert!(error.0.starts_with("Cannot read "), "{}", error.0);
    }

    #[test]
    fn the_format_flag_wins_and_png_is_the_default() {
        assert_eq!(options(&[]).file_format(), Ok(FileFormat::Png));
        assert_eq!(options(&["-tsvg"]).file_format(), Ok(FileFormat::Svg));
        assert_eq!(
            options(&["-tsvg", "-f", "debug"]).file_format(),
            Ok(FileFormat::Debug)
        );
        // PlantUML takes the first format flag in its own order.
        assert_eq!(
            options(&["-tsvg", "-tpng"]).file_format(),
            Ok(FileFormat::Png)
        );
    }

    #[test]
    fn thread_counts_default_to_the_processors() {
        assert_eq!(options(&["--threads", "3"]).nb_threads(), 3);
        assert!(options(&["--threads", "auto"]).nb_threads() >= 1);
        assert_eq!(options(&["--threads", "0"]).nb_threads(), 1);
    }
}

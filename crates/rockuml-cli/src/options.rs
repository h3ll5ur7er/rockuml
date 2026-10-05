//! Command-line parsing, accepting PlantUML's flag spellings.

use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Command {
    Version,
    Render(RenderOptions),
}

#[derive(Debug, PartialEq)]
pub struct RenderOptions {
    pub format: OutputFormat,
    pub output_directory: Option<PathBuf>,
    pub files: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputFormat {
    Preprocessed,
    Debug,
    Svg,
}

impl OutputFormat {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "preproc" => Some(Self::Preprocessed),
            "debug" => Some(Self::Debug),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Preprocessed => ".preproc",
            Self::Debug => ".debug",
            Self::Svg => ".svg",
        }
    }
}

pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut format = OutputFormat::Svg;
    let mut output_directory = None;
    let mut files = Vec::new();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--version" => return Ok(Command::Version),
            "-preproc" | "--preproc" => format = OutputFormat::Preprocessed,
            "-tsvg" | "-svg" | "--svg" => format = OutputFormat::Svg,
            "-f" | "--format" => {
                let name = arguments.next().ok_or("missing format name after -f")?;
                format = OutputFormat::from_name(&name)
                    .ok_or_else(|| format!("unsupported format: {name}"))?;
            }
            "-o" | "--output-dir" => {
                output_directory =
                    Some(arguments.next().ok_or("missing directory after -o")?.into());
            }
            flag if flag.starts_with('-') => return Err(format!("unsupported option: {flag}")),
            file => files.push(file.into()),
        }
    }
    Ok(Command::Render(RenderOptions {
        format,
        output_directory,
        files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(words: &str) -> Result<Command, String> {
        parse(words.split_whitespace().map(str::to_owned))
    }

    #[test]
    fn renders_svg_next_to_the_input_by_default() {
        assert_eq!(
            parse_words("a.puml"),
            Ok(Command::Render(RenderOptions {
                format: OutputFormat::Svg,
                output_directory: None,
                files: vec!["a.puml".into()],
            }))
        );
    }

    #[test]
    fn accepts_plantuml_flag_spellings() {
        let Ok(Command::Render(options)) = parse_words("-preproc -o out a.puml") else {
            panic!("expected a render command");
        };
        assert_eq!(options.format, OutputFormat::Preprocessed);
        assert_eq!(options.output_directory, Some("out".into()));
    }

    #[test]
    fn rejects_unknown_formats_and_flags() {
        assert!(parse_words("-f bogus a.puml").is_err());
        assert!(parse_words("-bogus a.puml").is_err());
    }
}

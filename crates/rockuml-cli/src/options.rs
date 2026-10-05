//! Command-line parsing, accepting PlantUML's flag spellings.

use std::path::PathBuf;

use rockuml::diagram::ImageFormat;

#[derive(Debug, PartialEq)]
pub enum Command {
    Version,
    Render(RenderOptions),
    /// Prints the sources encoded in these codes.
    DecodeUrl(Vec<String>),
}

#[derive(Debug, PartialEq)]
pub struct RenderOptions {
    pub format: OutputFormat,
    pub output_directory: Option<PathBuf>,
    /// Font files, or directories of them, to measure text with besides the embedded fonts.
    pub fonts: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputFormat {
    Preprocessed,
    Debug,
    Svg,
    Png,
    /// SVG with text measured by a fixed width table instead of fonts, identical on every machine.
    DeterministicSvg,
    /// Prints each diagram's URL code instead of writing a file.
    EncodedUrl,
}

impl OutputFormat {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "preproc" => Some(Self::Preprocessed),
            "debug" => Some(Self::Debug),
            "svg" => Some(Self::Svg),
            "png" => Some(Self::Png),
            "svg-deterministic" => Some(Self::DeterministicSvg),
            _ => None,
        }
    }

    /// The image format the engine exports this output in; `None` for outputs that are not images or not
    /// ported yet.
    pub fn image_format(self) -> Option<ImageFormat> {
        match self {
            Self::Debug => Some(ImageFormat::Debug),
            Self::Svg => Some(ImageFormat::Svg),
            Self::Png => Some(ImageFormat::Png),
            Self::DeterministicSvg => Some(ImageFormat::DeterministicSvg),
            Self::Preprocessed | Self::EncodedUrl => None,
        }
    }

    /// Whether text is measured with fonts, which `--font` adds to.
    pub fn measures_with_fonts(self) -> bool {
        matches!(self, Self::Svg | Self::Png)
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Preprocessed => ".preproc",
            Self::Debug => ".debug",
            Self::Svg | Self::DeterministicSvg => ".svg",
            Self::Png => ".png",
            Self::EncodedUrl => "",
        }
    }
}

pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut format = OutputFormat::Svg;
    let mut output_directory = None;
    let mut fonts = Vec::new();
    let mut files = Vec::new();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--version" => return Ok(Command::Version),
            "-preproc" | "--preproc" => format = OutputFormat::Preprocessed,
            "-encodeurl" | "--encode-url" | "-computeurl" | "--compute-url" => {
                format = OutputFormat::EncodedUrl;
            }
            "-decodeurl" | "--decode-url" => return Ok(Command::DecodeUrl(arguments.collect())),
            "-tsvg" | "-svg" | "--svg" => format = OutputFormat::Svg,
            "-tpng" | "-png" | "--png" => format = OutputFormat::Png,
            "-f" | "--format" => {
                let name = arguments.next().ok_or("missing format name after -f")?;
                format = OutputFormat::from_name(&name)
                    .ok_or_else(|| format!("unsupported format: {name}"))?;
            }
            "-o" | "--output-dir" => {
                output_directory =
                    Some(arguments.next().ok_or("missing directory after -o")?.into());
            }
            "-font" | "--font" => {
                fonts.push(arguments.next().ok_or("missing path after --font")?.into());
            }
            flag if flag.starts_with('-') => return Err(format!("unsupported option: {flag}")),
            file => files.push(file.into()),
        }
    }
    Ok(Command::Render(RenderOptions {
        format,
        output_directory,
        fonts,
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
                fonts: Vec::new(),
                files: vec!["a.puml".into()],
            }))
        );
    }

    #[test]
    fn accepts_plantuml_flag_spellings() {
        let Ok(Command::Render(options)) = parse_words("-preproc -o out --font f.ttf a.puml")
        else {
            panic!("expected a render command");
        };
        assert_eq!(options.format, OutputFormat::Preprocessed);
        assert_eq!(options.output_directory, Some("out".into()));
        assert_eq!(options.fonts, [PathBuf::from("f.ttf")]);
    }

    #[test]
    fn rejects_unknown_formats_and_flags() {
        assert!(parse_words("-f bogus a.puml").is_err());
        assert!(parse_words("-bogus a.puml").is_err());
    }
}

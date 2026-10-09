//! The output formats rockuml writes (PlantUML's `FileFormat`), and the output file names they give.

use std::sync::LazyLock;

use regex::Regex;
use rockuml::diagram::ImageFormat;

use crate::cli_flag::CliFlag;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileFormat {
    Png,
    Svg,
    /// SVG with text measured by a fixed width table instead of fonts, identical on every machine (PlantUML's
    /// `SVG_DETERMINISTIC`).
    DeterministicSvg,
    /// PlantUML's list of every drawn shape.
    Debug,
    /// The source after preprocessing.
    Preproc,
    /// Lays the diagrams out and writes empty files.
    Null,
}

/// The `-f` names of PlantUML's other formats.
const NOT_PORTED: [&str; 21] = [
    "eps",
    "eps-text",
    "eps-no-preamble",
    "txt",
    "utxt",
    "xmi",
    "xmi-star",
    "xmi-argo",
    "xmi-custom",
    "xmi-script",
    "scxml",
    "graphml",
    "pdf",
    "html",
    "html5",
    "vdx",
    "base64",
    "braille-png",
    "obfuscate",
    "png-empty",
    "raw",
];

/// PlantUML numbers the files of a source's later images: `name_001.svg`.
const FILE_SEPARATOR: &str = "_";

impl FileFormat {
    /// The format `-f` names.
    pub(crate) fn from_cli(name: &str) -> Result<Self, String> {
        Ok(match name {
            "png" => Self::Png,
            "svg" => Self::Svg,
            "svg-deterministic" => Self::DeterministicSvg,
            "debug" => Self::Debug,
            "preproc" => Self::Preproc,
            "null" => Self::Null,
            _ if NOT_PORTED.contains(&name) => {
                return Err(format!("{name} output is not ported yet"));
            }
            _ => return Err(format!("unknown output format: {name}")),
        })
    }

    /// The format a flag such as `-tsvg` selects.
    pub(crate) fn of_flag(flag: CliFlag) -> Option<Self> {
        match flag {
            CliFlag::TPng => Some(Self::Png),
            CliFlag::TSvg => Some(Self::Svg),
            CliFlag::Preprocess => Some(Self::Preproc),
            CliFlag::TNull => Some(Self::Null),
            _ => None,
        }
    }

    fn file_suffix(self) -> &'static str {
        match self {
            Self::Png => ".png",
            Self::Svg | Self::DeterministicSvg => ".svg",
            Self::Debug => ".debug",
            Self::Preproc => ".preproc",
            Self::Null => ".null",
        }
    }

    /// The name of a source's image number `cpt`: its extension replaced by the format's, the images after
    /// the first numbered (`changeName`).
    pub(crate) fn change_name(self, file_name: &str, cpt: usize) -> String {
        static EXTENSION: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\.[a-zA-Z0-9_]+$").unwrap());
        let replacement = if cpt == 0 {
            self.file_suffix().to_owned()
        } else {
            format!("{FILE_SEPARATOR}{cpt:03}{}", self.file_suffix())
        };
        let result = EXTENSION.replace(file_name, replacement.as_str());
        if result == file_name {
            format!("{file_name}{replacement}")
        } else {
            result.into_owned()
        }
    }

    /// The image format the engine exports, for formats that draw.
    /// The content type of the format's output (`getMimeType`).
    pub(crate) fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Svg | Self::DeterministicSvg => "image/svg+xml",
            Self::Debug | Self::Preproc | Self::Null => "text/plain",
        }
    }

    pub(crate) fn image_format(self) -> Option<ImageFormat> {
        match self {
            Self::Png => Some(ImageFormat::Png),
            Self::Svg => Some(ImageFormat::Svg),
            Self::DeterministicSvg | Self::Null => Some(ImageFormat::DeterministicSvg),
            Self::Debug => Some(ImageFormat::Debug),
            Self::Preproc => None,
        }
    }

    /// Whether text is measured with fonts, which `--font` adds to.
    pub(crate) fn measures_with_fonts(self) -> bool {
        matches!(self, Self::Svg | Self::Png)
    }

    /// Whether images carry their source, which `--skip-fresh` compares (`doesSupportMetadata`).
    pub(crate) fn supports_metadata(self) -> bool {
        matches!(self, Self::Png | Self::Svg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_images_are_numbered() {
        assert_eq!(FileFormat::Svg.change_name("flow.puml", 0), "flow.svg");
        assert_eq!(FileFormat::Svg.change_name("flow.puml", 2), "flow_002.svg");
        assert_eq!(FileFormat::Png.change_name("README", 0), "README.png");
        assert_eq!(
            FileFormat::Preproc.change_name("a.b.txt", 1),
            "a.b_001.preproc"
        );
    }

    #[test]
    fn other_plantuml_formats_are_not_ported() {
        assert_eq!(FileFormat::from_cli("svg"), Ok(FileFormat::Svg));
        assert_eq!(
            FileFormat::from_cli("pdf"),
            Err("pdf output is not ported yet".to_owned())
        );
        assert!(FileFormat::from_cli("bogus").is_err());
    }
}

//! rockuml for WebAssembly. `abi` exports the functions `web/rockuml.js` calls; `render` is what they do.

#[cfg(target_family = "wasm")]
pub mod abi;
#[cfg(target_family = "wasm")]
mod wasm_host;

use std::sync::Arc;

use rockuml::diagram::{Diagram, ImageFormat, NotYetPorted};
use rockuml::fonts::FontRegistry;
use rockuml::host::Host;
use rockuml::preproc::{PreprocessedBlock, PreprocessorEnvironment, Source};

/// What a source is rendered to. The numbers are the ABI's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Svg = 0,
    Png = 1,
    DeterministicSvg = 2,
    Debug = 3,
    /// The source after preprocessing, one page per diagram.
    Preprocessed = 4,
}

impl Format {
    pub fn from_code(code: u32) -> Option<Self> {
        [
            Self::Svg,
            Self::Png,
            Self::DeterministicSvg,
            Self::Debug,
            Self::Preprocessed,
        ]
        .into_iter()
        .find(|format| *format as u32 == code)
    }

    fn image_format(self) -> Option<ImageFormat> {
        match self {
            Self::Svg => Some(ImageFormat::Svg),
            Self::Png => Some(ImageFormat::Png),
            Self::DeterministicSvg => Some(ImageFormat::DeterministicSvg),
            Self::Debug => Some(ImageFormat::Debug),
            Self::Preprocessed => None,
        }
    }
}

/// How a rendering went. The numbers are the ABI's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Image = 0,
    /// The image shows the diagram's errors instead of the diagram.
    ErrorImage = 1,
    /// The diagram needs a part of PlantUML rockuml does not have yet; the data is the message.
    NotPorted = 2,
    /// There is no diagram, page or format of that number; the data is the message.
    NoImage = 3,
}

pub struct Rendering {
    pub status: Status,
    pub data: Vec<u8>,
    /// The pages of all the source's diagrams together.
    pub page_count: usize,
}

/// Like Java's `SourceStringReader`, which names sources that are no file `string`.
const SOURCE_DESCRIPTION: &str = "string";

type Created = Result<Box<dyn Diagram>, NotYetPorted>;

/// One page of the source: the pages of its diagrams follow each other, as the files the CLI writes do.
pub fn render(
    source: &str,
    format: Format,
    page: usize,
    host: &dyn Host,
    fonts: &Arc<FontRegistry>,
) -> Rendering {
    let source = Source {
        text: source,
        description: SOURCE_DESCRIPTION,
        directory: std::path::PathBuf::new(),
        environment: PreprocessorEnvironment::default(),
    };
    let blocks = rockuml::preproc::preprocess(&source, host);
    let Some(image_format) = format.image_format() else {
        return preprocessed(&blocks, page);
    };
    let diagrams: Vec<Created> = blocks
        .iter()
        .map(|block| rockuml::diagram::create(block, host))
        .collect();
    // A diagram rockuml cannot create takes one page, which reports that.
    let pages: Vec<(&Created, usize)> = diagrams
        .iter()
        .flat_map(|diagram| {
            let count = diagram.as_ref().map_or(1, |diagram| diagram.page_count());
            (0..count).map(move |page| (diagram, page))
        })
        .collect();
    let page_count = pages.len();
    let Some(&(diagram, page_in_diagram)) = pages.get(page) else {
        return no_such_page(page, page_count);
    };
    let exported = diagram.as_ref().map_err(Clone::clone).and_then(|diagram| {
        rockuml::diagram::export(diagram.as_ref(), page_in_diagram, image_format, fonts, host)
            .map(|data| (data, diagram.is_error()))
    });
    match exported {
        Ok((data, is_error)) => Rendering {
            status: if is_error {
                Status::ErrorImage
            } else {
                Status::Image
            },
            data,
            page_count,
        },
        Err(not_ported) => Rendering {
            status: Status::NotPorted,
            data: not_ported.to_string().into_bytes(),
            page_count,
        },
    }
}

fn preprocessed(blocks: &[PreprocessedBlock], page: usize) -> Rendering {
    let Some(block) = blocks.get(page) else {
        return no_such_page(page, blocks.len());
    };
    Rendering {
        status: Status::Image,
        data: block
            .lines()
            .flat_map(|line| [line, "\n"])
            .collect::<String>()
            .into_bytes(),
        page_count: blocks.len(),
    }
}

fn no_such_page(page: usize, page_count: usize) -> Rendering {
    let message = if page_count == 0 {
        "no @startuml diagram found".to_owned()
    } else {
        format!("there is no page {page}: the source has {page_count}")
    };
    Rendering {
        status: Status::NoImage,
        data: message.into_bytes(),
        page_count,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use rockuml::host::IsolatedHost;

    use super::*;

    fn render_text(source: &str, format: Format, page: usize) -> (Status, String, usize) {
        static FONTS: LazyLock<Arc<FontRegistry>> = LazyLock::new(Arc::default);
        let rendering = render(source, format, page, &IsolatedHost, &FONTS);
        (
            rendering.status,
            String::from_utf8(rendering.data).unwrap(),
            rendering.page_count,
        )
    }

    #[test]
    fn format_codes_round_trip() {
        for code in 0..5 {
            assert_eq!(
                Format::from_code(code).map(|format| format as u32),
                Some(code)
            );
        }
        assert_eq!(Format::from_code(5), None);
    }

    #[test]
    fn renders_svg() {
        let (status, svg, page_count) = render_text("@startuml\nA -> B\n@enduml", Format::Svg, 0);
        assert_eq!((status, page_count), (Status::Image, 1));
        assert!(svg.starts_with("<svg"), "{svg}");
    }

    #[test]
    fn pages_of_all_diagrams_follow_each_other() {
        let source = "@startuml\nA -> B\nnewpage\nB -> C\n@enduml\n@startuml\nC -> D\n@enduml";
        let (status, debug, page_count) = render_text(source, Format::Debug, 2);
        assert_eq!((status, page_count), (Status::Image, 3));
        assert!(debug.contains("text: D"), "{debug}");
    }

    #[test]
    fn reports_error_images() {
        let (status, _, _) = render_text("@startuml\nA -> B\n!error\n@enduml", Format::Svg, 0);
        assert_eq!(status, Status::ErrorImage);
    }

    #[test]
    fn diagrams_that_are_not_ported_say_so() {
        let (status, message, page_count) = render_text(
            "@startditaa\n+--+\n|A |\n+--+\n@endditaa",
            Format::Svg,
            0,
        );
        assert_eq!((status, page_count), (Status::NotPorted, 1));
        assert_eq!(message, "this diagram type is not ported yet");
    }

    #[test]
    fn missing_pages_are_reported() {
        assert_eq!(
            render_text("no diagram", Format::Svg, 0),
            (Status::NoImage, "no @startuml diagram found".to_owned(), 0)
        );
        let (status, message, _) = render_text("@startuml\nA -> B\n@enduml", Format::Svg, 1);
        assert_eq!(status, Status::NoImage);
        assert_eq!(message, "there is no page 1: the source has 1");
    }

    #[test]
    fn preprocessed_text_has_a_page_per_diagram() {
        let source = "@startuml\n!$x = 1\nA -> B : $x\n@enduml\n@startuml\nB -> C\n@enduml";
        assert_eq!(
            render_text(source, Format::Preprocessed, 0),
            (
                Status::Image,
                "@startuml\nA -> B : 1\n@enduml\n".to_owned(),
                2
            )
        );
    }
}

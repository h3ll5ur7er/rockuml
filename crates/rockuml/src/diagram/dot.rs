//! `@startdot`: a Graphviz source, which the host's `dot` turns into the image (PlantUML's
//! `PSystemDotFactory` and `PSystemDot`).

use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::builder::whole_line;
use super::diagram_type::DiagramType;
use super::error::ErrorDiagram;
use super::{Diagram, DirectImage, ExportSettings, ImageFormat, NotYetPorted, UmlSource};
use crate::host::Host;
use crate::java;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::graphic_strings::GraphicStrings;
use crate::preproc::start_utils;
use crate::security_profile::SecurityProfile;

/// What PlantUML writes instead of running `dot` for an SVG under the profiles that do not let it.
const SUPPRESSED_SVG: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">\n",
    "<a xlink:href=\"https://github.com/plantuml/plantuml/issues/2495\">\n",
    "<text x=\"10\" y=\"30\" font-family=\"sans-serif\" font-size=\"14\" fill=\"blue\" ",
    "text-decoration=\"underline\">This feature has been suppressed</text>\n",
    "</a>\n",
    "</svg>"
);

/// Whether `line` opens a Graphviz graph, alone on its line (`PSystemDotFactory.isGraphvizDotHeader`).
pub(super) fn is_graphviz_dot_header(line: &str) -> bool {
    static GRAPHVIZ_DOT_HEADER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
        whole_line(
            r#"\s*(strict\s+)?(di)?graph\s+([_\p{L}][_\p{L}\p{N}]*|-?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)|"([^"\\]|\\")*")?\s*\{\s*"#,
        )
    });
    GRAPHVIZ_DOT_HEADER_PATTERN.is_match(line)
}

/// PlantUML's `PSystemDotFactory`, a `PSystemBasicFactory`: the lines after the start line, from a graph's
/// header on, are the Graphviz source.
pub(super) fn create(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let source = Rc::new(source.without_initial_noise());
    let lines = source.lines();
    let error = |message: &str, read: usize| -> Result<Box<dyn Diagram>, NotYetPorted> {
        Ok(Box::new(ErrorDiagram::new(
            source.clone(),
            lines[..read].to_vec(),
            message,
            Some(DiagramType::Dot),
        )))
    };
    let mut data: Option<String> = None;
    let mut first = true;
    for (position, line) in lines.iter().enumerate().skip(1) {
        let text = line.text();
        if first && java::trim(text).is_empty() {
            continue;
        }
        first = false;
        if start_utils::is_end_directive(text) {
            if lines.len() == 2 {
                return error("Empty description", position + 1);
            }
            break;
        }
        if data.is_none() && !is_graphviz_dot_header(text) {
            return error("Syntax Error?", position + 1);
        }
        let data = data.get_or_insert_default();
        data.push_str(text);
        data.push('\n');
    }
    // PlantUML's factory gives no diagram for blank lines alone, so `PSystemErrorUtils.merge` throws out of
    // `createPSystem`, uncaught: the command line then writes no image for any block of the file and exits 0,
    // and `-pipe` dies with the exception's stack trace.
    let data = data.ok_or(NotYetPorted(
        "the exception PlantUML throws on a @startdot block of blank lines",
    ))?;
    Ok(Box::new(PSystemDot { source, data }))
}

/// PlantUML's `PSystemDot`, a `DirectOsDiagram`.
struct PSystemDot {
    source: Rc<UmlSource>,
    /// The Graphviz source handed to `dot`.
    data: String,
}

impl Diagram for PSystemDot {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    /// Only what `dot` draws: a host that runs no programs cannot draw it.
    fn text_block(
        &self,
        _page: usize,
        _string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Err(NotYetPorted("Graphviz's dot in the browser"))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        ExportSettings::without_skin(self.source.seed())
    }

    fn export_diagram_now(&self, format: ImageFormat, host: &dyn Host) -> Option<DirectImage> {
        if SecurityProfile::init(host).allow_dot_svg() && format == ImageFormat::Svg {
            return Some(DirectImage::Bytes(SUPPRESSED_SVG.as_bytes().to_vec()));
        }
        let graphviz = host.create_for_system_dot(&self.data, file_format_name(format))?;
        if !graphviz.exe_state_is_ok() {
            return Some(error_image(
                "There is an issue with your Dot/Graphviz installation",
            ));
        }
        let output = graphviz_has_format(format)
            .then(|| graphviz.create_file3())
            .flatten();
        Some(match output {
            Some(output) if !output.is_empty() => DirectImage::Bytes(output),
            _ => error_image("GraphViz has crashed"),
        })
    }
}

/// The `-T` format PlantUML asks `dot` for: its own format's name in lower case.
fn file_format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Debug => "debug",
        ImageFormat::Svg => "svg",
        ImageFormat::DeterministicSvg => "svg_deterministic",
        ImageFormat::Png => "png",
    }
}

/// Graphviz has no debug or deterministic SVG format. `dot` rejects them before it reads its input, so
/// PlantUML's write to it fails and it reports a crash, at least for the first diagram of a run; when the
/// write wins the race, the image is `dot`'s complaint instead. rockuml always reports the crash.
fn graphviz_has_format(format: ImageFormat) -> bool {
    matches!(format, ImageFormat::Svg | ImageFormat::Png)
}

fn error_image(message: &'static str) -> DirectImage {
    DirectImage::Drawing(Box::new(GraphicStrings::create_for_error(&[message])))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::host::{Graphviz, IsolatedHost};
    use crate::text::{LineLocation, StringLocated};

    /// A host whose `dot` is installed or not and answers `output`; it records what it was asked.
    struct GraphvizHost {
        installed: bool,
        output: Option<Vec<u8>>,
        security_profile: Option<&'static str>,
        asked: RefCell<Vec<(String, String)>>,
    }

    impl GraphvizHost {
        fn new(installed: bool, output: Option<&[u8]>) -> Self {
            Self {
                installed,
                output: output.map(<[u8]>::to_vec),
                security_profile: None,
                asked: RefCell::default(),
            }
        }
    }

    struct FakeGraphviz {
        installed: bool,
        output: Option<Vec<u8>>,
    }

    impl Graphviz for FakeGraphviz {
        fn exe_state_is_ok(&self) -> bool {
            self.installed
        }

        fn create_file3(&self) -> Option<Vec<u8>> {
            self.output.clone()
        }
    }

    impl Host for GraphvizHost {
        fn read_file(&self, _path: &Path) -> Option<Vec<u8>> {
            None
        }

        fn read_url(&self, _url: &str) -> Option<Vec<u8>> {
            None
        }

        fn file_exists(&self, _path: &Path) -> bool {
            false
        }

        fn current_directory(&self) -> PathBuf {
            PathBuf::new()
        }

        fn home_directory(&self) -> Option<PathBuf> {
            None
        }

        fn getenv(&self, name: &str) -> Option<String> {
            (name == "PLANTUML_SECURITY_PROFILE")
                .then_some(self.security_profile)
                .flatten()
                .map(str::to_owned)
        }

        fn current_time_millis(&self) -> i64 {
            0
        }

        fn local_time_zone(&self) -> Option<String> {
            None
        }

        fn create_for_system_dot(
            &self,
            dot_string: &str,
            format: &str,
        ) -> Option<Box<dyn Graphviz>> {
            self.asked
                .borrow_mut()
                .push((dot_string.to_owned(), format.to_owned()));
            Some(Box::new(FakeGraphviz {
                installed: self.installed,
                output: self.output.clone(),
            }))
        }
    }

    fn diagram(texts: &[&str]) -> Box<dyn Diagram> {
        let location = LineLocation::new("test", None);
        let lines = texts
            .iter()
            .map(|text| StringLocated::new(*text, location.clone()))
            .collect();
        create(UmlSource::new(lines, Vec::new())).unwrap()
    }

    fn graph() -> Box<dyn Diagram> {
        diagram(&["@startdot", "", "digraph G {", "  a -> b;", "}", "@enddot"])
    }

    /// PlantUML's green-on-black message.
    fn assert_error_drawing(image: Option<DirectImage>) {
        let Some(DirectImage::Drawing(drawing)) = image else {
            panic!("no drawing");
        };
        let bounder = crate::klimt::debug::StringBounderDebug;
        assert!(drawing.calculate_dimension(&bounder).width > 0.0);
        assert_eq!(drawing.backcolor(), Some(crate::color::HColor::BLACK));
    }

    #[test]
    fn dot_gets_the_graph_and_the_format_and_its_output_is_the_image() {
        let host = GraphvizHost::new(true, Some(b"<svg/>"));
        let image = graph().export_diagram_now(ImageFormat::Svg, &host);
        assert!(matches!(image, Some(DirectImage::Bytes(bytes)) if bytes == b"<svg/>"));
        assert_eq!(
            host.asked.borrow().as_slice(),
            [("digraph G {\n  a -> b;\n}\n".to_owned(), "svg".to_owned())]
        );
    }

    #[test]
    fn a_missing_dot_and_a_failed_run_draw_an_error() {
        for host in [
            GraphvizHost::new(false, Some(b"<svg/>")),
            GraphvizHost::new(true, None),
            GraphvizHost::new(true, Some(b"")),
        ] {
            assert_error_drawing(graph().export_diagram_now(ImageFormat::Png, &host));
        }
        // Graphviz has no debug format: PlantUML's run of it fails.
        let host = GraphvizHost::new(true, Some(b"Format: \"debug\" not recognized."));
        assert_error_drawing(graph().export_diagram_now(ImageFormat::Debug, &host));
    }

    #[test]
    fn some_security_profiles_suppress_svg() {
        let mut host = GraphvizHost::new(true, Some(b"<svg/>"));
        host.security_profile = Some("INSECURE");
        let image = graph().export_diagram_now(ImageFormat::Svg, &host);
        assert!(
            matches!(image, Some(DirectImage::Bytes(bytes)) if bytes.starts_with(SUPPRESSED_SVG.as_bytes()))
        );
        assert!(host.asked.borrow().is_empty());
    }

    #[test]
    fn hosts_that_run_no_programs_cannot_draw_it() {
        let graph = graph();
        assert!(
            graph
                .export_diagram_now(ImageFormat::Svg, &IsolatedHost)
                .is_none()
        );
        let bounder: Rc<dyn StringBounder> = Rc::new(crate::klimt::debug::StringBounderDebug);
        assert!(graph.text_block(0, &bounder).is_err());
    }

    #[test]
    fn lines_before_the_header_are_errors() {
        assert!(diagram(&["@startdot", "a -> b", "@enddot"]).is_error());
        assert!(diagram(&["@startdot", "@enddot"]).is_error());
        assert!(!graph().is_error());
    }

    #[test]
    fn blank_lines_alone_are_no_diagram() {
        let location = LineLocation::new("test", None);
        let lines = ["@startdot", "  ", "\t", "@enddot"]
            .iter()
            .map(|text| StringLocated::new(*text, location.clone()))
            .collect();
        let Err(not_ported) = create(UmlSource::new(lines, Vec::new())) else {
            panic!("a diagram");
        };
        assert_eq!(
            not_ported.to_string(),
            "the exception PlantUML throws on a @startdot block of blank lines is not ported yet"
        );
    }

    #[test]
    fn graph_headers_stand_alone_on_their_line() {
        for header in [
            "digraph G {",
            "  graph {  ",
            "strict digraph \"a \\\"b\\\"\" {",
            "graph -1.5 {",
            "digraph Ünïcode_1 {",
        ] {
            assert!(is_graphviz_dot_header(header), "{header}");
        }
        for line in [
            "digraph{",
            "digraph { a -> b }",
            "Digraph G {",
            "subgraph x {",
        ] {
            assert!(!is_graphviz_dot_header(line), "{line}");
        }
    }
}

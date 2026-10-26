//! The cards about the program: `version`, `authors`, `stdlib` and `testdot`. rockuml tells its own facts
//! in PlantUML's layout, except for `stdlib`, whose libraries are PlantUML's.

use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::{PlainStringsDiagram, UgDiagram};
use crate::diagram::builder::{PSystemSingleLineFactory, whole_line};
use crate::diagram::{Diagram, NotYetPorted, UmlSource};
use crate::stdlib::{Stdlib, library_names};

/// `version`, `authors`, `stdlib`… (PlantUML's `PSystemVersionFactory`). License keys, which only
/// PlantUML's professional edition has, are recognised and not drawn.
pub(in crate::diagram) struct PSystemVersionFactory;

impl PSystemSingleLineFactory for PSystemVersionFactory {
    fn execute_line(
        source: &Rc<UmlSource>,
        line: &str,
    ) -> Option<Result<Box<dyn Diagram>, NotYetPorted>> {
        static AUTHORS: LazyLock<Regex> = LazyLock::new(|| whole_line(r"(?i)(authors?|about)\s*"));
        static VERSION: LazyLock<Regex> = LazyLock::new(|| whole_line(r"(?i)version\s*"));
        static STDLIB: LazyLock<Regex> = LazyLock::new(|| whole_line(r"(?i)stdlib\s*"));
        static TESTDOT: LazyLock<Regex> = LazyLock::new(|| whole_line(r"(?i)testdot\s*"));
        static KEYS: LazyLock<Regex> = LazyLock::new(|| {
            whole_line(
                r"(?i)(keydistributor|keygen|keyimport(\s+[0-9a-z]+)?|keycheck\s+([0-9a-z]+)\s+([0-9a-z]+))\s*",
            )
        });
        let card = if AUTHORS.is_match(line) {
            create_show_authors()
        } else if VERSION.is_match(line) {
            create_show_version()
        } else if STDLIB.is_match(line) {
            create_std_lib()
        } else if TESTDOT.is_match(line) {
            create_test_dot()
        } else if KEYS.is_match(line) {
            return Some(Err(NotYetPorted("license keys")));
        } else {
            return None;
        };
        Some(Ok(UgDiagram::boxed(source, card)))
    }
}

const ROCKUML_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `version`: what PlantUML's card says that is true of rockuml.
fn create_show_version() -> PlainStringsDiagram {
    let mut strings = vec![
        format!("<b>rockuml</b> \"\"{ROCKUML_VERSION}\"\""),
        format!("<b>PlantUML</b> \"\"{}\"\"", crate::PLANTUML_VERSION),
        "<b>License</b> \"\"LGPL\"\"".to_owned(),
        " ".to_owned(),
    ];
    add_dot_status(&mut strings);
    PlainStringsDiagram::new(strings)
}

/// What rockuml lays graphs out with, where PlantUML tells how its Graphviz installation is
/// (`GraphvizUtils.addDotStatus`).
fn add_dot_status(strings: &mut Vec<String>) {
    strings.extend([
        "Layout: Smetana, ported from PlantUML's port of Graphviz's dot".to_owned(),
        "rockuml lays out every diagram itself: it needs no GraphViz installation.".to_owned(),
    ]);
}

/// `authors` and `about`.
fn create_show_authors() -> PlainStringsDiagram {
    let strings = [
        format!("<b>rockuml version {ROCKUML_VERSION}</b>"),
        "(LGPL source distribution)".to_owned(),
        " ".to_owned(),
        format!("A port of PlantUML {} to Rust.", crate::PLANTUML_VERSION),
        "<u>PlantUML</u>: Arnaud Roques and its contributors".to_owned(),
        " ".to_owned(),
        "https://github.com/h3ll5ur7er/rockuml".to_owned(),
        "https://plantuml.com".to_owned(),
        " ".to_owned(),
    ];
    PlainStringsDiagram::new(strings.into())
}

/// `stdlib`: the libraries of the standard library rockuml embeds (`Stdlib.addInfoVersion`).
fn create_std_lib() -> PlainStringsDiagram {
    let mut strings = Vec::new();
    for name in library_names() {
        let Some(folder) = Stdlib::retrieve(&name) else {
            continue;
        };
        strings.push(format!("<b>{name}"));
        strings.push(format!("Version {}", java_string(folder.version())));
        strings.push(format!("Delivered by {}", java_string(folder.source())));
        strings.push(" ".to_owned());
    }
    strings.push(" ".to_owned());
    PlainStringsDiagram::with_plantuml_image(strings)
}

/// A string as Java concatenates it, `null` when there is none.
fn java_string(value: Option<String>) -> String {
    value.unwrap_or_else(|| "null".to_owned())
}

/// `testdot`: the program's version, and what it lays graphs out with.
fn create_test_dot() -> PlainStringsDiagram {
    let mut strings = vec![format!(
        "rockuml version {ROCKUML_VERSION} (PlantUML {} compatible)",
        crate::PLANTUML_VERSION
    )];
    add_dot_status(&mut strings);
    PlainStringsDiagram::new(strings)
}

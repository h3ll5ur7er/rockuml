//! Builds a diagram whose type its lines decide: tries each diagram type in PlantUML's order and keeps the
//! first the lines make, or else the error that got furthest (PlantUML's `PSystemBuilder` and
//! `PSystemErrorUtils`).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::chen::ChenEerDiagramFactory;
use super::class::ClassDiagramFactory;
use super::description::DescriptionDiagramFactory;
use super::diagram_type::DiagramType;
use super::error::ErrorDiagram;
use super::sequence::SequenceDiagramFactory;
use super::state::StateDiagramFactory;
use super::titled::TitledDiagram;
use super::unported::{
    ActivityDiagramFactory, ActivityDiagramFactory3, HelpFactory, ListSpriteDiagramFactory,
    TimingDiagramFactory,
};
use super::{Diagram, NotYetPorted, UmlSource};
use crate::command::factory::{self, AbstractDiagram, Created, ParseFailure};
use crate::command::{Command, CommandError};
use crate::pattern::java_regex;
use crate::preproc::start_utils;
use crate::text::StringLocated;

/// A diagram type's error, kept until it is known whether another type reads the lines better.
pub(super) struct PSystemError {
    source: Rc<UmlSource>,
    failure: ParseFailure,
    diagram_type: DiagramType,
}

impl PSystemError {
    fn into_diagram(self) -> Box<dyn Diagram> {
        Box::new(ErrorDiagram::new(
            self.source,
            self.failure.trace,
            &self.failure.error.message,
            Some(self.diagram_type),
        ))
    }
}

/// What one diagram type made of the lines.
pub(super) enum Outcome {
    /// A diagram, which rockuml may not be able to draw yet.
    Diagram(Result<Box<dyn Diagram>, NotYetPorted>),
    Error(PSystemError),
    /// The lines are no such diagram, without an error either.
    Nothing,
}

/// A diagram type built by commands (PlantUML's `PSystemCommandFactory`).
pub(super) trait CommandFactory: 'static {
    type Diagram: AbstractDiagram + TitledDiagram + Diagram + 'static;

    const DIAGRAM_TYPE: DiagramType;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> Self::Diagram;

    /// The commands in the order they are tried on each line.
    fn init_commands_list() -> Vec<Box<dyn Command<Self::Diagram>>>;
}

/// The diagram the factory `F` makes of the source.
pub(super) fn create_system<F: CommandFactory>(source: &Rc<UmlSource>) -> Outcome {
    let commands = commands::<F>();
    match factory::create_system(source, || F::create_empty_diagram(source), &commands) {
        Created::Diagram(mut diagram) => {
            Outcome::Diagram(match diagram.titled().not_ported_part() {
                Some(not_ported) => Err(not_ported),
                None => Ok(Box::new(diagram)),
            })
        }
        Created::Failure(failure) => Outcome::Error(PSystemError {
            source: source.clone(),
            failure,
            diagram_type: F::DIAGRAM_TYPE,
        }),
        Created::Nothing => Outcome::Nothing,
    }
}

/// The factory's commands, made once per thread like PlantUML's factories make theirs once: commands compile
/// their patterns when first used, and every diagram tries the patterns of several factories.
fn commands<F: CommandFactory>() -> Rc<Vec<Box<dyn Command<F::Diagram>>>> {
    thread_local! {
        static COMMANDS: RefCell<HashMap<TypeId, Rc<dyn Any>>> = RefCell::default();
    }
    COMMANDS.with(|commands| {
        let made = commands
            .borrow_mut()
            .entry(TypeId::of::<F>())
            .or_insert_with(|| Rc::new(F::init_commands_list()))
            .clone();
        made.downcast()
            .unwrap_or_else(|_| unreachable!("commands are kept by their factory's type"))
    })
}

type Factory = fn(&Rc<UmlSource>) -> Outcome;

/// The factories of the diagram types `@startuml` may stand for, in PlantUML's order. PlantUML's easter eggs
/// are left out.
const UML_FACTORIES: [Factory; 20] = [
    welcome,
    colors,
    create_system::<SequenceDiagramFactory>,
    create_system::<ClassDiagramFactory>,
    create_system::<ActivityDiagramFactory>,
    create_system::<DescriptionDiagramFactory>,
    create_system::<StateDiagramFactory>,
    create_system::<ActivityDiagramFactory3>,
    license,
    version,
    donors,
    skinparameter_list,
    list_fonts,
    list_emoji,
    open_iconic,
    list_open_iconic,
    list_archimate_sprites,
    create_system::<ListSpriteDiagramFactory>,
    create_system::<TimingDiagramFactory>,
    create_system::<HelpFactory>,
];

/// A `@startuml` diagram. `located_lines` are the block's lines before continuation lines were joined.
pub(super) fn create_uml(
    source: UmlSource,
    located_lines: &[StringLocated],
) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let source = Rc::new(source);
    if let Some(error) = check_basic_error(&source, located_lines) {
        return Ok(error.into_diagram());
    }
    select(&source, &UML_FACTORIES)
}

/// A `@startchen` diagram.
pub(super) fn create_chen(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<ChenEerDiagramFactory>])
}

/// The first diagram a factory makes, or else the error with the best score, the earlier on a tie.
fn select(source: &Rc<UmlSource>, factories: &[Factory]) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let mut errors = Vec::new();
    for factory in factories {
        match factory(source) {
            Outcome::Diagram(diagram) => return diagram,
            Outcome::Error(error) => errors.push(error),
            Outcome::Nothing => {}
        }
    }
    merge(errors)
        .map(PSystemError::into_diagram)
        .ok_or(NotYetPorted("diagrams that no diagram type reads"))
}

/// The error that got furthest, the earliest on a tie (`PSystemErrorUtils.merge`).
fn merge(errors: Vec<PSystemError>) -> Option<PSystemError> {
    errors.into_iter().reduce(|best, error| {
        if best.failure.score() < error.failure.score() {
            error
        } else {
            best
        }
    })
}

/// The help PlantUML gives for sources meant for other start lines (`PSystemErrorUtils.checkBasicError`).
fn check_basic_error(
    source: &Rc<UmlSource>,
    located_lines: &[StringLocated],
) -> Option<PSystemError> {
    static DOT_HEADER: LazyLock<Regex> = LazyLock::new(|| {
        java_regex(
            r#"^\s*(strict\s+)?(di)?graph\s+([_\p{L}][_\p{L}\p{N}]*|-?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)|"([^"\\]|\\")*")?\s*\{\s*$"#,
            false,
        )
    });
    let second = located_lines.get(1)?.text();
    let trimmed = crate::java::trim(second);
    let message = if DOT_HEADER.is_match(second) {
        "This looks like a DOT diagram. Please use @startdot instead of @startuml."
    } else if trimmed == "ditaa" {
        "This looks like a DITAA diagram. Please use @startditaa instead of @startuml."
    } else if trimmed == "salt" {
        "This looks like a salt diagram. Please use @startsalt instead of @startuml."
    } else if trimmed == "nwdiag {" {
        "This looks like a network diagram. Please use @startnwdiag instead of @startuml."
    } else {
        return None;
    };
    Some(PSystemError {
        source: source.clone(),
        failure: ParseFailure {
            error: CommandError::with_score(message, 100),
            trace: located_lines.to_vec(),
        },
        diagram_type: DiagramType::Sequence,
    })
}

/// `@startuml` and `@enduml` alone show PlantUML's welcome screen.
fn welcome(source: &Rc<UmlSource>) -> Outcome {
    if source.lines().len() == 2 {
        Outcome::Diagram(Err(NotYetPorted("the welcome screen")))
    } else {
        Outcome::Nothing
    }
}

/// A diagram of a single keyword line, which `accepts` recognises (PlantUML's `PSystemSingleLineFactory`).
fn single_line(
    source: &Rc<UmlSource>,
    accepts: impl Fn(&str) -> bool,
    what: &'static str,
) -> Outcome {
    if source.lines().len() - source.initial_noise() != 3 {
        return Outcome::Nothing;
    }
    let source = Rc::new(source.as_ref().clone().without_initial_noise());
    let lines = source.lines();
    let failure = |error: &str, trace: &[StringLocated]| {
        Outcome::Error(PSystemError {
            source: source.clone(),
            failure: ParseFailure {
                error: CommandError::new(error),
                trace: trace.to_vec(),
            },
            diagram_type: DiagramType::Sequence,
        })
    };
    if source.is_empty() {
        return failure("Empty description", &lines[..1]);
    }
    let line = lines[1].text();
    if start_utils::is_end_directive(line) {
        failure("Empty description", &lines[..2])
    } else if accepts(line) {
        Outcome::Diagram(Err(NotYetPorted(what)))
    } else {
        failure("Syntax Error?", &lines[..2])
    }
}

/// Whether the whole line matches, like Java's `String.matches`.
fn matches_whole(pattern: &str, line: &str) -> bool {
    java_regex(&format!("^(?:{pattern})$"), false).is_match(line)
}

fn colors(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| matches_whole(r"colors?\s*(#?\w+)?\s*", line),
        "the colors diagram",
    )
}

fn license(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| matches_whole(r"(?i)li[sc][ea]n[sc]e\s*", line),
        "the license diagram",
    )
}

fn version(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| {
            matches_whole(
                r"(?i)(authors?|about|version|stdlib|testdot|keydistributor|keygen|keyimport(\s+[0-9a-z]+)?|keycheck\s+([0-9a-z]+)\s+([0-9a-z]+))\s*",
                line,
            )
        },
        "the version diagrams",
    )
}

fn donors(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| matches_whole(r"(?i)(donors)\s*", line),
        "the donors diagram",
    )
}

fn skinparameter_list(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| matches_whole(r"(?i)(skinparameters)\s*", line),
        "the skin parameter list",
    )
}

fn list_fonts(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| {
            let line = line.to_lowercase();
            ["listfont", "listfonts"].contains(&line.as_str())
                || line.starts_with("listfont ")
                || line.starts_with("listfonts ")
        },
        "the font list",
    )
}

fn list_emoji(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| {
            let line = line.to_lowercase();
            line == "emoji" || line.starts_with("emoji ")
        },
        "the emoji list",
    )
}

fn open_iconic(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| line.to_lowercase().starts_with("openiconic "),
        "the OpenIconic diagram",
    )
}

fn list_open_iconic(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| line.to_lowercase().starts_with("listopeniconic"),
        "the OpenIconic list",
    )
}

fn list_archimate_sprites(source: &Rc<UmlSource>) -> Outcome {
    single_line(
        source,
        |line| line.to_lowercase().starts_with("listsprite"),
        "the Archimate sprite list",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LineLocation;

    fn lines(texts: &[&str]) -> Vec<StringLocated> {
        let location = LineLocation::new("test", None);
        texts
            .iter()
            .map(|text| StringLocated::new(*text, location.clone()))
            .collect()
    }

    fn create(texts: &[&str]) -> Result<Box<dyn Diagram>, NotYetPorted> {
        let lines = lines(texts);
        create_uml(UmlSource::new(lines.clone(), Vec::new()), &lines)
    }

    /// What the lines read as: the part not ported, or `"error"`.
    fn read_as(texts: &[&str]) -> &'static str {
        match create(texts) {
            Ok(diagram) if diagram.is_error() => "error",
            Ok(_) => "a drawable diagram",
            Err(NotYetPorted(what)) => what,
        }
    }

    #[test]
    fn each_diagram_type_reads_its_own_lines() {
        assert_eq!(read_as(&["@startuml", "@enduml"]), "the welcome screen");
        assert_eq!(
            read_as(&["@startuml", "Alice -> Bob", "@enduml"]),
            "a drawable diagram"
        );
        assert_eq!(
            read_as(&["@startuml", "class A", "A <|-- B", "@enduml"]),
            "class diagrams"
        );
        assert_eq!(
            read_as(&[
                "@startuml",
                "(*) --> \"First\"",
                "\"First\" --> (*)",
                "@enduml"
            ]),
            "legacy activity diagrams"
        );
        assert_eq!(
            read_as(&["@startuml", "actor User", "User --> (Login)", "@enduml"]),
            "usecase, component and deployment diagrams"
        );
        assert_eq!(
            read_as(&["@startuml", "[*] --> Idle", "@enduml"]),
            "state diagrams"
        );
        assert_eq!(
            read_as(&["@startuml", "start", ":Hello;", "stop", "@enduml"]),
            "activity diagrams"
        );
        assert_eq!(
            read_as(&[
                "@startuml",
                "concise \"Web\" as W",
                "@0",
                "W is Idle",
                "@enduml"
            ]),
            "timing diagrams"
        );
        assert_eq!(
            read_as(&["@startuml", "license", "@enduml"]),
            "the license diagram"
        );
    }

    #[test]
    fn lines_no_type_reads_fail_with_the_error_that_got_furthest() {
        assert_eq!(
            read_as(&["@startuml", "class A", "activate A", "@enduml"]),
            "error"
        );
        assert_eq!(
            read_as(&["@startuml", "digraph G {", "}", "@enduml"]),
            "error"
        );
    }

    fn error(trace_length: usize, score: i32, diagram_type: DiagramType) -> PSystemError {
        PSystemError {
            source: Rc::new(UmlSource::new(Vec::new(), Vec::new())),
            failure: ParseFailure {
                error: CommandError::with_score("Syntax Error?", score),
                trace: lines(&vec!["line"; trace_length]),
            },
            diagram_type,
        }
    }

    #[test]
    fn the_best_score_wins_and_the_earlier_error_a_tie() {
        let best = |errors| merge(errors).map(|error| error.diagram_type);
        assert_eq!(
            best(vec![
                error(2, 0, DiagramType::Sequence),
                error(3, 0, DiagramType::Class),
                error(3, 0, DiagramType::State),
            ]),
            Some(DiagramType::Class)
        );
        assert_eq!(
            best(vec![
                error(3, 0, DiagramType::Sequence),
                error(2, 10, DiagramType::Class),
            ]),
            Some(DiagramType::Sequence)
        );
        assert_eq!(
            best(vec![
                error(2, 10, DiagramType::Sequence),
                error(2, 11, DiagramType::Class),
            ]),
            Some(DiagramType::Class)
        );
        assert_eq!(best(Vec::new()), None);
    }
}

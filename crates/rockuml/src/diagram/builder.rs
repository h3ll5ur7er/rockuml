//! Builds a diagram whose type its lines decide: tries each diagram type in PlantUML's order and keeps the
//! first the lines make, or else the error that got furthest (PlantUML's `PSystemBuilder` and
//! `PSystemErrorUtils`).

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
pub(super) trait CommandFactory {
    type Diagram: AbstractDiagram + TitledDiagram + Diagram + 'static;

    const DIAGRAM_TYPE: DiagramType;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> Self::Diagram;

    /// The commands in the order they are tried on each line.
    fn init_commands_list() -> Vec<Box<dyn Command<Self::Diagram>>>;
}

/// The diagram the factory `F` makes of the source.
pub(super) fn create_system<F: CommandFactory>(source: &Rc<UmlSource>) -> Outcome {
    let commands = F::init_commands_list();
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
    let mut best: Option<PSystemError> = None;
    for factory in factories {
        match factory(source) {
            Outcome::Diagram(diagram) => return diagram,
            Outcome::Error(error) => {
                if best
                    .as_ref()
                    .is_none_or(|best| best.failure.score() < error.failure.score())
                {
                    best = Some(error);
                }
            }
            Outcome::Nothing => {}
        }
    }
    best.map(PSystemError::into_diagram)
        .ok_or(NotYetPorted("diagrams that no diagram type reads"))
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

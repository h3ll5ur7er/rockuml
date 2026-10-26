//! Builds a diagram whose type its lines decide: tries each diagram type in PlantUML's order and keeps the
//! first the lines make, or else the error that got furthest (PlantUML's `PSystemBuilder` and
//! `PSystemErrorUtils`).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::activity::ActivityDiagramFactory;
use super::activity3::ActivityDiagramFactory3;
use super::board::BoardDiagramFactory;
use super::cards::{
    PSystemColorsFactory, PSystemLicenseFactory, PSystemListFontsFactory,
    PSystemSkinparameterListFactory, PSystemVersionFactory, PSystemWelcomeFactory,
};
use super::chart::ChartDiagramFactory;
use super::chen::ChenEerDiagramFactory;
use super::class::ClassDiagramFactory;
use super::description::DescriptionDiagramFactory;
use super::diagram_type::DiagramType;
use super::ebnf::PSystemEbnfFactory;
use super::error::ErrorDiagram;
use super::flow::FlowDiagramFactory;
use super::gantt::GanttDiagramFactory;
use super::help::HelpFactory;
use super::mindmap::MindMapDiagramFactory;
use super::nwdiag::NwDiagramFactory;
use super::packetdiag::PacketDiagramFactory;
use super::regexdiagram::PSystemRegexFactory;
use super::sequence::SequenceDiagramFactory;
use super::sprite_list::ListSpriteDiagramFactory;
use super::state::StateDiagramFactory;
use super::timing::TimingDiagramFactory;
use super::titled::TitledDiagram;
use super::wbs::WbsDiagramFactory;
use super::{Diagram, NotYetPorted, UmlSource};
use super::{emoji_list, openiconic, sprite_list};
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
    PSystemWelcomeFactory::create_system,
    create_single_line_system::<PSystemColorsFactory>,
    create_system::<SequenceDiagramFactory>,
    create_system::<ClassDiagramFactory>,
    create_system::<ActivityDiagramFactory>,
    create_system::<DescriptionDiagramFactory>,
    create_system::<StateDiagramFactory>,
    create_system::<ActivityDiagramFactory3>,
    create_single_line_system::<PSystemLicenseFactory>,
    create_single_line_system::<PSystemVersionFactory>,
    donors,
    create_single_line_system::<PSystemSkinparameterListFactory>,
    create_single_line_system::<PSystemListFontsFactory>,
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

/// A `@startmindmap` diagram.
pub(super) fn create_mindmap(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<MindMapDiagramFactory>])
}

/// A `@startgantt` diagram.
pub(super) fn create_gantt(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<GanttDiagramFactory>])
}

/// A `@startnwdiag` diagram.
pub(super) fn create_nwdiag(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<NwDiagramFactory>])
}

/// A `@startpacketdiag` diagram.
pub(super) fn create_packet(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<PacketDiagramFactory>])
}

/// A `@startboard` diagram.
pub(super) fn create_board(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<BoardDiagramFactory>])
}

/// A `@startchart` diagram.
pub(super) fn create_chart(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<ChartDiagramFactory>])
}

/// A `@startwbs` diagram.
pub(super) fn create_wbs(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<WbsDiagramFactory>])
}

/// A `@startebnf` diagram.
pub(super) fn create_ebnf(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<PSystemEbnfFactory>])
}

/// A `@startregex` diagram.
pub(super) fn create_regex(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<PSystemRegexFactory>])
}

/// A `@startflow` diagram.
pub(super) fn create_flow(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    select(&Rc::new(source), &[create_system::<FlowDiagramFactory>])
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

/// A diagram type of a single line between the start and end lines (PlantUML's `PSystemSingleLineFactory`).
pub(super) trait PSystemSingleLineFactory {
    /// The diagram `line` makes, or `None` if it is no diagram of this type (`executeLine`). `source` is
    /// without its initial noise.
    fn execute_line(
        source: &Rc<UmlSource>,
        line: &str,
    ) -> Option<Result<Box<dyn Diagram>, NotYetPorted>>;
}

/// The diagram the single-line factory `F` makes of the source.
pub(super) fn create_single_line_system<F: PSystemSingleLineFactory>(
    source: &Rc<UmlSource>,
) -> Outcome {
    single_line_with(source, F::execute_line)
}

/// A diagram of a single keyword line, which `accepts` recognises, of a type rockuml cannot draw yet.
fn single_line(
    source: &Rc<UmlSource>,
    accepts: impl Fn(&str) -> bool,
    what: &'static str,
) -> Outcome {
    single_line_with(source, |_, line| {
        accepts(line).then_some(Err(NotYetPorted(what)))
    })
}

/// The diagram `execute_line` makes of a source of a single line, or else a syntax error (PlantUML's
/// `PSystemSingleLineFactory`). It is given the source without its initial noise.
fn single_line_with(
    source: &Rc<UmlSource>,
    execute_line: impl Fn(&Rc<UmlSource>, &str) -> Option<Result<Box<dyn Diagram>, NotYetPorted>>,
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
        return failure("Empty description", &lines[..2]);
    }
    match execute_line(&source, line) {
        Some(diagram) => Outcome::Diagram(diagram),
        None => failure("Syntax Error?", &lines[..2]),
    }
}

/// A pattern the whole line must match, like Java's `String.matches`.
pub(super) fn whole_line(pattern: &str) -> Regex {
    java_regex(&format!("^(?:{pattern})$"), false)
}

fn donors(source: &Rc<UmlSource>) -> Outcome {
    static DONORS: LazyLock<Regex> = LazyLock::new(|| whole_line(r"(?i)(donors)\s*"));
    single_line(source, |line| DONORS.is_match(line), "the donors diagram")
}

fn list_emoji(source: &Rc<UmlSource>) -> Outcome {
    single_line_with(source, |source, line| {
        emoji_list::execute_line(source, line).map(Ok)
    })
}

fn open_iconic(source: &Rc<UmlSource>) -> Outcome {
    single_line_with(source, openiconic::execute_line)
}

fn list_open_iconic(source: &Rc<UmlSource>) -> Outcome {
    single_line_with(source, |source, line| {
        openiconic::execute_list_line(source, line).map(Ok)
    })
}

fn list_archimate_sprites(source: &Rc<UmlSource>) -> Outcome {
    single_line_with(source, |source, line| {
        sprite_list::execute_line(source, line).map(Ok)
    })
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

    /// What the lines read as: the part not ported, `"error"`, or the type a drawable diagram announces.
    fn read_as(texts: &[&str]) -> &'static str {
        match create(texts) {
            Ok(diagram) if diagram.is_error() => "error",
            Ok(diagram) => diagram
                .export_settings(0)
                .diagram_type
                .unwrap_or("a drawable diagram"),
            Err(NotYetPorted(what)) => what,
        }
    }

    #[test]
    fn each_diagram_type_reads_its_own_lines() {
        assert_eq!(
            read_as(&["@startuml", "Alice -> Bob", "@enduml"]),
            "SEQUENCE"
        );
        assert_eq!(
            read_as(&["@startuml", "class A", "A <|-- B", "@enduml"]),
            "CLASS"
        );
        assert_eq!(
            read_as(&[
                "@startuml",
                "(*) --> \"First\"",
                "\"First\" --> (*)",
                "@enduml"
            ]),
            "ACTIVITY"
        );
        assert_eq!(
            read_as(&["@startuml", "actor User", "User --> (Login)", "@enduml"]),
            "DESCRIPTION"
        );
        assert_eq!(read_as(&["@startuml", "[*] --> Idle", "@enduml"]), "STATE");
        assert_eq!(
            read_as(&["@startuml", "start", ":Hello;", "stop", "@enduml"]),
            "ACTIVITY"
        );
        assert_eq!(
            read_as(&[
                "@startuml",
                "concise \"Web\" as W",
                "@0",
                "W is Idle",
                "@enduml"
            ]),
            "TIMING"
        );
    }

    #[test]
    fn single_keyword_lines_draw_reference_cards() {
        let drawn = "a drawable diagram";
        assert_eq!(read_as(&["@startuml", "@enduml"]), drawn);
        for card in [
            "colors",
            "color #Pink",
            "Licence",
            "version",
            "about",
            "stdlib",
            "testdot",
            "skinparameters",
            "listfonts Hello",
        ] {
            assert_eq!(read_as(&["@startuml", card, "@enduml"]), drawn, "{card}");
        }
        assert_eq!(read_as(&["@startuml", "keygen", "@enduml"]), "license keys");
        assert_eq!(read_as(&["@startuml", "Colors", "@enduml"]), "error");
        assert_eq!(read_as(&["@startuml", "", "@enduml"]), "error");
    }

    #[test]
    fn reference_cards_read_their_keyword_lines() {
        assert_eq!(read_as(&["@startuml", "help fonts", "@enduml"]), "HELP");
        assert_eq!(
            read_as(&[
                "@startuml",
                "sprite foo <svg></svg>",
                "listsprites",
                "@enduml"
            ]),
            "HELP"
        );
        for card in [
            "emoji 1f6",
            "openiconic heart",
            "listopeniconic",
            "listsprite",
        ] {
            assert_eq!(
                read_as(&["@startuml", card, "@enduml"]),
                "a drawable diagram",
                "{card}"
            );
        }
        assert_eq!(
            read_as(&["@startuml", "openiconic no-such-icon", "@enduml"]),
            "the crash report of an unknown Open Iconic icon"
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

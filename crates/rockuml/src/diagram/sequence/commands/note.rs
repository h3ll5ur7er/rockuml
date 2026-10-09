//! Notes: on a participant, over several, across all, or on the last message, on one line or as a block
//! ending with `end note` (PlantUML's `FactorySequenceNote*Command`).

use std::sync::LazyLock;

use regex::Regex;

use super::{PARTICIPANT_CODE_OR_QUOTED, optional_colors, optional_url, unquoted};
use crate::color::{ColorType, Colors};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, SingleLine, SingleLineCommand,
};
use crate::creole::Display;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{Note, NotePosition, NoteStyle, ParticipantId};
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stereo::{self, Stereotype};
use crate::text::LineLocation;

/// Where a kind of note goes.
#[derive(Clone, Copy)]
enum Kind {
    /// `note left of A`, `note over A`.
    OnParticipant,
    /// `note over A, B`.
    OverSeveral,
    /// `note across`.
    Across,
    /// `note left` after a message.
    OnArrow,
}

fn style_and_stereotype() -> Vec<RegexTree> {
    vec![
        RegexTree::named(1, "STYLE", "(note|hnote|rnote)"),
        stereo::optional_pattern("STEREO1"),
    ]
}

/// The pattern of a kind of note, up to its colour and link; single lines go on with the text.
fn head(kind: Kind, single_line: bool) -> Vec<RegexTree> {
    let mut parts = vec![RegexTree::start()];
    match kind {
        Kind::OnParticipant => {
            parts.extend([
                RegexTree::named(1, "PARALLEL", r"(&[%s]*)?"),
                RegexTree::named(1, "VMERGE", "(/)?"),
                RegexTree::spaces_zero_or_more(),
            ]);
            parts.extend(style_and_stereotype());
            let of = if single_line {
                r"(?:of[%s])?"
            } else {
                r"(?:of[%s]+)?"
            };
            parts.extend([
                RegexTree::named(1, "POSITION", "(right|left|over)"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(
                    1,
                    "PARTICIPANT",
                    format!("{of}{PARTICIPANT_CODE_OR_QUOTED}"),
                ),
                stereo::optional_pattern("STEREO2"),
            ]);
        }
        Kind::OverSeveral => {
            parts.extend([
                RegexTree::named(1, "PARALLEL", r"(&[%s]*)?"),
                RegexTree::named(1, "VMERGE", "(/)?"),
                RegexTree::spaces_zero_or_more(),
            ]);
            parts.extend(style_and_stereotype());
            parts.extend([
                RegexTree::leaf("over"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "P1", PARTICIPANT_CODE_OR_QUOTED),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(","),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "P2", PARTICIPANT_CODE_OR_QUOTED),
                stereo::optional_pattern("STEREO2"),
            ]);
        }
        Kind::Across => {
            parts.extend([
                RegexTree::named(1, "VMERGE", "(/)?"),
                RegexTree::spaces_zero_or_more(),
            ]);
            parts.extend(style_and_stereotype());
            parts.extend([
                RegexTree::named(1, "ACROSS", "(accross|across)"),
                stereo::optional_pattern("STEREO2"),
            ]);
        }
        Kind::OnArrow => {
            parts.push(RegexTree::spaces_zero_or_more());
            parts.extend(style_and_stereotype());
            parts.extend([
                RegexTree::named(1, "POSITION", "(right|left|bottom|top)"),
                stereo::optional_pattern("STEREO2"),
            ]);
        }
    }
    parts.extend([
        optional_colors(),
        RegexTree::spaces_zero_or_more(),
        optional_url(),
    ]);
    if single_line {
        parts.extend([
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NOTE", "(.*)"),
        ]);
    }
    parts.push(RegexTree::end());
    parts
}

struct SingleLineNote {
    kind: Kind,
    pattern: RegexTree,
}

fn single(kind: Kind) -> Box<dyn Command<SequenceDiagram>> {
    Box::new(SingleLine(SingleLineNote {
        kind,
        pattern: RegexTree::concat(head(kind, true)),
    }))
}

/// PlantUML's `FactorySequenceNoteCommand`, single line.
pub(super) fn single_line() -> Box<dyn Command<SequenceDiagram>> {
    single(Kind::OnParticipant)
}

/// PlantUML's `FactorySequenceNoteOverSeveralCommand`, single line.
pub(super) fn over_several_single_line() -> Box<dyn Command<SequenceDiagram>> {
    single(Kind::OverSeveral)
}

/// PlantUML's `FactorySequenceNoteAcrossCommand`, single line.
pub(super) fn across_single_line() -> Box<dyn Command<SequenceDiagram>> {
    single(Kind::Across)
}

/// PlantUML's `FactorySequenceNoteOnArrowCommand`, single line.
pub(super) fn on_arrow_single_line() -> Box<dyn Command<SequenceDiagram>> {
    single(Kind::OnArrow)
}

impl SingleLineCommand<SequenceDiagram> for SingleLineNote {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(
        &self,
        diagram: &mut SequenceDiagram,
        _location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        let display = Display::with_newlines(arg.get("NOTE", 0).unwrap_or_default());
        let display = diagram.manage_variable(&display);
        execute(self.kind, diagram, arg, &display)
    }
}

/// The patterns of the first lines of multi-line notes, by kind.
static BLOCKS: LazyLock<[RegexTree; 4]> = LazyLock::new(|| {
    [
        Kind::OnParticipant,
        Kind::OverSeveral,
        Kind::Across,
        Kind::OnArrow,
    ]
    .map(|kind| RegexTree::concat(head(kind, false)))
});

fn block_index(kind: Kind) -> usize {
    match kind {
        Kind::OnParticipant => 0,
        Kind::OverSeveral => 1,
        Kind::Across => 2,
        Kind::OnArrow => 3,
    }
}

static END: LazyLock<Regex> = LazyLock::new(|| plantuml_regex("^end[%s]?(note|hnote|rnote)$"));
static END_ON_ARROW: LazyLock<Regex> = LazyLock::new(|| plantuml_regex("^[%s]*end[%s]?note$"));

/// PlantUML's `FactorySequenceNoteCommand`, several lines.
pub(super) fn multi_line() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(Multiline::starting_with(
        &BLOCKS[0],
        &END,
        |diagram, lines| multi(Kind::OnParticipant, diagram, lines),
    ))
}

/// PlantUML's `FactorySequenceNoteOverSeveralCommand`, several lines.
pub(super) fn over_several_multi_line() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(Multiline::starting_with(
        &BLOCKS[1],
        &END,
        |diagram, lines| multi(Kind::OverSeveral, diagram, lines),
    ))
}

/// PlantUML's `FactorySequenceNoteAcrossCommand`, several lines.
pub(super) fn across_multi_line() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(Multiline::starting_with(
        &BLOCKS[2],
        &END,
        |diagram, lines| multi(Kind::Across, diagram, lines),
    ))
}

/// PlantUML's `FactorySequenceNoteOnArrowCommand`, several lines.
pub(super) fn on_arrow_multi_line() -> Box<dyn Command<SequenceDiagram>> {
    Box::new(Multiline::starting_with(
        &BLOCKS[3],
        &END_ON_ARROW,
        |diagram, lines| multi(Kind::OnArrow, diagram, lines),
    ))
}

fn multi(kind: Kind, diagram: &mut SequenceDiagram, lines: &BlocLines) -> CommandResult {
    let first = lines.first().expect("a block has its first line").trimmed();
    let arg = BLOCKS[block_index(kind)]
        .matcher(first.text())
        .expect("the start pattern matched");
    let body = lines.sub_extract(1, 1).without_empty_columns();
    let display = diagram.manage_variable(&body.to_display());
    execute(kind, diagram, &arg, &display)
}

fn stereotype(arg: &RegexResult) -> Option<Stereotype> {
    arg.get_lazzy("STEREO", 0).map(Stereotype::new)
}

fn execute(
    kind: Kind,
    diagram: &mut SequenceDiagram,
    arg: &RegexResult,
    display: &Display,
) -> CommandResult {
    let style = NoteStyle::named(arg.get("STYLE", 0).unwrap_or_default());
    let style_builder = diagram.style_builder();
    let note = |participant, participant2, position, colors| Note {
        participant,
        participant2,
        display: display.clone(),
        position,
        style,
        colors,
        stereotype: stereotype(arg),
        parallel: arg.get("PARALLEL", 0).is_some(),
        style_builder: style_builder.clone(),
    };
    match kind {
        Kind::OnParticipant => {
            let code = unquoted(arg.get("PARTICIPANT", 0).unwrap_or_default()).to_owned();
            let participant = diagram.get_or_create_participant(&code, None);
            let position = NotePosition::named(arg.get("POSITION", 0).unwrap_or_default())
                .expect("the pattern only matches positions");
            add(
                diagram,
                arg,
                note(Some(participant), None, position, super::colors(arg)?),
            );
        }
        Kind::OverSeveral => {
            let participants: Vec<ParticipantId> = ["P1", "P2"]
                .iter()
                .map(|name| {
                    let code = unquoted(arg.get(name, 0).unwrap_or_default()).to_owned();
                    diagram.get_or_create_participant(&code, None)
                })
                .collect();
            add(
                diagram,
                arg,
                note(
                    Some(participants[0]),
                    Some(participants[1]),
                    NotePosition::OverSeveral,
                    super::colors(arg)?,
                ),
            );
        }
        Kind::Across => {
            if arg
                .get("ACROSS", 0)
                .is_some_and(|across| across.eq_ignore_ascii_case("accross"))
            {
                return Err(CommandError::new("Use 'across' instead of 'accross'"));
            }
            add(
                diagram,
                arg,
                note(None, None, NotePosition::OverSeveral, super::colors(arg)?),
            );
        }
        Kind::OnArrow => {
            let position = NotePosition::named(arg.get("POSITION", 0).unwrap_or_default())
                .expect("the pattern only matches positions");
            let colors = Colors::default().with(
                ColorType::Back,
                arg.get("COLOR", 0).map(super::color_named).transpose()?,
            );
            let note = Note {
                parallel: false,
                ..note(None, None, position, colors)
            };
            if let Some(event) = diagram.last_event_with_note() {
                event.add_note(note);
            }
        }
    }
    Ok(())
}

/// Empty notes are dropped.
fn add(diagram: &mut SequenceDiagram, arg: &RegexResult, note: Note) {
    if !note.display.lines().is_empty() {
        diagram.add_note(note, arg.get("VMERGE", 0).is_some());
    }
}

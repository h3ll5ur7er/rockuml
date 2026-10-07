//! What a sequence diagram is made of: participants and the events between them, in source order
//! (PlantUML's `Participant`, `Event` and its implementations).

use std::rc::Rc;

use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::klimt::url::Url;
use crate::skin::arrow::ArrowConfiguration;
use crate::stereo::Stereotype;
use crate::style::StyleBuilder;

/// A participant, by its place in the diagram's list of participants ever declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ParticipantId(pub usize);

/// An event, by its place in the diagram's events.
pub(crate) type EventId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParticipantType {
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Queue,
    Database,
    Collections,
}

impl ParticipantType {
    pub(crate) fn named(name: &str) -> Option<Self> {
        Some(match name.to_lowercase().as_str() {
            "participant" => Self::Participant,
            "actor" => Self::Actor,
            "boundary" => Self::Boundary,
            "control" => Self::Control,
            "entity" => Self::Entity,
            "queue" => Self::Queue,
            "database" => Self::Database,
            "collections" => Self::Collections,
            _ => return None,
        })
    }
}

/// The colours an activation is drawn with (`Fashion` in PlantUML's life events).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LiveColors {
    pub back: Option<HColor>,
    pub line: Option<HColor>,
}

pub(crate) struct Participant {
    pub code: String,
    pub display: Display,
    pub kind: ParticipantType,
    /// Activations before any message, with their colours.
    pub initial_lives: Vec<LiveColors>,
    /// The stereotype, and whether it goes above the name.
    pub stereotype: Option<(Stereotype, bool)>,
    pub colors: Colors,
    pub url: Option<Url>,
    pub order: i32,
    pub style_builder: Rc<StyleBuilder>,
}

/// A box drawn around several participants (`box ... end box`).
pub(crate) struct ParticipantEnglober {
    pub parent: Option<usize>,
    pub title: Display,
    pub box_color: Option<HColor>,
    pub stereotype: Option<Stereotype>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LifeEventType {
    Activate,
    Deactivate,
    Destroy,
    Create,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotePosition {
    Left,
    Right,
    Over,
    OverSeveral,
    Bottom,
    Top,
}

impl NotePosition {
    pub(crate) fn named(name: &str) -> Option<Self> {
        Some(match name.to_lowercase().as_str() {
            "left" => Self::Left,
            "right" => Self::Right,
            "over" => Self::Over,
            "bottom" => Self::Bottom,
            "top" => Self::Top,
            _ => return None,
        })
    }
}

/// The shape of a note: folded corner, hexagon (`hnote`) or box (`rnote`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteStyle {
    Normal,
    Hexagonal,
    Box,
}

impl NoteStyle {
    pub(crate) fn named(name: &str) -> Self {
        if name.eq_ignore_ascii_case("hnote") {
            Self::Hexagonal
        } else if name.eq_ignore_ascii_case("rnote") {
            Self::Box
        } else {
            Self::Normal
        }
    }
}

#[derive(Clone)]
pub(crate) struct Note {
    /// Notes across all participants have none.
    pub participant: Option<ParticipantId>,
    pub participant2: Option<ParticipantId>,
    pub display: Display,
    pub position: NotePosition,
    pub style: NoteStyle,
    pub colors: Colors,
    pub stereotype: Option<Stereotype>,
    pub parallel: bool,
    pub style_builder: Rc<StyleBuilder>,
}

impl Note {
    fn deals_with(&self, participant: ParticipantId) -> bool {
        self.participant == Some(participant) || self.participant2 == Some(participant)
    }
}

/// What every message shares (PlantUML's `AbstractMessage`).
pub(crate) struct MessageCommon {
    pub label: Display,
    pub arrow_configuration: ArrowConfiguration,
    pub life_event_types: Vec<LifeEventType>,
    pub url: Option<Url>,
    pub message_number: Option<String>,
    pub parallel: bool,
    pub parallel_brother: Option<EventId>,
    pub style_builder: Rc<StyleBuilder>,
    pub notes: Vec<Note>,
    pub stereotype: Option<Stereotype>,
    pub anchor: Option<String>,
    pub part1_anchor: Option<String>,
    pub part2_anchor: Option<String>,
    first_is_activate: bool,
    no_activation_authorized: Vec<ParticipantId>,
}

impl MessageCommon {
    pub(crate) fn new(
        label: Display,
        arrow_configuration: ArrowConfiguration,
        message_number: Option<String>,
        style_builder: Rc<StyleBuilder>,
    ) -> Self {
        Self {
            label,
            arrow_configuration,
            life_event_types: Vec::new(),
            url: None,
            message_number,
            parallel: false,
            parallel_brother: None,
            style_builder,
            notes: Vec::new(),
            stereotype: None,
            anchor: None,
            part1_anchor: None,
            part2_anchor: None,
            first_is_activate: false,
            no_activation_authorized: Vec::new(),
        }
    }

    fn has(&self, kind: LifeEventType) -> bool {
        self.life_event_types.contains(&kind)
    }

    pub(crate) fn is_create(&self) -> bool {
        self.has(LifeEventType::Create)
    }

    pub(crate) fn is_activate(&self) -> bool {
        self.has(LifeEventType::Activate)
    }

    pub(crate) fn is_deactivate(&self) -> bool {
        self.has(LifeEventType::Deactivate)
    }

    pub(crate) fn is_destroy(&self) -> bool {
        self.has(LifeEventType::Destroy)
    }

    /// Records a life event; activating a participant this message deactivated is refused.
    fn add_life_event(&mut self, kind: LifeEventType, participant: ParticipantId) -> bool {
        if !self.life_event_types.contains(&kind) {
            self.life_event_types.push(kind);
        }
        if self.life_event_types.len() == 1 && self.is_activate() {
            self.first_is_activate = true;
        }
        if kind == LifeEventType::Activate && self.no_activation_authorized.contains(&participant) {
            return false;
        }
        if matches!(kind, LifeEventType::Deactivate | LifeEventType::Destroy) {
            self.no_activation_authorized.push(participant);
        }
        true
    }
}

pub(crate) struct Message {
    pub common: MessageCommon,
    pub participant1: ParticipantId,
    pub participant2: ParticipantId,
    pub multicast: Vec<ParticipantId>,
}

impl Message {
    pub(crate) fn is_self_message(&self) -> bool {
        self.participant1 == self.participant2
    }
}

/// Messages from or to the diagram's border.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MessageExoType {
    FromLeft,
    ToLeft,
    FromRight,
    ToRight,
}

impl MessageExoType {
    pub(crate) fn direction(self) -> i32 {
        match self {
            Self::FromLeft | Self::ToRight => 1,
            Self::ToLeft | Self::FromRight => -1,
        }
    }

    pub(crate) fn is_left_border(self) -> bool {
        matches!(self, Self::FromLeft | Self::ToLeft)
    }

    pub(crate) fn is_right_border(self) -> bool {
        matches!(self, Self::FromRight | Self::ToRight)
    }

    pub(crate) fn reverse(self) -> Self {
        match self {
            Self::FromLeft => Self::ToLeft,
            Self::ToRight => Self::FromRight,
            Self::FromRight => Self::ToRight,
            Self::ToLeft => Self::FromLeft,
        }
    }
}

pub(crate) struct MessageExo {
    pub common: MessageCommon,
    pub participant: ParticipantId,
    pub kind: MessageExoType,
    pub short_arrow: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupingType {
    Start,
    StartPartition,
    Else,
    End,
}

impl GroupingType {
    pub(crate) fn named(name: &str) -> Option<Self> {
        Some(match name.to_lowercase().as_str() {
            "opt" | "alt" | "loop" | "par" | "par2" | "break" | "group" | "critical" => Self::Start,
            "partition" => Self::StartPartition,
            "also" | "else" => Self::Else,
            "end" => Self::End,
            _ => return None,
        })
    }

    pub(crate) fn is_start(self) -> bool {
        matches!(self, Self::Start | Self::StartPartition)
    }
}

pub(crate) struct GroupingStart {
    pub title: String,
    pub comment: Option<String>,
    pub kind: GroupingType,
    pub back_color_general: Option<HColor>,
    pub back_color_element: Option<HColor>,
    pub parallel: bool,
    pub style_builder: Rc<StyleBuilder>,
}

impl GroupingStart {
    pub(crate) fn is_par2(&self) -> bool {
        self.title == "par2"
    }

    pub(crate) fn is_parallel(&self) -> bool {
        self.parallel || self.is_par2()
    }
}

/// An `else` or `end` of a group.
pub(crate) struct GroupingLeaf {
    pub comment: Option<String>,
    pub kind: GroupingType,
    pub back_color_general: Option<HColor>,
    pub start: EventId,
    pub notes: Vec<Note>,
    pub style_builder: Rc<StyleBuilder>,
}

pub(crate) struct LifeEvent {
    pub participant: ParticipantId,
    pub kind: LifeEventType,
    pub colors: LiveColors,
    /// The message the event belongs to.
    pub message: Option<EventId>,
    pub style_builder: Rc<StyleBuilder>,
}

impl LifeEvent {
    pub(crate) fn is_activate(&self) -> bool {
        self.kind == LifeEventType::Activate
    }

    pub(crate) fn is_deactivate(&self) -> bool {
        self.kind == LifeEventType::Deactivate
    }

    pub(crate) fn is_deactivate_or_destroy(&self) -> bool {
        matches!(
            self.kind,
            LifeEventType::Deactivate | LifeEventType::Destroy
        )
    }
}

pub(crate) struct Reference {
    pub participants: Vec<ParticipantId>,
    pub display: Display,
    pub back_color_element: Option<HColor>,
    pub notes: Vec<Note>,
    pub style_builder: Rc<StyleBuilder>,
}

/// A labelled text with its style builder, as dividers and delays are.
pub(crate) struct Labelled {
    pub display: Display,
    pub style_builder: Rc<StyleBuilder>,
}

pub(crate) enum Event {
    Message(Message),
    MessageExo(MessageExo),
    Note(Note),
    /// Notes written one after the other with `/`, side by side.
    Notes(Vec<Note>),
    Divider(Labelled),
    Delay(Labelled),
    /// Vertical space, in pixels.
    HSpace(i32),
    GroupingStart(GroupingStart),
    GroupingLeaf(GroupingLeaf),
    LifeEvent(LifeEvent),
    Newpage(Rc<StyleBuilder>),
    Reference(Reference),
}

impl Event {
    pub(crate) fn deals_with(&self, participant: ParticipantId) -> bool {
        match self {
            Event::Message(message) => {
                message.participant1 == participant || message.participant2 == participant
            }
            Event::MessageExo(exo) => exo.participant == participant,
            Event::Note(note) => note.deals_with(participant),
            Event::Notes(notes) => notes.iter().any(|note| note.deals_with(participant)),
            Event::LifeEvent(life) => life.participant == participant,
            Event::Reference(reference) => reference.participants.contains(&participant),
            Event::Divider(_)
            | Event::Delay(_)
            | Event::HSpace(_)
            | Event::GroupingStart(_)
            | Event::GroupingLeaf(_)
            | Event::Newpage(_) => false,
        }
    }

    pub(crate) fn message_common(&self) -> Option<&MessageCommon> {
        match self {
            Event::Message(message) => Some(&message.common),
            Event::MessageExo(exo) => Some(&exo.common),
            _ => None,
        }
    }

    fn message_common_mut(&mut self) -> Option<&mut MessageCommon> {
        match self {
            Event::Message(message) => Some(&mut message.common),
            Event::MessageExo(exo) => Some(&mut exo.common),
            _ => None,
        }
    }

    /// Events that a following `activate`, `deactivate` or `destroy` attaches to (`EventWithDeactivate`).
    pub(crate) fn takes_deactivate(&self) -> bool {
        matches!(
            self,
            Event::Message(_) | Event::MessageExo(_) | Event::Divider(_) | Event::GroupingLeaf(_)
        )
    }

    /// Events that a note on a message attaches to (`EventWithNote`).
    pub(crate) fn takes_notes(&self) -> bool {
        matches!(
            self,
            Event::Message(_) | Event::MessageExo(_) | Event::GroupingLeaf(_) | Event::Reference(_)
        )
    }

    /// Attaches a life event; dividers and group ends take them silently.
    pub(crate) fn add_life_event(
        &mut self,
        kind: LifeEventType,
        participant: ParticipantId,
    ) -> bool {
        match self {
            Event::Message(message) => {
                if message.is_self_message() && participant != message.participant1 {
                    return true;
                }
                message.common.add_life_event(kind, participant)
            }
            Event::MessageExo(exo) => exo.common.add_life_event(kind, participant),
            _ => true,
        }
    }

    pub(crate) fn add_note(&mut self, note: Note) {
        match self {
            Event::Message(message) => {
                if matches!(
                    note.position,
                    NotePosition::Left
                        | NotePosition::Right
                        | NotePosition::Bottom
                        | NotePosition::Top
                ) {
                    message.common.notes.push(note);
                }
            }
            Event::MessageExo(exo) => {
                // Notes on border messages always go to the side facing the diagram.
                if matches!(
                    note.position,
                    NotePosition::Left
                        | NotePosition::Right
                        | NotePosition::Bottom
                        | NotePosition::Top
                ) {
                    let position = if exo.kind.is_left_border() {
                        NotePosition::Right
                    } else {
                        NotePosition::Left
                    };
                    exo.common.notes.push(Note { position, ..note });
                }
            }
            Event::GroupingLeaf(leaf) => leaf.notes.push(note),
            Event::Reference(reference) => {
                if matches!(note.position, NotePosition::Left | NotePosition::Right) {
                    reference.notes.push(note);
                }
            }
            _ => unreachable!("only events that take notes get them"),
        }
    }

    pub(crate) fn set_message_common(&mut self, change: impl FnOnce(&mut MessageCommon)) {
        if let Some(common) = self.message_common_mut() {
            change(common);
        }
    }
}

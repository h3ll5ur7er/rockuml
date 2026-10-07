//! Sequence diagrams: participants exchanging messages along their lifelines (PlantUML's
//! `sequencediagram` package), laid out by the teoz engine.

mod autonumber;
mod commands;
pub(crate) mod model;
mod styles;
mod teoz;

use std::rc::Rc;

use super::diagram_type::DiagramType;
use super::error::ErrorDiagram;
use super::titled::{Positioned, Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::{Colors, HColor};
use crate::command::{CommandError, CommandResult, factory};
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::stereo::Stereotype;
use crate::style::SName;
use autonumber::AutoNumber;
use model::{
    Event, EventId, GroupingLeaf, GroupingStart, GroupingType, LifeEvent, LifeEventType,
    LiveColors, Note, Participant, ParticipantEnglober, ParticipantId, ParticipantType,
};

// The flags are independent settings, each mirroring a field of PlantUML's `SequenceDiagram`.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct SequenceDiagram {
    source: UmlSource,
    titled: Titled,
    /// Every participant ever declared; `ParticipantId`s index it.
    participants: Vec<Participant>,
    /// The participants shown, in display order.
    order: Vec<ParticipantId>,
    /// The innermost box of each participant, by `ParticipantId`.
    englober_of: Vec<Option<usize>>,
    englobers: Vec<ParticipantEnglober>,
    events: Vec<Event>,
    /// The titles of the pages after the first.
    page_titles: Vec<Positioned>,
    last_event_with_deactivate: Option<EventId>,
    last_delay: Option<EventId>,
    pending_create: Option<EventId>,
    /// The messages that activated a lifeline still active, for `return`.
    activation_state: Vec<EventId>,
    /// The groups still open, innermost last.
    open_groupings: Vec<EventId>,
    autonumber: AutoNumber,
    show_footbox: bool,
    hide_unlinked: bool,
    ignore_newpage: bool,
    current_englober: Option<usize>,
    autoactivate: bool,
    link_anchors: Vec<LinkAnchor>,
}

/// `{start} <-> {end} : text`, a duration between two anchored messages.
pub(crate) struct LinkAnchor {
    pub anchor1: String,
    pub anchor2: String,
    pub message: Option<String>,
}

impl TitledDiagram for SequenceDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl SequenceDiagram {
    /// The diagram, the error image for its first faulty line, or nothing if the lines are no sequence
    /// diagram.
    pub(crate) fn create(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
        let mut diagram = Self {
            titled: Titled::new(SName::SequenceDiagram, "SEQUENCE", &source),
            source,
            participants: Vec::new(),
            order: Vec::new(),
            englober_of: Vec::new(),
            englobers: Vec::new(),
            events: Vec::new(),
            page_titles: Vec::new(),
            last_event_with_deactivate: None,
            last_delay: None,
            pending_create: None,
            activation_state: Vec::new(),
            open_groupings: Vec::new(),
            autonumber: AutoNumber::default(),
            show_footbox: true,
            hide_unlinked: false,
            ignore_newpage: false,
            current_englober: None,
            autoactivate: false,
            link_anchors: Vec::new(),
        };
        let lines = diagram.source.lines().to_vec();
        if let Err(failure) = factory::execute_lines(&lines, &mut diagram, &commands::commands()) {
            if failure.error.message == "Syntax Error?" {
                return Err(NotYetPorted("diagram types other than sequence"));
            }
            return Ok(Box::new(ErrorDiagram::new(
                diagram.source,
                failure.trace,
                &failure.error.message,
                Some(DiagramType::Sequence),
            )));
        }
        if diagram.hide_unlinked {
            diagram.remove_hidden_participants();
        }
        if diagram.order.is_empty() {
            return Err(NotYetPorted("diagram types other than sequence"));
        }
        Ok(Box::new(diagram))
    }

    pub(crate) fn participant(&self, id: ParticipantId) -> &Participant {
        &self.participants[id.0]
    }

    pub(crate) fn participants(&self) -> &[ParticipantId] {
        &self.order
    }

    pub(crate) fn events(&self) -> &[Event] {
        &self.events
    }

    fn find_participant(&self, code: &str) -> Option<ParticipantId> {
        self.order
            .iter()
            .copied()
            .find(|&id| self.participants[id.0].code == code)
    }

    pub(crate) fn contains_participant(&self, code: &str) -> bool {
        self.find_participant(code).is_some()
    }

    /// `getOrCreateParticipant`, with the name as display unless given.
    pub(crate) fn get_or_create_participant(
        &mut self,
        code: &str,
        display: Option<Display>,
    ) -> ParticipantId {
        if let Some(existing) = self.find_participant(code) {
            return existing;
        }
        let display = display.unwrap_or_else(|| Display::with_newlines(code));
        self.add_participant(ParticipantType::Participant, code, display, 0)
    }

    pub(crate) fn create_new_participant(
        &mut self,
        kind: ParticipantType,
        code: &str,
        display: Option<Display>,
        order: i32,
    ) -> ParticipantId {
        let display = display.unwrap_or_else(|| Display::with_newlines(code));
        self.add_participant(kind, code, display, order)
    }

    fn add_participant(
        &mut self,
        kind: ParticipantType,
        code: &str,
        display: Display,
        order: i32,
    ) -> ParticipantId {
        let id = ParticipantId(self.participants.len());
        self.participants.push(Participant {
            code: code.to_owned(),
            display,
            kind,
            initial_lives: Vec::new(),
            stereotype: None,
            colors: Colors::default(),
            url: None,
            order,
            style_builder: self.titled.skin.current_style_builder(),
        });
        self.englober_of.push(self.current_englober);
        self.add_with_order(id);
        id
    }

    pub(crate) fn participant_mut(&mut self, id: ParticipantId) -> &mut Participant {
        &mut self.participants[id.0]
    }

    /// Inserts before the first participant of a higher order.
    fn add_with_order(&mut self, id: ParticipantId) {
        let order = self.participants[id.0].order;
        let position = self
            .order
            .iter()
            .position(|other| order < self.participants[other.0].order)
            .unwrap_or(self.order.len());
        self.order.insert(position, id);
    }

    /// A participant declared again moves to the end of its order, into the current box.
    pub(crate) fn put_participant_in_last(&mut self, code: &str) {
        let id = self
            .find_participant(code)
            .expect("an existing participant");
        self.order.retain(|other| *other != id);
        self.add_with_order(id);
        self.englober_of[id.0] = self.current_englober;
    }

    pub(crate) fn englober_of(&self, id: ParticipantId) -> Option<usize> {
        self.englober_of[id.0]
    }

    pub(crate) fn englober(&self, index: usize) -> &ParticipantEnglober {
        &self.englobers[index]
    }

    fn last_event_index(&self, wanted: impl Fn(&Event) -> bool) -> Option<EventId> {
        self.events.iter().rposition(wanted)
    }

    pub(crate) fn add_message(&mut self, mut event: Event) -> CommandResult {
        let parallel = event.message_common().is_some_and(|common| common.parallel);
        if parallel {
            let brother = self.last_event_index(|event| event.message_common().is_some());
            event.set_message_common(|common| common.parallel_brother = brother);
        }
        let id = self.events.len();
        self.last_event_with_deactivate = Some(id);
        self.last_delay = None;
        self.events.push(event);
        if let Some(create) = self.pending_create.take() {
            let Event::LifeEvent(life) = &self.events[create] else {
                unreachable!("pending creations are life events");
            };
            let created = life.participant;
            let compatible = match &self.events[id] {
                Event::Message(message) => {
                    message.participant1 != created && message.participant2 == created
                }
                Event::MessageExo(exo) => exo.participant == created,
                _ => unreachable!("messages only"),
            };
            if !compatible {
                return Err(CommandError::new(format!(
                    "After create command, you have to send a message to \"{}\"",
                    self.participants[created.0].code
                )));
            }
            self.events[id].add_life_event(LifeEventType::Create, created);
            if let Event::LifeEvent(life) = &mut self.events[create] {
                life.message = Some(id);
            }
        }
        Ok(())
    }

    /// Notes written with `/` merge with the note right before them.
    pub(crate) fn add_note(&mut self, note: Note, try_merge: bool) {
        if try_merge {
            match self.events.last_mut() {
                Some(Event::Notes(notes)) => {
                    notes.push(note);
                    return;
                }
                Some(last @ Event::Note(_)) => {
                    let Event::Note(previous) = std::mem::replace(last, Event::HSpace(0)) else {
                        unreachable!("just matched a note");
                    };
                    *last = Event::Notes(vec![previous, note]);
                    return;
                }
                _ => {}
            }
        }
        self.events.push(Event::Note(note));
    }

    /// The last event a note on a message can attach to.
    pub(crate) fn last_event_with_note(&mut self) -> Option<&mut Event> {
        let index = self.last_event_index(Event::takes_notes)?;
        Some(&mut self.events[index])
    }

    pub(crate) fn last_event_with_deactivate(&self) -> Option<EventId> {
        self.last_event_index(Event::takes_deactivate)
    }

    pub(crate) fn event(&self, id: EventId) -> &Event {
        &self.events[id]
    }

    pub(crate) fn newpage(&mut self, title: Positioned) {
        if self.ignore_newpage {
            return;
        }
        self.page_titles.push(title);
        self.events
            .push(Event::Newpage(self.titled.skin.current_style_builder()));
    }

    pub(crate) fn ignore_newpage(&mut self) {
        self.ignore_newpage = true;
    }

    pub(crate) fn divider(&mut self, display: Display) {
        self.last_event_with_deactivate = Some(self.events.len());
        self.events.push(Event::Divider(model::Labelled {
            display,
            style_builder: self.titled.skin.current_style_builder(),
        }));
    }

    pub(crate) fn hspace(&mut self, pixels: i32) {
        self.events.push(Event::HSpace(pixels));
    }

    pub(crate) fn delay(&mut self, display: Display) {
        self.last_delay = Some(self.events.len());
        self.events.push(Event::Delay(model::Labelled {
            display,
            style_builder: self.titled.skin.current_style_builder(),
        }));
    }

    /// The message that activated the most recent lifeline still active.
    pub(crate) fn activating_message(&self) -> Option<EventId> {
        self.activation_state.last().copied()
    }

    /// `activate`, `deactivate`, `destroy` or `create`, attached to the last message when it concerns
    /// the participant. Returns PlantUML's error message when refused.
    pub(crate) fn activate(
        &mut self,
        participant: ParticipantId,
        kind: LifeEventType,
        colors: LiveColors,
    ) -> Result<(), String> {
        if self.last_delay.is_some() {
            return Err("You cannot Activate/Deactivate just after a ...".to_owned());
        }
        let life_id = self.events.len();
        self.events.push(Event::LifeEvent(LifeEvent {
            participant,
            kind,
            colors: colors.clone(),
            message: None,
            style_builder: self.titled.skin.current_style_builder(),
        }));
        if kind == LifeEventType::Create {
            self.pending_create = Some(life_id);
            return Ok(());
        }
        let Some(last) = self.last_event_with_deactivate else {
            if kind == LifeEventType::Activate {
                self.participants[participant.0].initial_lives.push(colors);
                return Ok(());
            }
            if self.participants[participant.0].initial_lives.is_empty() {
                return Err("You cannot deactivate here".to_owned());
            }
            return Ok(());
        };
        if !self.events[last].deals_with(participant) {
            return Ok(());
        }
        let is_message = self.events[last].message_common().is_some();
        if kind == LifeEventType::Activate && is_message {
            self.activation_state.push(last);
        } else if kind == LifeEventType::Deactivate {
            self.activation_state.pop();
        }
        let accepted = self.events[last].add_life_event(kind, participant);
        if is_message && let Event::LifeEvent(life) = &mut self.events[life_id] {
            life.message = Some(last);
        }
        if accepted {
            Ok(())
        } else {
            Err(format!(
                "Activate/Deactivate already done on {}",
                self.participants[participant.0].code
            ))
        }
    }

    /// Opens, continues or closes a group; returns false for an `else` or `end` outside any group.
    pub(crate) fn grouping(
        &mut self,
        title: &str,
        comment: Option<&str>,
        kind: GroupingType,
        back_color_general: Option<HColor>,
        back_color_element: Option<HColor>,
        parallel: bool,
    ) -> bool {
        if !kind.is_start() && self.open_groupings.is_empty() {
            return false;
        }
        let back_color_general = back_color_general.or_else(|| {
            self.titled
                .skin
                .value("sequencegroupbodybackgroundcolor")
                .map(|value| HColor::parse_or_white(&value))
        });
        let top = self.open_groupings.last().copied();
        let style_builder = self.titled.skin.current_style_builder();
        let id = self.events.len();
        if kind.is_start() {
            self.events.push(Event::GroupingStart(GroupingStart {
                title: title.to_owned(),
                comment: comment.map(str::to_owned),
                kind,
                back_color_general,
                back_color_element,
                parallel,
                style_builder,
            }));
            self.open_groupings.push(id);
        } else {
            self.events.push(Event::GroupingLeaf(GroupingLeaf {
                comment: comment.map(str::to_owned),
                kind,
                back_color_general,
                start: top.expect("checked above"),
                notes: Vec::new(),
                style_builder,
            }));
            if kind == GroupingType::End {
                self.open_groupings.pop();
                self.last_event_with_deactivate = Some(id);
            }
        }
        true
    }

    pub(crate) fn autonumber(&mut self) -> &mut AutoNumber {
        &mut self.autonumber
    }

    pub(crate) fn next_message_number(&mut self) -> Option<String> {
        self.autonumber.next_message_number()
    }

    /// `%autonumber%` in a label becomes the last message's number.
    pub(crate) fn manage_variable(&self, display: &Display) -> Display {
        display.replace("%autonumber%", &self.autonumber.current_message_number())
    }

    pub(crate) fn set_show_footbox(&mut self, show: bool) {
        self.show_footbox = show;
    }

    fn is_show_footbox(&self) -> bool {
        if self.titled.skin.strict_uml_style() {
            return false;
        }
        match self.titled.skin.value("footbox") {
            None => self.show_footbox,
            Some(footbox) => !footbox.eq_ignore_ascii_case("hide"),
        }
    }

    pub(crate) fn box_start(
        &mut self,
        title: Display,
        color: Option<HColor>,
        stereotype: Option<Stereotype>,
    ) {
        self.englobers.push(ParticipantEnglober {
            parent: self.current_englober,
            title,
            box_color: color,
            stereotype,
        });
        self.current_englober = Some(self.englobers.len() - 1);
    }

    /// Returns false when no box is open.
    pub(crate) fn end_box(&mut self) -> bool {
        match self.current_englober {
            None => false,
            Some(current) => {
                self.current_englober = self.englobers[current].parent;
                true
            }
        }
    }

    pub(crate) fn set_hide_unlinked(&mut self, hide: bool) {
        self.hide_unlinked = hide;
    }

    fn remove_hidden_participants(&mut self) {
        let events = &self.events;
        self.order
            .retain(|&id| events.iter().any(|event| event.deals_with(id)));
    }

    pub(crate) fn set_autoactivate(&mut self, autoactivate: bool) {
        self.autoactivate = autoactivate;
    }

    pub(crate) fn is_autoactivate(&self) -> bool {
        self.autoactivate
    }

    pub(crate) fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub(crate) fn link_anchor(&mut self, anchor: LinkAnchor) {
        self.link_anchors.push(anchor);
    }

    pub(crate) fn link_anchors(&self) -> &[LinkAnchor] {
        &self.link_anchors
    }

    pub(crate) fn style_builder(&self) -> Rc<crate::style::StyleBuilder> {
        self.titled.skin.current_style_builder()
    }

    /// `!pragma sequenceMessageSpan`: messages do not push participants apart.
    pub(crate) fn sequence_message_span(&self) -> bool {
        self.titled
            .pragma
            .is_true(super::titled::PragmaKey::SequenceMessageSpan)
    }

    pub(crate) fn skin(&self) -> &crate::skin::SkinParam {
        &self.titled.skin
    }

    /// Whether two messages were sent together with `&`.
    pub(crate) fn is_parallel_with(&self, message: EventId, other: EventId) -> bool {
        if message == other {
            return true;
        }
        match self.events[message]
            .message_common()
            .and_then(|common| common.parallel_brother)
        {
            None => false,
            Some(brother) => brother == other || self.is_parallel_with(brother, other),
        }
    }

    pub(crate) fn is_parallel(&self, id: EventId) -> bool {
        match &self.events[id] {
            Event::Message(message) => message.common.parallel,
            Event::MessageExo(exo) => exo.common.parallel,
            Event::Note(note) => note.parallel,
            Event::GroupingStart(start) => start.is_parallel(),
            Event::GroupingLeaf(leaf) => match &self.events[leaf.start] {
                Event::GroupingStart(start) => start.is_par2(),
                _ => unreachable!("leaves belong to group starts"),
            },
            _ => false,
        }
    }

    fn page_count(&self) -> usize {
        self.page_titles.len() + 1
    }
}

impl Diagram for SequenceDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn page_count(&self) -> usize {
        SequenceDiagram::page_count(self)
    }

    fn text_block(
        &self,
        page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let drawing = teoz::SequenceDiagramFileMakerTeoz::new(self, string_bounder, page)?;
        if page == 0 {
            return Ok(self.titled.add_chrome(Box::new(drawing), string_bounder));
        }
        let title = Some(&self.page_titles[page - 1]).filter(|title| !title.display.is_white());
        Ok(self
            .titled
            .add_chrome_titled(Box::new(drawing), string_bounder, title))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled.export_settings(self.source.seed(), 5.0)
    }
}

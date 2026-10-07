//! Notes: on their own, side by side, or attached to a message (PlantUML's `NoteTile`, `NotesTile`,
//! `CommunicationTileNoteLeft`, `...Right`, `...Top`, `...Bottom`, `CommunicationTileSelfNoteLeft` and
//! `...Right`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::communication::LIVE_DELTA_SIZE;
use super::components;
use super::living_space::{EventsHistoryMode, LivingSpace};
use super::self_tile::CommunicationTileSelf;
use super::tile::{Tile, TileArguments};
use super::y_gauge::YGauge;
use crate::diagram::sequence::model::{EventId, Note, NotePosition, ParticipantId};
use crate::diagram::sequence::styles::{merged, sequence_signature_root};
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::real::Real;
use crate::skin::component::{Area, Component, Context2D};
use crate::style::{PName, ValueReading};

/// A note on its own: beside, over, or across participants.
pub(super) struct NoteTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    note: &'a Note,
    participant1: ParticipantId,
    participant2: Option<ParticipantId>,
    y_gauge: YGauge,
    component: OnceCell<Box<dyn Component>>,
}

impl<'a> NoteTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        note: &'a Note,
        current_y: &YGauge,
    ) -> Self {
        let (participant1, participant2) = match (note.participant, note.participant2) {
            (None, None) => (
                arguments.living_spaces.first().participant,
                Some(arguments.living_spaces.last().participant),
            ),
            (participant1, participant2) => (
                participant1.expect("a note on participants names the first"),
                participant2,
            ),
        };
        let mut tile = Self {
            arguments,
            event,
            note,
            participant1,
            participant2,
            y_gauge: current_y.clone(),
            component: OnceCell::new(),
        };
        let contact_relative = tile.contact_point_relative();
        let height = tile.preferred_height();
        tile.y_gauge = if note.parallel {
            YGauge::create_parallel(current_y, contact_relative, height)
        } else {
            YGauge::create_with_contact(current_y, contact_relative, height)
        };
        tile
    }

    fn component(&self) -> &dyn Component {
        self.component
            .get_or_init(|| {
                components::note(
                    self.arguments.diagram,
                    self.note,
                    false,
                    self.note.position == NotePosition::OverSeveral,
                )
            })
            .as_ref()
    }

    fn living_space1(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.participant1)
    }

    fn living_space2(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(
            self.participant2
                .expect("notes over several participants have a second one"),
        )
    }

    /// Notes over several participants are at least as wide as the participants.
    fn used_width(&self) -> f64 {
        let string_bounder = self.arguments.string_bounder();
        let width = self.component().preferred_dimension(string_bounder).width;
        if self.note.position == NotePosition::OverSeveral {
            let span = self.living_space2().pos_d(string_bounder).current_value()
                - self.living_space1().pos_b().current_value();
            if width < span {
                return span;
            }
        }
        width
    }

    fn x(&self) -> Real {
        let string_bounder = self.arguments.string_bounder();
        let width = self.used_width();
        let pos_c = self.living_space1().pos_c(string_bounder);
        match self.note.position {
            NotePosition::Left => pos_c.add_fixed(-width),
            NotePosition::Right => {
                let level = self
                    .living_space1()
                    .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
                pos_c.add_fixed(level as f64 * LIVE_DELTA_SIZE)
            }
            NotePosition::OverSeveral => {
                Real::middle(&pos_c, &self.living_space2().pos_c(string_bounder))
                    .add_fixed(-width / 2.0)
            }
            NotePosition::Over => pos_c.add_fixed(-width / 2.0),
            NotePosition::Bottom | NotePosition::Top => {
                unreachable!("only notes on messages go above or below")
            }
        }
    }

    pub(super) fn left_anchor(&self) -> ParticipantId {
        self.participant1
    }

    /// How far the note reaches left of its participant's centre.
    pub(super) fn left_overhang(&self) -> f64 {
        match self.note.position {
            NotePosition::Left => self.used_width(),
            NotePosition::Over => self.used_width() / 2.0,
            _ => 0.0,
        }
    }

    /// Makes the participants' boxes clear the note, which may stick out past them.
    fn ensure_own_box_clears_note(&self) {
        let string_bounder = self.arguments.string_bounder();
        let overflow_left =
            self.living_space1().pos_b().current_value() - self.min_x().current_value();
        if overflow_left > 0.0 {
            self.living_space1().ensure_margin_before(overflow_left);
        }
        let right_anchor = if self.note.position == NotePosition::OverSeveral {
            self.living_space2()
        } else {
            self.living_space1()
        };
        let overflow_right =
            self.max_x().current_value() - right_anchor.pos_d(string_bounder).current_value();
        if overflow_right > 0.0 {
            right_anchor.ensure_margin_after(overflow_right);
        }
    }
}

impl<'a> Tile<'a> for NoteTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        self.component()
            .preferred_height(self.arguments.string_bounder())
            / 2.0
    }

    fn preferred_height(&self) -> f64 {
        self.component()
            .preferred_dimension(self.arguments.string_bounder())
            .height
    }

    fn add_constraints(&self) {
        self.ensure_own_box_clears_note();
    }

    fn min_x(&self) -> Real {
        let x = self.x();
        if self.note.position == NotePosition::OverSeveral {
            return Real::min(vec![x, self.living_space1().pos_b().clone()]);
        }
        x
    }

    fn max_x(&self) -> Real {
        let right = self.x().add_fixed(self.used_width());
        if self.note.position == NotePosition::OverSeveral {
            let pos_d = self.living_space2().pos_d(self.arguments.string_bounder());
            return Real::max(vec![right, pos_d]);
        }
        right
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let dimension = self.component().preferred_dimension(ug.string_bounder());
        let x = self.x().current_value();
        let area = Area::new(self.used_width(), dimension.height);
        self.component()
            .draw_u(&ug.translated(x, 0.0), &area, context);
    }

    fn stable_min_x(&self) -> Vec<Real> {
        let mut result = vec![self.x()];
        if self.note.position == NotePosition::OverSeveral {
            result.push(self.living_space1().pos_b().clone());
        }
        result
    }

    fn stable_max_x(&self) -> Vec<Real> {
        let mut result = vec![self.x().add_fixed(self.used_width())];
        if self.note.position == NotePosition::OverSeveral {
            result.push(self.living_space2().pos_d(self.arguments.string_bounder()));
        }
        result
    }

    fn as_note(&self) -> Option<&NoteTile<'a>> {
        Some(self)
    }
}

/// Notes merged with `/`, side by side on one row.
pub(super) struct NotesTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    notes: &'a [Note],
    y_gauge: YGauge,
    components: Vec<Box<dyn Component>>,
}

impl<'a> NotesTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        notes: &'a [Note],
        current_y: &YGauge,
    ) -> Self {
        let components = notes
            .iter()
            .map(|note| {
                components::note(
                    arguments.diagram,
                    note,
                    false,
                    note.position == NotePosition::OverSeveral,
                )
            })
            .collect();
        let mut tile = Self {
            arguments,
            event,
            notes,
            y_gauge: current_y.clone(),
            components,
        };
        let height = tile.preferred_height();
        tile.y_gauge = YGauge::create_with_contact(current_y, height / 2.0, height);
        tile
    }

    fn living_space(&self, participant: Option<ParticipantId>) -> &LivingSpace<'a> {
        // PlantUML crashes on notes across all participants merged with `/`; they go over the first here.
        participant.map_or_else(
            || self.arguments.living_spaces.first(),
            |participant| self.arguments.living_space(participant),
        )
    }

    fn used_width(&self, index: usize) -> f64 {
        self.components[index]
            .preferred_dimension(self.arguments.string_bounder())
            .width
    }

    /// How far the notes before on the same side of the same participant push this one.
    fn stacking_offset(&self, index: usize) -> f64 {
        let note = &self.notes[index];
        (0..index)
            .filter(|&other| {
                self.notes[other].position == note.position
                    && self.notes[other].participant == note.participant
            })
            .map(|other| self.used_width(other))
            .sum()
    }

    fn x_center(&self, index: usize) -> Real {
        self.living_space(self.notes[index].participant)
            .pos_c(self.arguments.string_bounder())
    }

    fn x(&self, index: usize) -> Real {
        let note = &self.notes[index];
        let string_bounder = self.arguments.string_bounder();
        let living_space1 = self.living_space(note.participant);
        let width = self.used_width(index);
        let pos_c = living_space1.pos_c(string_bounder);
        match note.position {
            NotePosition::Left => pos_c.add_fixed(-width - self.stacking_offset(index)),
            NotePosition::Right => {
                let level =
                    living_space1.level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
                pos_c.add_fixed(level as f64 * LIVE_DELTA_SIZE + self.stacking_offset(index))
            }
            NotePosition::OverSeveral => {
                let living_space2 = self.living_space(note.participant2);
                Real::middle(&pos_c, &living_space2.pos_c(string_bounder)).add_fixed(-width / 2.0)
            }
            NotePosition::Over => pos_c.add_fixed(-width / 2.0),
            NotePosition::Bottom | NotePosition::Top => {
                unreachable!("only notes on messages go above or below")
            }
        }
    }

    fn x2(&self, index: usize) -> Real {
        self.x(index).add_fixed(self.used_width(index))
    }
}

impl<'a> Tile<'a> for NotesTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        self.preferred_height() / 2.0
    }

    fn preferred_height(&self) -> f64 {
        let string_bounder = self.arguments.string_bounder();
        self.components
            .iter()
            .map(|component| component.preferred_dimension(string_bounder).height)
            .fold(0.0, f64::max)
    }

    fn add_constraints(&self) {
        for i in 0..self.notes.len() {
            for j in i + 1..self.notes.len() {
                let center1 = self.x_center(i).current_value();
                let center2 = self.x_center(j).current_value();
                if center1 == center2 {
                    continue;
                }
                if center2 > center1 {
                    self.x(j).ensure_bigger_than(&self.x2(i));
                } else {
                    self.x(i).ensure_bigger_than(&self.x2(j));
                }
            }
        }
    }

    fn min_x(&self) -> Real {
        Real::min((0..self.notes.len()).map(|index| self.x(index)).collect())
    }

    fn max_x(&self) -> Real {
        Real::max((0..self.notes.len()).map(|index| self.x2(index)).collect())
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        for (index, component) in self.components.iter().enumerate() {
            let dimension = component.preferred_dimension(ug.string_bounder());
            let x = self.x(index).current_value();
            let area = Area::new(self.used_width(index), dimension.height);
            component.draw_u(&ug.translated(x, 0.0), &area, context);
        }
    }
}

/// The gauge of a tile wrapped with a note: its top and arrow line stay, its bottom may go down.
fn wrapped_gauge(inner: &YGauge, height: f64) -> YGauge {
    YGauge {
        min: inner.min.clone(),
        max: inner.min.add_at_least(height),
        contact: inner.contact.clone(),
        origin: inner.origin.clone(),
    }
}

/// Which side of a message a note goes on.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Left,
    Right,
}

/// A note beside a message between participants or with the border.
pub(super) struct CommunicationTileNoteSide<'a> {
    arguments: Rc<TileArguments<'a>>,
    tile: Box<dyn Tile<'a> + 'a>,
    note: &'a Note,
    side: Side,
    participant: ParticipantId,
    /// The message creates the participant the note is beside.
    create: bool,
    y_gauge: YGauge,
    component: Box<dyn Component>,
}

impl<'a> CommunicationTileNoteSide<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        tile: Box<dyn Tile<'a> + 'a>,
        note: &'a Note,
        side: Side,
        participant: ParticipantId,
        create: bool,
    ) -> Self {
        let component = components::note(arguments.diagram, note, false, false);
        let mut wrapper = Self {
            y_gauge: tile.y_gauge().clone(),
            arguments,
            tile,
            note,
            side,
            participant,
            create,
            component,
        };
        wrapper.y_gauge = wrapped_gauge(wrapper.tile.y_gauge(), wrapper.preferred_height());
        wrapper
    }

    fn living_space(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.participant)
    }

    fn note_width(&self) -> f64 {
        self.component
            .preferred_dimension(self.arguments.string_bounder())
            .width
    }

    fn note_position(&self) -> Real {
        let string_bounder = self.arguments.string_bounder();
        let living_space = self.living_space();
        match self.side {
            Side::Left => living_space
                .pos_c(string_bounder)
                .add_fixed(-self.note_width()),
            Side::Right if self.create => living_space.pos_d(string_bounder),
            Side::Right => {
                let level =
                    living_space.level_at(self.event(), EventsHistoryMode::IgnoreFutureDeactivate);
                living_space
                    .pos_c(string_bounder)
                    .add_fixed(level as f64 * LIVE_DELTA_SIZE)
            }
        }
    }
}

impl<'a> Tile<'a> for CommunicationTileNoteSide<'a> {
    fn event(&self) -> EventId {
        self.tile.event()
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        self.tile.contact_point_relative()
    }

    fn preferred_height(&self) -> f64 {
        let note_height = self
            .component
            .preferred_dimension(self.arguments.string_bounder())
            .height;
        self.tile.preferred_height().max(note_height)
    }

    fn on_gauge_resolved(&self) {
        self.tile.on_gauge_resolved();
    }

    fn add_constraints(&self) {
        self.tile.add_constraints();
    }

    fn min_x(&self) -> Real {
        match self.side {
            Side::Left => self.note_position(),
            Side::Right => self.tile.min_x(),
        }
    }

    fn max_x(&self) -> Real {
        match self.side {
            Side::Left => self.tile.max_x(),
            Side::Right => self.note_position().add_fixed(self.note_width()),
        }
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        self.tile.draw_u(ug, context);
        let dimension = self.component.preferred_dimension(ug.string_bounder());
        let x = self.note_position().current_value();
        let ug = ug.translated(x, self.y_gauge.min.current_value());
        self.component
            .draw_u(&ug, &Area::new(dimension.width, dimension.height), context);
    }

    fn stable_min_x(&self) -> Vec<Real> {
        match self.side {
            Side::Left => vec![self.drawn_min_x()],
            Side::Right => Vec::new(),
        }
    }

    fn stable_max_x(&self) -> Vec<Real> {
        match self.side {
            Side::Left => Vec::new(),
            Side::Right => vec![self.drawn_max_x()],
        }
    }
}

/// Above or below a message, linked to it by a dotted line.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Level {
    Top,
    Bottom,
}

pub(super) struct CommunicationTileNoteLevel<'a> {
    arguments: Rc<TileArguments<'a>>,
    tile: Box<dyn Tile<'a> + 'a>,
    level: Level,
    y_gauge: YGauge,
    component: Box<dyn Component>,
}

/// Space between the message and its note.
const SPACE_Y: f64 = 10.0;
/// `Rose.paddingY`.
const ROSE_PADDING_Y: f64 = 5.0;

impl<'a> CommunicationTileNoteLevel<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        tile: Box<dyn Tile<'a> + 'a>,
        note: &'a Note,
        level: Level,
    ) -> Self {
        let component = components::note(arguments.diagram, note, true, false);
        let mut wrapper = Self {
            y_gauge: tile.y_gauge().clone(),
            arguments,
            tile,
            level,
            component,
        };
        wrapper.y_gauge = wrapped_gauge(wrapper.tile.y_gauge(), wrapper.preferred_height());
        wrapper
    }

    fn middle_message(&self) -> f64 {
        (self.tile.min_x().current_value() + self.tile.max_x().current_value()) / 2.0
    }

    /// The dotted line from the message to its note.
    fn draw_line(&self, ug: &UGraphic, (x1, y1): (f64, f64), (x2, y2): (f64, f64)) {
        let style = merged(
            &self.arguments.diagram.style_builder(),
            &sequence_signature_root(),
        );
        ug.translated(x1, y1)
            .with_color(style.value(PName::LineColor).as_color())
            .with_stroke(UStroke {
                dash_visible: 2.0,
                dash_space: 2.0,
                thickness: 1.0,
            })
            .draw(&UShape::Line {
                dx: x2 - x1,
                dy: y2 - y1,
            });
    }
}

impl<'a> Tile<'a> for CommunicationTileNoteLevel<'a> {
    fn event(&self) -> EventId {
        self.tile.event()
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        self.tile.contact_point_relative()
    }

    fn preferred_height(&self) -> f64 {
        let note_height = self
            .component
            .preferred_dimension(self.arguments.string_bounder())
            .height;
        self.tile.preferred_height() + note_height + SPACE_Y
    }

    fn on_gauge_resolved(&self) {
        self.tile.on_gauge_resolved();
    }

    fn add_constraints(&self) {
        self.tile.add_constraints();
    }

    fn min_x(&self) -> Real {
        self.tile.min_x()
    }

    fn max_x(&self) -> Real {
        let width = self
            .component
            .preferred_dimension(self.arguments.string_bounder())
            .width;
        Real::max(vec![self.tile.max_x(), self.tile.min_x().add_fixed(width)])
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let dimension = self.component.preferred_dimension(ug.string_bounder());
        let area = Area::new(dimension.width, dimension.height);
        let x_note = self.tile.min_x().current_value();
        let middle = self.middle_message();
        let contact = self.tile.contact_point_relative();
        match self.level {
            Level::Bottom => {
                self.tile.draw_u(ug, context);
                let ug = ug.translated(0.0, self.y_gauge.min.current_value());
                let y_note = self.tile.preferred_height();
                self.component
                    .draw_u(&ug.translated(x_note, y_note + SPACE_Y), &area, context);
                self.draw_line(
                    &ug,
                    (middle, contact),
                    (
                        x_note + dimension.width / 2.0,
                        y_note + SPACE_Y + ROSE_PADDING_Y,
                    ),
                );
            }
            Level::Top => {
                self.tile
                    .draw_u(&ug.translated(0.0, dimension.height + SPACE_Y), context);
                let ug = ug.translated(0.0, self.y_gauge.min.current_value());
                self.component
                    .draw_u(&ug.translated(x_note, 0.0), &area, context);
                self.draw_line(
                    &ug,
                    (middle, contact + dimension.height + SPACE_Y),
                    (
                        x_note + dimension.width / 2.0,
                        dimension.height - 2.0 * ROSE_PADDING_Y,
                    ),
                );
            }
        }
    }
}

/// A note beside a message to self; the participant's box clears it.
pub(super) struct CommunicationTileSelfNote<'a> {
    arguments: Rc<TileArguments<'a>>,
    tile: CommunicationTileSelf<'a>,
    side: Side,
    y_gauge: YGauge,
    component: Box<dyn Component>,
}

impl<'a> CommunicationTileSelfNote<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        tile: CommunicationTileSelf<'a>,
        note: &'a Note,
        side: Side,
    ) -> Self {
        let component = components::note(arguments.diagram, note, true, false);
        let mut wrapper = Self {
            y_gauge: tile.y_gauge().clone(),
            arguments,
            tile,
            side,
            component,
        };
        wrapper.y_gauge = wrapped_gauge(wrapper.tile.y_gauge(), wrapper.preferred_height());
        wrapper
    }

    fn note_width(&self) -> f64 {
        self.component
            .preferred_dimension(self.arguments.string_bounder())
            .width
    }

    fn note_position(&self) -> Real {
        match self.side {
            Side::Left => self.tile.min_x().add_fixed(-self.note_width()),
            Side::Right => self.tile.max_x(),
        }
    }
}

impl<'a> Tile<'a> for CommunicationTileSelfNote<'a> {
    fn event(&self) -> EventId {
        self.tile.event()
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        self.tile.contact_point_relative()
    }

    fn preferred_height(&self) -> f64 {
        let note_height = self
            .component
            .preferred_dimension(self.arguments.string_bounder())
            .height;
        self.tile.preferred_height().max(note_height)
    }

    fn on_gauge_resolved(&self) {
        self.tile.on_gauge_resolved();
    }

    fn add_constraints(&self) {
        self.tile.add_constraints();
        let string_bounder = self.arguments.string_bounder();
        let living_space = self.tile.living_space1();
        match self.side {
            Side::Left => {
                let overflow = living_space.pos_b().current_value() - self.min_x().current_value();
                if overflow > 0.0 {
                    living_space.ensure_margin_before(overflow);
                }
            }
            Side::Right => {
                let overflow = self.max_x().current_value()
                    - living_space.pos_d(string_bounder).current_value();
                if overflow > 0.0 {
                    living_space.ensure_margin_after(overflow);
                }
            }
        }
    }

    fn min_x(&self) -> Real {
        match self.side {
            Side::Left => self.note_position(),
            Side::Right => self.tile.min_x(),
        }
    }

    fn max_x(&self) -> Real {
        match self.side {
            Side::Left => self.tile.max_x(),
            Side::Right => self.note_position().add_fixed(self.note_width()),
        }
    }

    fn drawn_min_x(&self) -> Real {
        match self.side {
            Side::Left => self.tile.drawn_min_x().add_fixed(-self.note_width()),
            Side::Right => self.tile.drawn_min_x(),
        }
    }

    fn drawn_max_x(&self) -> Real {
        match self.side {
            Side::Left => self.tile.drawn_max_x(),
            Side::Right => self.tile.drawn_max_x().add_fixed(self.note_width()),
        }
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        self.tile.draw_u(ug, context);
        let dimension = self.component.preferred_dimension(ug.string_bounder());
        let x = self.note_position().current_value();
        let ug = ug.translated(x, self.y_gauge.min.current_value());
        self.component
            .draw_u(&ug, &Area::new(dimension.width, dimension.height), context);
    }

    fn stable_min_x(&self) -> Vec<Real> {
        match self.side {
            Side::Left => vec![self.drawn_min_x()],
            Side::Right => Vec::new(),
        }
    }

    fn stable_max_x(&self) -> Vec<Real> {
        match self.side {
            Side::Left => Vec::new(),
            Side::Right => vec![self.drawn_max_x()],
        }
    }
}

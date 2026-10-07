//! Tiles: one per event, each knowing its height, its horizontal extent as constrained positions, and how
//! to draw itself (PlantUML's `Tile`, `TileArguments` and `TileBuilder`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::communication::CommunicationTile;
use super::communication_exo::CommunicationExoTile;
use super::life_event::LifeEventTile;
use super::living_space::{LivingSpace, LivingSpaces};
use super::note_tiles::{
    CommunicationTileNoteLevel, CommunicationTileNoteSide, CommunicationTileSelfNote, Level,
    NoteTile, NotesTile, Side,
};
use super::self_tile::CommunicationTileSelf;
use super::span_tiles::{DelayTile, DividerTile, HSpaceTile, NewpageTile, ReferenceTile};
use super::y_gauge::YGauge;
use crate::diagram::NotYetPorted;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{
    Event, EventId, Message, MessageExo, NotePosition, ParticipantId,
};
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::Context2D;

pub(super) trait Tile<'a> {
    fn event(&self) -> EventId;

    fn y_gauge(&self) -> &YGauge;

    fn preferred_height(&self) -> f64;

    /// How far below its top the tile meets its message's arrow line, or -1.
    fn contact_point_relative(&self) -> f64 {
        -1.0
    }

    /// The height below the contact point.
    fn zzz(&self) -> f64 {
        self.preferred_height() - self.contact_point_relative()
    }

    /// Called once the vertical positions are solved, before each drawing pass.
    fn on_gauge_resolved(&self) {}

    fn add_constraints(&self);

    fn min_x(&self) -> Real;

    fn max_x(&self) -> Real;

    /// How far the drawing really reaches, which may be less than what the tile reserves.
    fn drawn_min_x(&self) -> Real {
        self.min_x()
    }

    fn drawn_max_x(&self) -> Real {
        self.max_x()
    }

    fn middle_x(&self) -> f64 {
        (self.min_x().current_value() + self.max_x().current_value()) / 2.0
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D);

    fn match_anchor(&self, anchor: &str) -> bool {
        let _ = anchor;
        false
    }

    /// Positions a group's frame must clear on the left, for the tiles that are drawn there.
    fn stable_min_x(&self) -> Vec<Real> {
        Vec::new()
    }

    fn stable_max_x(&self) -> Vec<Real> {
        Vec::new()
    }

    fn is_life_event(&self) -> bool {
        false
    }

    fn as_note(&self) -> Option<&NoteTile<'a>> {
        None
    }

    fn as_newpage(&self) -> Option<&NewpageTile<'a>> {
        None
    }
}

/// What every tile is built with (PlantUML's `TileArguments`).
pub(super) struct TileArguments<'a> {
    pub diagram: &'a SequenceDiagram,
    pub string_bounder: Rc<dyn StringBounder>,
    pub living_spaces: LivingSpaces<'a>,
    pub x_origin: Real,
    pub y_origin: Real,
    /// The diagram's left and right borders, known once the playing space is built.
    pub borders: OnceCell<(Real, Real)>,
}

impl<'a> TileArguments<'a> {
    pub(super) fn string_bounder(&self) -> &dyn StringBounder {
        self.string_bounder.as_ref()
    }

    pub(super) fn living_space(&self, participant: ParticipantId) -> &LivingSpace<'a> {
        self.living_spaces.get(participant)
    }

    pub(super) fn border1(&self) -> f64 {
        self.borders
            .get()
            .expect("borders are set before drawing")
            .0
            .current_value()
    }

    pub(super) fn border2(&self) -> f64 {
        self.borders
            .get()
            .expect("borders are set before drawing")
            .1
            .current_value()
    }
}

/// The tiles of the events from `events` on, each chained below the one before; groups take the events
/// up to their end.
pub(super) fn build_several<'a>(
    arguments: &Rc<TileArguments<'a>>,
    events: &mut std::iter::Peekable<impl Iterator<Item = EventId>>,
    current_y: YGauge,
) -> Result<Vec<Box<dyn Tile<'a> + 'a>>, NotYetPorted> {
    let mut tiles: Vec<Box<dyn Tile<'a> + 'a>> = Vec::new();
    let mut current_y = current_y;
    while let Some(event) = events.next() {
        if let Some(tile) = build_one(arguments, events, event, &current_y)? {
            current_y = tile.y_gauge().clone();
            tiles.push(tile);
        }
    }
    Ok(tiles)
}

pub(super) fn build_one<'a>(
    arguments: &Rc<TileArguments<'a>>,
    _events: &mut std::iter::Peekable<impl Iterator<Item = EventId>>,
    event: EventId,
    current_y: &YGauge,
) -> Result<Option<Box<dyn Tile<'a> + 'a>>, NotYetPorted> {
    let diagram = arguments.diagram;
    Ok(Some(match diagram.event(event) {
        Event::Message(message) => message_tile(arguments, event, message, current_y)?,
        Event::MessageExo(exo) => exo_tile(arguments, event, exo, current_y),
        Event::Note(note) => Box::new(NoteTile::new(arguments.clone(), event, note, current_y)),
        Event::Notes(notes) => Box::new(NotesTile::new(arguments.clone(), event, notes, current_y)),
        Event::Divider(divider) => Box::new(DividerTile::new(
            arguments.clone(),
            event,
            divider,
            current_y,
        )),
        Event::Delay(delay) => Box::new(DelayTile::new(arguments.clone(), event, delay, current_y)),
        Event::HSpace(pixels) => Box::new(HSpaceTile::new(
            arguments.clone(),
            event,
            *pixels,
            current_y,
        )),
        Event::GroupingStart(_) | Event::GroupingLeaf(_) => {
            return Err(NotYetPorted("sequence groups"));
        }
        Event::LifeEvent(life_event) => Box::new(LifeEventTile::new(
            arguments.clone(),
            event,
            life_event,
            current_y,
        )),
        Event::Newpage(style_builder) => Box::new(NewpageTile::new(
            arguments.clone(),
            event,
            style_builder,
            current_y,
        )),
        Event::Reference(reference) => Box::new(ReferenceTile::new(
            arguments.clone(),
            event,
            reference,
            current_y,
        )),
    }))
}

/// A message from or to the border, wrapped by a tile per note on it; PlantUML places every note where
/// the first one goes.
fn exo_tile<'a>(
    arguments: &Rc<TileArguments<'a>>,
    event: EventId,
    exo: &'a MessageExo,
    current_y: &YGauge,
) -> Box<dyn Tile<'a> + 'a> {
    let mut result: Box<dyn Tile<'a> + 'a> = Box::new(CommunicationExoTile::new(
        arguments.clone(),
        event,
        exo,
        current_y,
    ));
    let notes = &exo.common.notes;
    let side = match notes.first().map(|note| note.position) {
        Some(NotePosition::Left) => Side::Left,
        Some(NotePosition::Right) => Side::Right,
        _ => return result,
    };
    for note in notes {
        result = Box::new(CommunicationTileNoteSide::new(
            arguments.clone(),
            result,
            note,
            side,
            exo.participant,
            exo.common.is_create(),
        ));
    }
    result
}

/// A message's tile, wrapped by a tile per note on it.
fn message_tile<'a>(
    arguments: &Rc<TileArguments<'a>>,
    event: EventId,
    message: &'a Message,
    current_y: &YGauge,
) -> Result<Box<dyn Tile<'a> + 'a>, NotYetPorted> {
    let notes = &message.common.notes;
    if message.is_self_message() {
        let tile = CommunicationTileSelf::new(arguments.clone(), event, message, current_y);
        return match notes.as_slice() {
            [] => Ok(Box::new(tile)),
            [note] => Ok(match note.position {
                NotePosition::Left => Box::new(CommunicationTileSelfNote::new(
                    arguments.clone(),
                    tile,
                    note,
                    Side::Left,
                )),
                NotePosition::Right => Box::new(CommunicationTileSelfNote::new(
                    arguments.clone(),
                    tile,
                    note,
                    Side::Right,
                )),
                NotePosition::Top => Box::new(CommunicationTileNoteLevel::new(
                    arguments.clone(),
                    Box::new(tile),
                    note,
                    Level::Top,
                )),
                _ => Box::new(CommunicationTileNoteLevel::new(
                    arguments.clone(),
                    Box::new(tile),
                    note,
                    Level::Bottom,
                )),
            }),
            // PlantUML fails on these.
            _ => Err(NotYetPorted("several notes on a message to self")),
        };
    }
    let tile = CommunicationTile::new(arguments.clone(), event, message, current_y);
    let reverse = tile.is_reverse();
    let mut result: Box<dyn Tile<'a> + 'a> = Box::new(tile);
    for note in notes {
        result = match note.position {
            NotePosition::Left => Box::new(CommunicationTileNoteSide::new(
                arguments.clone(),
                result,
                note,
                Side::Left,
                if reverse {
                    message.participant2
                } else {
                    message.participant1
                },
                message.common.is_create(),
            )),
            NotePosition::Right => Box::new(CommunicationTileNoteSide::new(
                arguments.clone(),
                result,
                note,
                Side::Right,
                if reverse {
                    message.participant1
                } else {
                    message.participant2
                },
                message.common.is_create(),
            )),
            NotePosition::Top => Box::new(CommunicationTileNoteLevel::new(
                arguments.clone(),
                result,
                note,
                Level::Top,
            )),
            _ => Box::new(CommunicationTileNoteLevel::new(
                arguments.clone(),
                result,
                note,
                Level::Bottom,
            )),
        };
    }
    Ok(result)
}

//! Tiles: one per event, each knowing its height, its horizontal extent as constrained positions, and how
//! to draw itself (PlantUML's `Tile`, `TileArguments` and `TileBuilder`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::communication::CommunicationTile;
use super::communication_exo::CommunicationExoTile;
use super::grouping_tile::GroupingTile;
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
    Event, EventId, Message, MessageExo, Note, NotePosition, ParticipantId,
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
        f64::midpoint(self.min_x().current_value(), self.max_x().current_value())
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D);

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

    fn as_grouping(&self) -> Option<&GroupingTile<'a>> {
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

    fn borders(&self) -> &(Real, Real) {
        self.borders.get().expect("borders are set before drawing")
    }

    pub(super) fn border1(&self) -> f64 {
        self.borders().0.current_value()
    }

    pub(super) fn border2(&self) -> f64 {
        self.borders().1.current_value()
    }
}

/// The tiles of the events from `events` on, each chained below the one before; groups take the events
/// up to their end.
pub(super) fn build_several<'a>(
    arguments: &Rc<TileArguments<'a>>,
    events: &mut impl Iterator<Item = EventId>,
    current_y: YGauge,
) -> Result<Vec<Box<dyn Tile<'a> + 'a>>, NotYetPorted> {
    let mut tiles: Vec<Box<dyn Tile<'a> + 'a>> = Vec::new();
    let mut current_y = current_y;
    while let Some(event) = events.next() {
        let tile = build_one(arguments, events, event, &current_y)?;
        current_y = tile.y_gauge().clone();
        tiles.push(tile);
    }
    Ok(tiles)
}

pub(super) fn build_one<'a>(
    arguments: &Rc<TileArguments<'a>>,
    events: &mut impl Iterator<Item = EventId>,
    event: EventId,
    current_y: &YGauge,
) -> Result<Box<dyn Tile<'a> + 'a>, NotYetPorted> {
    let diagram = arguments.diagram;
    Ok(match diagram.event(event) {
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
        Event::GroupingStart(start) => Box::new(GroupingTile::new(
            arguments, events, event, start, current_y,
        )?),
        Event::GroupingLeaf(_) => unreachable!("groups take their own else and end"),
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
    })
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
    let Some(side) = notes.first().and_then(|note| side(note.position)) else {
        return result;
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
    if message.is_self_message() {
        return self_message_tile(arguments, event, message, current_y);
    }
    let tile = CommunicationTile::new(arguments.clone(), event, message, current_y);
    let reverse = tile.is_reverse();
    let mut result: Box<dyn Tile<'a> + 'a> = Box::new(tile);
    for note in &message.common.notes {
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
            _ => note_level_tile(arguments, result, note),
        };
    }
    Ok(result)
}

/// A message to self with its notes. Only the first note can go beside it: PlantUML fails on a later one.
fn self_message_tile<'a>(
    arguments: &Rc<TileArguments<'a>>,
    event: EventId,
    message: &'a Message,
    current_y: &YGauge,
) -> Result<Box<dyn Tile<'a> + 'a>, NotYetPorted> {
    let tile = CommunicationTileSelf::new(arguments.clone(), event, message, current_y);
    let notes = message.common.notes.as_slice();
    let beside = notes
        .first()
        .and_then(|first| side(first.position).map(|side| (first, side)));
    let (mut result, notes_above_or_below): (Box<dyn Tile<'a> + 'a>, _) = match beside {
        Some((first, side)) => (
            Box::new(CommunicationTileSelfNote::new(
                arguments.clone(),
                tile,
                first,
                side,
            )),
            &notes[1..],
        ),
        None => (Box::new(tile), notes),
    };
    for note in notes_above_or_below {
        if side(note.position).is_some() {
            return Err(NotYetPorted(
                "a note beside a message to self after another note",
            ));
        }
        result = note_level_tile(arguments, result, note);
    }
    Ok(result)
}

fn side(position: NotePosition) -> Option<Side> {
    match position {
        NotePosition::Left => Some(Side::Left),
        NotePosition::Right => Some(Side::Right),
        _ => None,
    }
}

/// A note above or below a message.
fn note_level_tile<'a>(
    arguments: &Rc<TileArguments<'a>>,
    tile: Box<dyn Tile<'a> + 'a>,
    note: &'a Note,
) -> Box<dyn Tile<'a> + 'a> {
    let level = if note.position == NotePosition::Top {
        Level::Top
    } else {
        Level::Bottom
    };
    Box::new(CommunicationTileNoteLevel::new(
        arguments.clone(),
        tile,
        note,
        level,
    ))
}

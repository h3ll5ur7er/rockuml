//! Groups (`alt`, `loop`, `group`, ... and `partition`): a frame around the tiles of their events, up to
//! their `end` (PlantUML's `GroupingTile` and `PartitionTile`).

use std::cell::OnceCell;
use std::collections::HashSet;
use std::rc::Rc;

use super::blotter::Blotter;
use super::components;
use super::else_tile::ElseTile;
use super::note_tiles::{NoteTile, Side};
use super::tile::{Tile, TileArguments, build_one};
use super::y_gauge::YGauge;
use crate::creole::Display;
use crate::diagram::NotYetPorted;
use crate::diagram::sequence::model::{
    Event, EventId, GroupingLeaf, GroupingStart, GroupingType, Note, NotePosition, ParticipantId,
};
use crate::diagram::sequence::styles::grouping_start_styles;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::real::Real;
use crate::skin::component::{Area, Component, Context2D, creole_text};
use crate::style::{PName, Style, ValueReading};

/// How far the frame reaches beyond the room its tiles take, left and right.
const EXTERNAL_MARGINX1: f64 = 3.0;
const EXTERNAL_MARGINX2: f64 = 9.0;
/// The room above and below the frame.
const EXTERNAL_MARGINY: f64 = 4.0;
/// How far the frame is from what it contains.
const MARGINX: f64 = 16.0;
const MARGINY_MAGIC: f64 = 20.0;
/// The room above and below a partition's title.
const TITLE_VPAD: f64 = 4.0;

/// A group's frame, which its else tiles span but which is only known once they are built.
#[derive(Default)]
pub(super) struct GroupingFrame {
    min: OnceCell<Real>,
    max: OnceCell<Real>,
    /// The left of the room the group takes, its notes included.
    min_x: OnceCell<Real>,
}

impl GroupingFrame {
    pub(super) fn min(&self) -> &Real {
        self.min
            .get()
            .expect("the frame is known once the group is built")
    }

    pub(super) fn max(&self) -> &Real {
        self.max
            .get()
            .expect("the frame is known once the group is built")
    }

    pub(super) fn min_x(&self) -> Real {
        self.min_x
            .get()
            .expect("the frame is known once the group is built")
            .clone()
    }
}

pub(super) struct GroupingTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    start: &'a GroupingStart,
    /// None for a group left open at the end of the diagram.
    end: Option<&'a GroupingLeaf>,
    tiles: Vec<Box<dyn Tile<'a> + 'a>>,
    frame: Rc<GroupingFrame>,
    body_height: f64,
    y_gauge: YGauge,
    component: OnceCell<Box<dyn Component + 'a>>,
}

impl<'a> GroupingTile<'a> {
    /// The group and the tiles of its events, which it takes from `events` up to its end.
    pub(super) fn new(
        arguments: &Rc<TileArguments<'a>>,
        events: &mut impl Iterator<Item = EventId>,
        event: EventId,
        start: &'a GroupingStart,
        current_y: &YGauge,
    ) -> Result<Self, NotYetPorted> {
        let diagram = arguments.diagram;
        let mut tile = Self {
            arguments: arguments.clone(),
            event,
            start,
            end: None,
            tiles: Vec::new(),
            frame: Rc::new(GroupingFrame::default()),
            body_height: 0.0,
            y_gauge: current_y.clone(),
            component: OnceCell::new(),
        };
        let dim1 = tile.preferred_dimension_if_empty();
        // A parallel group starts beside the tile before, not below it.
        let previous_max = current_y.max.clone();
        let parallel = start.is_parallel();
        let first_y = if parallel {
            current_y.min.clone()
        } else {
            previous_max.clone()
        };
        let header = dim1.height + MARGINY_MAGIC / 2.0 + EXTERNAL_MARGINY;
        let mut current_y = YGauge::create(&first_y.add_at_least(header), 0.0);
        while let Some(child) = events.next() {
            let child_tile: Box<dyn Tile<'a> + 'a> = match diagram.event(child) {
                Event::GroupingLeaf(leaf) if leaf.kind == GroupingType::End => {
                    tile.end = Some(leaf);
                    break;
                }
                Event::GroupingLeaf(leaf) => Box::new(ElseTile::new(
                    arguments.clone(),
                    child,
                    leaf,
                    tile.frame.clone(),
                    &current_y,
                )),
                _ => build_one(arguments, events, child, &current_y)?,
            };
            current_y = child_tile.y_gauge().clone();
            tile.tiles.push(child_tile);
        }
        tile.body_height = tile.compute_body_height();
        tile.set_frame(dim1.width);
        let height = tile.preferred_height();
        tile.y_gauge = if parallel {
            let parallel_max = first_y.add_at_least(height);
            parallel_max.ensure_bigger_than(&previous_max);
            YGauge::new(first_y, parallel_max)
        } else {
            YGauge::new(first_y.clone(), first_y.add_at_least(height))
        };
        Ok(tile)
    }

    /// The frame spans its tiles, its else lines and its title.
    fn set_frame(&self, title_width: f64) {
        let mut min2 = Vec::new();
        let mut max2 = Vec::new();
        let mut elses = Vec::new();
        for tile in &self.tiles {
            match self.arguments.diagram.event(tile.event()) {
                Event::GroupingLeaf(_) => elses.push(tile),
                // Spacing reports the diagram's origin, which would stretch the frame to the left edge.
                Event::HSpace(_) => {}
                _ => {
                    min2.push(tile.drawn_min_x().add_fixed(-MARGINX));
                    max2.push(tile.drawn_max_x().add_fixed(MARGINX));
                }
            }
        }
        if min2.is_empty() {
            min2.push(self.arguments.x_origin.clone());
        }
        let min = Real::min(min2);
        let left_notes = self.notes_width(Side::Left);
        let _ = self
            .frame
            .min_x
            .set(min.add_fixed(-EXTERNAL_MARGINX1 - left_notes));
        max2.extend(elses.iter().map(|tile| tile.max_x()));
        max2.push(min.add_fixed(title_width + 16.0));
        let _ = self.frame.min.set(min);
        let _ = self.frame.max.set(Real::max(max2));
    }

    fn string_bounder(&self) -> &dyn StringBounder {
        self.arguments.string_bounder()
    }

    fn is_partition(&self) -> bool {
        self.start.kind == GroupingType::StartPartition
    }

    fn component(&self) -> &dyn Component {
        self.component
            .get_or_init(|| {
                if self.is_partition() {
                    Box::new(PartitionHeader::new(self.arguments.clone(), self.start))
                } else {
                    components::grouping_header(self.start)
                }
            })
            .as_ref()
    }

    fn preferred_dimension_if_empty(&self) -> XDimension2D {
        self.component().preferred_dimension(self.string_bounder())
    }

    /// The frame's top, below the room kept above it; parallel groups chain on the gauge's top instead.
    fn frame_y(&self) -> f64 {
        self.y_gauge.min.current_value() + EXTERNAL_MARGINY
    }

    fn total_height(&self) -> f64 {
        self.body_height + self.preferred_dimension_if_empty().height + MARGINY_MAGIC / 2.0
    }

    /// A partition spans the whole diagram.
    fn area(&self) -> Area {
        let width = if self.is_partition() {
            self.arguments.border2() - self.arguments.border1()
        } else {
            self.frame.max().current_value() - self.frame.min().current_value()
        };
        Area::new(width, self.total_height())
    }

    fn min_for_drawing(&self) -> f64 {
        if self.is_partition() {
            0.0
        } else {
            self.frame.min().current_value()
        }
    }

    fn draw_background(&self, ug: &UGraphic, area: &Area) {
        let (style, _) = grouping_start_styles(self.start);
        let back = style.value(PName::BackGroundColor).as_color();
        let round = style.value(PName::RoundCorner).as_double();
        let ug = ug.translated(0.0, self.frame_y());
        if self.is_partition() {
            ug.translated(self.arguments.border1(), 0.0)
                .with_backcolor(back)
                .draw(&UShape::Rectangle(
                    URectangle::new(area.dimension.width, area.dimension.height).rounded(round),
                ));
            return;
        }
        let mut blotter = Blotter::new(area.dimension, back.clone(), round);
        for tile in &self.tiles {
            if let Event::GroupingLeaf(leaf) = self.arguments.diagram.event(tile.event()) {
                let y = tile.y_gauge().min.current_value() - self.frame_y();
                let back_else = leaf
                    .back_color_general
                    .as_ref()
                    .or(self.start.back_color_general.as_ref())
                    .cloned()
                    .unwrap_or_else(|| back.clone());
                blotter.add_change(y + 1.0, back_else);
            }
        }
        blotter.close_changes();
        blotter.draw_u(&ug.translated(self.frame.min().current_value(), 0.0));
    }

    /// The notes after the group's `end`, at the top corners of its frame.
    fn note_components(&self, side: Side) -> Vec<Box<dyn Component>> {
        let Some(end) = self.end else {
            return Vec::new();
        };
        end.notes
            .iter()
            .filter(|note| (note.position == NotePosition::Left) == (side == Side::Left))
            .map(|note| components::note(self.arguments.diagram, note, false, false))
            .collect()
    }

    fn notes_width(&self, side: Side) -> f64 {
        self.note_components(side)
            .iter()
            .map(|note| note.preferred_width(self.string_bounder()))
            .sum()
    }

    fn draw_notes(&self, ug: &UGraphic, context: Context2D) {
        let string_bounder = ug.string_bounder();
        let mut x_right = self.frame.max().current_value();
        for note in self.note_components(Side::Right) {
            let dimension = note.preferred_dimension(string_bounder);
            note.draw_u(
                &ug.translated(x_right, 0.0),
                &Area::new(dimension.width, dimension.height),
                context,
            );
            x_right += dimension.width;
        }
        let mut x_left = self.frame.min().current_value();
        for note in self.note_components(Side::Left) {
            let dimension = note.preferred_dimension(string_bounder);
            note.draw_u(
                &ug.translated(x_left - dimension.width, 0.0),
                &Area::new(dimension.width, dimension.height),
                context,
            );
            x_left -= dimension.width;
        }
    }

    /// The height of the tiles, counting parallel tiles once: they share their arrow line.
    fn compute_body_height(&self) -> f64 {
        let mut total = 0.0;
        let mut pending: Vec<&dyn Tile<'a>> = Vec::new();
        let mut cluster_open = false;
        for tile in &self.tiles {
            if self.arguments.diagram.is_parallel(tile.event()) {
                cluster_open = true;
            } else if !tile.is_life_event() {
                total += flush_pending(&mut pending, cluster_open);
                cluster_open = false;
            }
            pending.push(tile.as_ref());
        }
        total + flush_pending(&mut pending, cluster_open)
    }

    pub(super) fn tiles(&self) -> &[Box<dyn Tile<'a> + 'a>] {
        &self.tiles
    }

    /// Keeps the participant right after the group's out of its frame, which the tiles inside know
    /// nothing of.
    fn ensure_following_participant_clears_frame(&self) {
        let living_spaces = &self.arguments.living_spaces;
        let touched = touched_participants(&self.arguments, &self.tiles);
        let Some(rightmost) = living_spaces
            .values()
            .iter()
            .rfind(|space| touched.contains(&space.participant))
        else {
            return;
        };
        let Some(next) = living_spaces.next(rightmost.participant) else {
            return;
        };
        let next_pos_a = next.pos_a();
        let frame_margin = MARGINX + EXTERNAL_MARGINX2;
        next_pos_a.ensure_bigger_than(
            &rightmost
                .pos_c(self.string_bounder())
                .add_fixed(frame_margin),
        );
        self.ensure_clears_frame_of(&next_pos_a, frame_margin);
    }

    /// Keeps the participant right before the group's out of its frame.
    fn ensure_preceding_participant_clears_frame(&self) {
        let living_spaces = &self.arguments.living_spaces;
        let touched = touched_participants(&self.arguments, &self.tiles);
        let Some(leftmost) = living_spaces
            .values()
            .iter()
            .find(|space| touched.contains(&space.participant))
        else {
            return;
        };
        let Some(previous) = living_spaces.previous(leftmost.participant) else {
            return;
        };
        let previous_pos_c = previous.pos_c(self.string_bounder());
        let frame_margin = MARGINX + EXTERNAL_MARGINX2;
        leftmost
            .pos_c(self.string_bounder())
            .ensure_bigger_than(&previous_pos_c.add_fixed(frame_margin));
        self.ensure_clears_frame_of_left(&previous_pos_c, frame_margin);
    }

    /// Pushes every left edge this frame is built on past `previous_pos_c`. Each nested frame adds its own
    /// margins.
    fn ensure_clears_frame_of_left(&self, previous_pos_c: &Real, margin: f64) {
        for tile in &self.tiles {
            if let Some(nested) = tile.as_grouping() {
                let notes = nested.notes_width(Side::Left);
                nested.ensure_clears_frame_of_left(
                    previous_pos_c,
                    margin + MARGINX + EXTERNAL_MARGINX2 + notes,
                );
                continue;
            }
            for min_x in tile.stable_min_x() {
                min_x.ensure_bigger_than(&previous_pos_c.add_fixed(margin));
            }
        }
    }

    /// Pushes `next_pos_a` past every right edge this frame is built on. Only positions that never cache
    /// their value can be pushed against: a max read too early would keep a stale value.
    fn ensure_clears_frame_of(&self, next_pos_a: &Real, margin: f64) {
        self.ensure_clears_title_of(next_pos_a, margin);
        for tile in &self.tiles {
            if let Some(nested) = tile.as_grouping() {
                let notes = nested.notes_width(Side::Right);
                nested.ensure_clears_frame_of(
                    next_pos_a,
                    margin + MARGINX + EXTERNAL_MARGINX2 + notes,
                );
                continue;
            }
            for max_x in tile.stable_max_x() {
                next_pos_a.ensure_bigger_than(&max_x.add_fixed(margin));
            }
        }
    }

    /// A title wider than the group's content stretches the frame; its left edge is found again from the
    /// leftmost participant, less what notes there reach further left.
    fn ensure_clears_title_of(&self, next_pos_a: &Real, margin: f64) {
        let touched = touched_participants(&self.arguments, &self.tiles);
        let Some(leftmost) = self
            .arguments
            .living_spaces
            .values()
            .iter()
            .find(|space| touched.contains(&space.participant))
        else {
            return;
        };
        let overhang = self
            .tiles
            .iter()
            .filter_map(|tile| tile.as_note())
            .filter(|note| note.left_anchor() == leftmost.participant)
            .map(NoteTile::left_overhang)
            .fold(0.0, f64::max);
        let width = self.preferred_dimension_if_empty().width;
        next_pos_a.ensure_bigger_than(
            &leftmost
                .pos_c(self.string_bounder())
                .add_fixed(width + 16.0 - MARGINX - MARGINX - overhang + margin),
        );
    }
}

/// The height of a run of tiles: parallel ones overlap on their arrow line.
fn flush_pending(pending: &mut Vec<&dyn Tile<'_>>, was_cluster: bool) -> f64 {
    let result = if was_cluster {
        let contact = pending
            .iter()
            .map(|tile| tile.contact_point_relative())
            .fold(0.0, f64::max);
        let after = pending.iter().map(|tile| tile.zzz()).fold(0.0, f64::max);
        contact + after
    } else {
        pending.iter().map(|tile| tile.preferred_height()).sum()
    };
    pending.clear();
    result
}

/// The participants the tiles (and the tiles of groups among them) draw on, which lie inside the frame.
fn touched_participants(
    arguments: &TileArguments<'_>,
    tiles: &[Box<dyn Tile<'_> + '_>],
) -> HashSet<ParticipantId> {
    let mut touched = HashSet::new();
    collect_touched_participants(arguments, tiles, &mut touched);
    touched
}

fn collect_touched_participants(
    arguments: &TileArguments<'_>,
    tiles: &[Box<dyn Tile<'_> + '_>],
    touched: &mut HashSet<ParticipantId>,
) {
    for tile in tiles {
        match arguments.diagram.event(tile.event()) {
            Event::Message(message) => {
                touched.insert(message.participant1);
                touched.insert(message.participant2);
            }
            Event::MessageExo(exo) => {
                touched.insert(exo.participant);
            }
            Event::LifeEvent(life_event) => {
                touched.insert(life_event.participant);
            }
            Event::Note(note) => add_note_touched(arguments, note, touched),
            Event::Notes(notes) => {
                for note in notes {
                    add_note_touched(arguments, note, touched);
                }
            }
            Event::Reference(reference) => touched.extend(&reference.participants),
            _ => {}
        }
        if let Some(group) = tile.as_grouping() {
            collect_touched_participants(arguments, group.tiles(), touched);
        }
    }
}

/// A note across the diagram touches every participant.
fn add_note_touched(
    arguments: &TileArguments<'_>,
    note: &Note,
    touched: &mut HashSet<ParticipantId>,
) {
    if note.participant.is_none() && note.participant2.is_none() {
        touched.extend(
            arguments
                .living_spaces
                .values()
                .iter()
                .map(|space| space.participant),
        );
        return;
    }
    touched.extend(note.participant);
    touched.extend(note.participant2);
}

impl<'a> Tile<'a> for GroupingTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        0.0
    }

    fn preferred_height(&self) -> f64 {
        self.preferred_dimension_if_empty().height
            + self.body_height
            + MARGINY_MAGIC
            + 2.0 * EXTERNAL_MARGINY
    }

    fn on_gauge_resolved(&self) {
        for tile in &self.tiles {
            tile.on_gauge_resolved();
        }
    }

    fn add_constraints(&self) {
        for tile in &self.tiles {
            tile.add_constraints();
        }
        self.ensure_following_participant_clears_frame();
        self.ensure_preceding_participant_clears_frame();
    }

    fn min_x(&self) -> Real {
        self.frame.min_x()
    }

    fn max_x(&self) -> Real {
        self.frame
            .max()
            .add_fixed(EXTERNAL_MARGINX2 + self.notes_width(Side::Right))
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let area = self.area();
        if context.is_background {
            self.draw_background(ug, &area);
        }
        self.component().draw_u(
            &ug.translated(self.min_for_drawing(), self.frame_y()),
            &area,
            context,
        );
        self.draw_notes(&ug.translated(0.0, self.frame_y()), context);
        for tile in &self.tiles {
            tile.draw_u(ug, context);
        }
    }

    fn as_grouping(&self) -> Option<&GroupingTile<'a>> {
        Some(self)
    }
}

/// A partition's title, centred over the whole diagram, and its frame (PlantUML's `PartitionTile`
/// component). Unlike a group's header it draws alike in the background and the foreground.
struct PartitionHeader<'a> {
    arguments: Rc<TileArguments<'a>>,
    title: Box<dyn TextBlock>,
    frame_style: Style,
}

impl<'a> PartitionHeader<'a> {
    fn new(arguments: Rc<TileArguments<'a>>, start: &GroupingStart) -> Self {
        let (frame_style, header) = grouping_start_styles(start);
        let display = Display::create([start.comment.clone().unwrap_or_default()]);
        let title = creole_text(
            display.lines(),
            header.font_configuration(),
            HorizontalAlignment::Left,
            0.0,
        );
        Self {
            arguments,
            title,
            frame_style,
        }
    }
}

impl Component for PartitionHeader<'_> {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.title.calculate_dimension(string_bounder).width
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.title.calculate_dimension(string_bounder).height + 2.0 * TITLE_VPAD
    }

    fn draw_u(&self, ug: &UGraphic, area: &Area, _context: Context2D) {
        self.draw_internal(ug, area);
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let border1 = self.arguments.border1();
        let width = self.arguments.border2() - border1;
        let delta = (width - self.preferred_width(ug.string_bounder())) / 2.0;
        self.title
            .draw_u(&ug.translated(border1 + delta, TITLE_VPAD));
        let round = self.frame_style.value(PName::RoundCorner).as_double();
        ug.with_color(self.frame_style.value(PName::LineColor).as_color())
            .with_stroke(self.frame_style.stroke())
            .translated(border1, 0.0)
            .draw(&UShape::Rectangle(
                URectangle::new(area.dimension.width, area.dimension.height).rounded(round),
            ));
    }
}

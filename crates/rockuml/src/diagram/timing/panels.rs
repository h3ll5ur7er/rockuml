//! How each kind of player draws itself: the labels left of the time grid and the signal over it
//! (PlantUML's `Panels` and its subclasses, with the shapes of `timingdiagram.graphic`).

use super::player::{
    AbstractStatePlayer, ChangeState, HIGH_STRING, LOW_STRING, NotePosition, Player, PlayerAnalog,
    PlayerBinary, PlayerClock, PlayerKind, TimeConstraint, TimingNote, TimingStyle, is_flat,
};
use super::ruler::{TimingRuler, apply_for_vlines};
use super::time::TimeTick;
use crate::color::{ColorType, Colors};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XPoint2D;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, Style, ValueReading};

const MARGIN_Y: f64 = 8.0;
const MARGIN_X: f64 = 12.0;
const BOTTOM_MARGIN: f64 = 10.0;
const LEFT_PANEL_MIN_WIDTH: f64 = 5.0;
/// How far the slanted ends of a state reach into it.
const DELTA: f64 = 12.0;

/// Where a message to or from a player starts or ends at a time: two points where the signal changes,
/// the one closer to the other end being used.
#[derive(Clone, Copy)]
pub(super) struct IntricatedPoint {
    pub(super) a: XPoint2D,
    pub(super) b: XPoint2D,
}

impl IntricatedPoint {
    fn single(point: XPoint2D) -> Self {
        Self { a: point, b: point }
    }

    pub(super) fn translated(self, dy: f64) -> Self {
        Self {
            a: self.a.move_by(0.0, dy),
            b: self.b.move_by(0.0, dy),
        }
    }
}

pub(super) trait Panels {
    fn draw_left_panel(&self, ug: &UGraphic, full_available_width: f64);
    fn draw_right_panel(&self, ug: &UGraphic);
    fn get_full_height(&self, string_bounder: &dyn StringBounder) -> f64;
    fn get_left_panel_width(&self, string_bounder: &dyn StringBounder) -> f64;
    fn get_time_projection(
        &self,
        string_bounder: &dyn StringBounder,
        tick: &TimeTick,
    ) -> Option<IntricatedPoint>;
}

/// The panels of a player, as it draws itself now.
pub(super) fn panels<'a>(
    player: &'a Player,
    ruler: &'a TimingRuler,
    skin: &'a SkinParam,
) -> Box<dyn Panels + 'a> {
    let base = |notes: &'a [TimingNote], constraints: &'a [TimeConstraint]| PanelsBase {
        ruler,
        skin,
        suggested_height: player.suggested_height,
        style: player.get_style(skin),
        notes,
        constraints,
    };
    match &player.kind {
        PlayerKind::State(state) if state.style == TimingStyle::Robust => {
            Box::new(PanelsRobust::new(base(&[], &state.constraints), state))
        }
        PlayerKind::State(state) => Box::new(PanelsState::new(
            base(&player.notes, &state.constraints),
            state,
        )),
        PlayerKind::Clock(clock) => Box::new(PanelsClock {
            base: base(&[], &[]),
            clock,
        }),
        PlayerKind::Binary(binary) => Box::new(PanelsBinary {
            base: base(&player.notes, &binary.constraints),
            binary,
        }),
        PlayerKind::Analog(analog) => Box::new(PanelsAnalog {
            base: base(&[], &analog.constraints),
            analog,
        }),
    }
}

struct PanelsBase<'a> {
    ruler: &'a TimingRuler,
    skin: &'a SkinParam,
    suggested_height: i32,
    style: Style,
    notes: &'a [TimingNote],
    constraints: &'a [TimeConstraint],
}

impl PanelsBase<'_> {
    fn suggested_height(&self) -> f64 {
        f64::from(self.suggested_height)
    }

    fn draw_constraints(&self, ug: &UGraphic, delta_y: impl Fn(&TimeConstraint) -> f64) {
        for constraint in self.constraints {
            constraint.draw_u(
                &ug.translated(0.0, delta_y(constraint)),
                self.ruler,
                self.skin,
            );
        }
    }

    fn get_height_for_constraints(
        &self,
        string_bounder: &dyn StringBounder,
        delta_y: impl Fn(&TimeConstraint) -> f64,
    ) -> f64 {
        self.constraints.iter().fold(0.0, |result, constraint| {
            f64::max(
                result,
                constraint.get_constraint_height(string_bounder, self.skin) - delta_y(constraint),
            )
        })
    }

    fn draw_notes(&self, ug: &UGraphic, position: NotePosition) {
        for note in self.notes.iter().filter(|note| note.position == position) {
            let x = note
                .when
                .as_ref()
                .map_or(0.0, |when| self.ruler.get_pos_in_pixel(when));
            note.draw_u(&ug.translated(x, 0.0), self.skin);
        }
    }

    fn get_height_for_notes(
        &self,
        string_bounder: &dyn StringBounder,
        position: NotePosition,
    ) -> f64 {
        self.notes
            .iter()
            .filter(|note| note.position == position)
            .fold(0.0, |height, note| {
                f64::max(height, note.get_height(string_bounder, self.skin))
            })
    }

    fn get_context(&self) -> Fashion {
        Fashion::new(
            self.style.value(PName::BackGroundColor).as_color(),
            self.style.value(PName::LineColor).as_color(),
        )
        .with_stroke(self.style.stroke())
    }

    fn create_text_block(&self, value: &str) -> SheetBlock2 {
        Display::with_newlines(value).create0(
            &self.style.font_configuration(),
            HorizontalAlignment::Left,
            self.skin,
            0.0,
            CreoleMode::Full,
        )
    }

    fn draw_horizontal_between_times(&self, ug: &UGraphic, start_time: f64, end_time: f64) {
        let x1 = self.x_of_time(start_time);
        let x2 = f64::min(self.ruler.get_width(), self.x_of_time(end_time));
        ug.translated(x1, 0.0).draw(&hline(x2 - x1));
    }

    fn x_of_time(&self, time: f64) -> f64 {
        self.ruler.get_pos_in_pixel_internal(time)
    }
}

fn hline(length: f64) -> UShape {
    UShape::Line {
        dx: length,
        dy: 0.0,
    }
}

fn vline(length: f64) -> UShape {
    UShape::Line {
        dx: 0.0,
        dy: length,
    }
}

fn polygon(points: &[(f64, f64)]) -> UShape {
    UShape::polygon(points.to_vec())
}

/// An open path through the points.
fn path(points: &[(f64, f64)]) -> UShape {
    let segments = points
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            if i == 0 {
                USegment::MoveTo(x, y)
            } else {
                USegment::LineTo(x, y)
            }
        })
        .collect();
    UShape::path(segments)
}

fn rectangle(width: f64, height: f64) -> UShape {
    UShape::Rectangle(URectangle::new(width, height))
}

/// A shape filled without an outline, then its outline drawn as `outline` (`PentaAShape` and friends).
fn draw_filled_then_outlined(ug: &UGraphic, context: &Fashion, fill: &UShape, outline: &UShape) {
    let fill_context = Fashion {
        fore_color: context.back_color.clone(),
        ..context.clone()
    };
    fill_context.apply(ug).draw(fill);
    context.apply(ug).draw(outline);
}

/// Concise and rectangle players: a ribbon of states with their names in it (`PanelsState`,
/// `PanelsConcise`, `PanelsRectangle`).
struct PanelsState<'a> {
    base: PanelsBase<'a>,
    player: &'a AbstractStatePlayer,
}

impl<'a> PanelsState<'a> {
    const DEFAULT_RIBBON_HEIGHT: i32 = 24;

    fn new(mut base: PanelsBase<'a>, player: &'a AbstractStatePlayer) -> Self {
        if base.suggested_height == 0 {
            base.suggested_height = Self::DEFAULT_RIBBON_HEIGHT;
        }
        Self { base, player }
    }

    fn changes(&self) -> &[ChangeState] {
        &self.player.changes
    }

    fn ribbon_height(&self) -> f64 {
        self.base.suggested_height()
    }

    fn pos(&self, change: &ChangeState) -> f64 {
        self.base.ruler.get_pos_in_pixel(&change.when)
    }

    fn get_initial_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.player.initial_state.as_deref().map_or(0.0, |initial| {
            self.base
                .create_text_block(initial)
                .calculate_dimension(string_bounder)
                .width
                + 2.0 * MARGIN_X
        })
    }

    fn get_height_for_constraints(&self, string_bounder: &dyn StringBounder) -> f64 {
        f64::max(
            5.0,
            self.base
                .get_height_for_constraints(string_bounder, |_| 0.0),
        )
    }

    fn get_comment_top_block(&self, change: &ChangeState) -> Option<SheetBlock2> {
        change
            .comment
            .as_deref()
            .map(|comment| self.base.create_text_block(comment))
    }

    fn get_height_for_top_comment(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.changes().iter().fold(0.0, |result, change| {
            f64::max(
                result,
                self.get_comment_top_block(change).map_or(0.0, |block| {
                    block.calculate_dimension(string_bounder).height
                }),
            )
        })
    }

    fn is_rectangle(&self) -> bool {
        self.player.style == TimingStyle::Rectangle
    }

    fn draw_hexa(&self, ug: &UGraphic, len: f64, change: &ChangeState) {
        let context = change.get_context(&self.base.style);
        let height = self.ribbon_height();
        if self.is_rectangle() {
            context.apply(ug).draw(&rectangle(len, height));
        } else {
            context.apply(ug).draw(&polygon(&[
                (DELTA, 0.0),
                (len - DELTA, 0.0),
                (len, height / 2.0),
                (len - DELTA, height),
                (DELTA, height),
                (0.0, height / 2.0),
            ]));
        }
    }

    fn draw_penta_a(&self, ug: &UGraphic, len: f64, change: &ChangeState) {
        let context = self.get_context_with_initial_colors(change);
        let height = self.ribbon_height();
        if self.is_rectangle() {
            let outline = [(0.0, 0.0), (len, 0.0), (len, height), (0.0, height)];
            draw_filled_then_outlined(ug, &context, &rectangle(len, height), &path(&outline));
        } else {
            let points = [
                (0.0, 0.0),
                (len - DELTA, 0.0),
                (len, height / 2.0),
                (len - DELTA, height),
                (0.0, height),
            ];
            draw_filled_then_outlined(ug, &context, &polygon(&points), &path(&points));
        }
    }

    fn draw_penta_b(&self, ug: &UGraphic, len: f64, change: &ChangeState) {
        let context = change.get_context(&self.base.style);
        let height = self.ribbon_height();
        if self.is_rectangle() {
            let outline = [(len, 0.0), (0.0, 0.0), (0.0, height), (len, height)];
            draw_filled_then_outlined(ug, &context, &rectangle(len, height), &path(&outline));
        } else {
            let fill = [
                (DELTA, 0.0),
                (len, 0.0),
                (len, height),
                (DELTA, height),
                (0.0, height / 2.0),
            ];
            let outline = [
                (len, 0.0),
                (DELTA, 0.0),
                (0.0, height / 2.0),
                (DELTA, height),
                (len, height),
            ];
            draw_filled_then_outlined(ug, &context, &polygon(&fill), &path(&outline));
        }
    }

    fn draw_flat(&self, ug: &UGraphic, len: f64, change: &ChangeState) {
        change
            .get_context(&self.base.style)
            .apply(ug)
            .translated(0.0, self.ribbon_height() / 2.0)
            .draw(&hline(len));
    }

    fn get_context_with_initial_colors(&self, change: &ChangeState) -> Fashion {
        let mut context = change.get_context(&self.base.style);
        let initial_colors: &Colors = &self.player.initial_colors;
        if let Some(back) = initial_colors.get(ColorType::Back) {
            context.back_color = back.clone();
        }
        if let Some(line) = initial_colors.get(ColorType::Line) {
            context.fore_color = line.clone();
        }
        context
    }

    fn draw_before_zero_state(&self, ug: &UGraphic) {
        let Some(initial) = self.player.initial_state.as_deref() else {
            return;
        };
        let initial_width = self.get_initial_width(ug.string_bounder());
        let ug_before = ug.translated(-initial_width, 0.0);
        match self.changes().first() {
            None => self.draw_single(
                &ug_before,
                initial,
                initial_width + self.base.ruler.get_width(),
            ),
            Some(first) => {
                let len = initial_width + self.pos(first);
                if is_flat(initial) {
                    self.draw_flat(&ug_before, len, first);
                } else {
                    self.draw_penta_a(&ug_before, len, first);
                }
            }
        }
    }

    fn draw_before_zero_state_label(&self, ug: &UGraphic) {
        let Some(initial) = self.player.initial_state.as_deref() else {
            return;
        };
        if is_flat(initial) {
            return;
        }
        let block = self.base.create_text_block(initial);
        let dim = block.calculate_dimension(ug.string_bounder());
        block.draw_u(&ug.translated(-MARGIN_X - dim.width, -dim.height / 2.0));
    }

    fn draw_single(&self, ug: &UGraphic, initial: &str, len: f64) {
        let style = &self.base.style;
        let back = style.value(PName::BackGroundColor).as_color();
        let line = style.value(PName::LineColor).as_color();
        let ug = Fashion::new(back.clone(), back)
            .with_stroke(style.stroke())
            .apply(ug);
        let height = self.ribbon_height();
        if is_flat(initial) {
            ug.with_color(line)
                .translated(0.0, height / 2.0)
                .draw(&hline(len));
            return;
        }
        ug.draw(&rectangle(len, height));
        let ug = ug.with_color(line);
        ug.draw(&hline(len));
        ug.translated(0.0, height).draw(&hline(len));
    }

    fn draw_states(&self, ug: &UGraphic) {
        let changes = self.changes();
        for pair in changes.windows(2) {
            let (a, b) = (self.pos(&pair[0]), self.pos(&pair[1]));
            if pair[0].is_flat() {
                self.draw_flat(&ug.translated(a, 0.0), b - a, &pair[0]);
            } else if !pair[0].is_completely_hidden() {
                self.draw_hexa(&ug.translated(a, 0.0), b - a, &pair[0]);
            }
        }
        if let Some(last) = changes.last() {
            let a = self.pos(last);
            let len = self.base.ruler.get_width() - a;
            if last.is_flat() {
                self.draw_flat(&ug.translated(a, 0.0), len, last);
            } else if !last.is_completely_hidden() {
                self.draw_penta_b(&ug.translated(a, 0.0), len, last);
            }
        }
    }

    fn draw_states_labels(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let changes = self.changes();
        for (i, change) in changes.iter().enumerate() {
            let x = self.pos(change);
            if !change.is_blank() && !change.is_completely_hidden() && !change.is_flat() {
                let state = self.base.create_text_block(change.get_state());
                let dim = state.calculate_dimension(string_bounder);
                let xtext = match changes.get(i + 1) {
                    None => x + MARGIN_X,
                    Some(next) => f64::midpoint(x, self.pos(next)) - dim.width / 2.0,
                };
                state.draw_u(&ug.translated(xtext, -dim.height / 2.0));
            }
            if let Some(comment) = self.get_comment_top_block(change) {
                let dim_comment = comment.calculate_dimension(string_bounder);
                comment.draw_u(&ug.translated(
                    x + MARGIN_X,
                    -self.ribbon_height() / 2.0 - dim_comment.height,
                ));
            }
        }
    }
}

impl Panels for PanelsState<'_> {
    fn draw_left_panel(&self, _ug: &UGraphic, _full_available_width: f64) {}

    fn draw_right_panel(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let height_for_constraints = self.get_height_for_constraints(string_bounder);
        let height_for_top_notes = self
            .base
            .get_height_for_notes(string_bounder, NotePosition::Top);
        let ug_ribbon = ug.translated(
            0.0,
            height_for_constraints
                + self.get_height_for_top_comment(string_bounder)
                + height_for_top_notes,
        );
        let half_ribbon = self.ribbon_height() / 2.0;
        self.draw_before_zero_state(&ug_ribbon);
        self.draw_before_zero_state_label(&ug_ribbon.translated(0.0, half_ribbon));
        self.draw_states(&ug_ribbon);
        self.draw_states_labels(&ug_ribbon.translated(0.0, half_ribbon));
        self.base
            .draw_constraints(&ug.translated(0.0, height_for_constraints / 2.0), |_| 0.0);
        self.base.draw_notes(ug, NotePosition::Top);
        self.base.draw_notes(
            &ug.translated(
                0.0,
                height_for_constraints + self.ribbon_height() + height_for_top_notes,
            ),
            NotePosition::Bottom,
        );
    }

    fn get_full_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_height_for_constraints(string_bounder)
            + self.get_height_for_top_comment(string_bounder)
            + self
                .base
                .get_height_for_notes(string_bounder, NotePosition::Top)
            + self.ribbon_height()
            + self
                .base
                .get_height_for_notes(string_bounder, NotePosition::Bottom)
            + BOTTOM_MARGIN
    }

    fn get_left_panel_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_initial_width(string_bounder)
    }

    fn get_time_projection(
        &self,
        string_bounder: &dyn StringBounder,
        tick: &TimeTick,
    ) -> Option<IntricatedPoint> {
        let x = self.base.ruler.get_pos_in_pixel(tick);
        let y = self.get_height_for_constraints(string_bounder)
            + self
                .base
                .get_height_for_notes(string_bounder, NotePosition::Top)
            + self.get_height_for_top_comment(string_bounder)
            + self.ribbon_height() / 2.0;
        if self.changes().iter().any(|change| change.when == *tick) {
            return Some(IntricatedPoint::single(XPoint2D::new(x, y)));
        }
        let half = self.ribbon_height() / 2.0;
        Some(IntricatedPoint {
            a: XPoint2D::new(x, y - half),
            b: XPoint2D::new(x, y + half),
        })
    }
}

/// Robust players: one level per state, their names on the left (`PanelsRobust`).
struct PanelsRobust<'a> {
    base: PanelsBase<'a>,
    player: &'a AbstractStatePlayer,
    /// The states from the bottom level up.
    all_states: Vec<String>,
}

impl<'a> PanelsRobust<'a> {
    const HISTOGRAM_BOTTOM_MARGIN: f64 = 12.0;
    const HISTOGRAM_INITIAL_WIDTH: f64 = 40.0;

    fn new(base: PanelsBase<'a>, player: &'a AbstractStatePlayer) -> Self {
        let mut all_states: Vec<String> = player
            .states_label
            .iter()
            .rev()
            .map(|(_, label)| label.clone())
            .collect();
        let mut add = |state: &String| {
            if !all_states.contains(state) {
                all_states.push(state.clone());
            }
        };
        if let Some(initial) = &player.initial_state {
            add(initial);
        }
        for change in player
            .changes
            .iter()
            .filter(|change| !change.is_completely_hidden())
        {
            change.states.iter().for_each(&mut add);
        }
        Self {
            base,
            player,
            all_states,
        }
    }

    fn changes(&self) -> &[ChangeState] {
        &self.player.changes
    }

    fn get_states_at(&self, tick: &TimeTick) -> Vec<&str> {
        let changes = self.changes();
        let Some(last) = changes.last() else {
            return Vec::new();
        };
        for (i, change) in changes.iter().enumerate() {
            match change.when.cmp(tick) {
                std::cmp::Ordering::Equal => {
                    return match (i, self.player.initial_state.as_deref()) {
                        (0, None) => vec![change.get_state()],
                        (0, Some(initial)) => vec![initial, change.get_state()],
                        _ => vec![changes[i - 1].get_state(), change.get_state()],
                    };
                }
                // A time before the first change counts as the first change's.
                std::cmp::Ordering::Greater => {
                    return vec![changes[i.saturating_sub(1)].get_state()];
                }
                std::cmp::Ordering::Less => {}
            }
        }
        vec![last.get_state()]
    }

    fn get_points(&self, n: usize) -> Vec<XPoint2D> {
        let x = self.get_pointx(n);
        self.changes()[n]
            .states
            .iter()
            .take(2)
            .map(|state| XPoint2D::new(x, self.y_of_state(state)))
            .collect()
    }

    fn get_pointx(&self, n: usize) -> f64 {
        self.base.ruler.get_pos_in_pixel(&self.changes()[n].when)
    }

    fn state_ys(&self, n: usize) -> impl Iterator<Item = f64> + '_ {
        self.changes()[n]
            .states
            .iter()
            .take(2)
            .map(|state| self.y_of_state(state))
    }

    fn get_point_min_y(&self, n: usize) -> f64 {
        self.state_ys(n).fold(f64::INFINITY, f64::min)
    }

    fn get_point_max_y(&self, n: usize) -> f64 {
        self.state_ys(n).fold(f64::NEG_INFINITY, f64::max)
    }

    fn get_states_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.all_states.iter().fold(0.0, |result, state| {
            f64::max(
                result,
                self.base
                    .create_text_block(state)
                    .calculate_dimension(string_bounder)
                    .width,
            )
        })
    }

    fn draw_hlines(&self, ug: &UGraphic) {
        let changes = self.changes();
        if self.player.initial_state.is_some() {
            let initial_point = self.get_initial_point();
            for point in self.get_points(0) {
                draw_hline_from_point(ug, initial_point, Self::HISTOGRAM_INITIAL_WIDTH + point.x);
            }
        }
        for (i, change) in changes.iter().enumerate() {
            if change.is_completely_hidden() {
                continue;
            }
            let x2 = if i < changes.len() - 1 {
                self.get_pointx(i + 1)
            } else {
                self.base.ruler.get_width()
            };
            let len = x2 - self.get_pointx(i);
            let points = self.get_points(i);
            if let [point1, point2] = points[..] {
                draw_hblock(
                    &ug.with_backcolor(change.get_back_color(&self.base.style)),
                    point1,
                    point2,
                    len,
                );
            }
            if i < changes.len() - 1 {
                for point in points {
                    draw_hline_from_point(ug, point, len);
                }
            }
        }
        let last = changes.len() - 1;
        if !changes[last].is_completely_hidden() {
            for point in self.get_points(last) {
                draw_hline_from_point(ug, point, self.base.ruler.get_width() - point.x);
            }
        }
    }

    fn draw_vlines(&self, ug: &UGraphic) {
        if self.player.initial_state.is_some() {
            let before = self.get_initial_point();
            let current = self.get_points(0)[0];
            ug.translated(current.x, current.y)
                .draw(&vline(before.y - current.y));
        }
        let changes = self.changes();
        for i in 1..changes.len() {
            if changes[i - 1].is_completely_hidden() || changes[i].is_completely_hidden() {
                continue;
            }
            let min_y = f64::min(self.get_point_min_y(i), self.get_point_min_y(i - 1));
            let max_y = f64::max(self.get_point_max_y(i), self.get_point_max_y(i - 1));
            ug.translated(self.get_pointx(i), min_y)
                .draw(&vline(max_y - min_y));
        }
    }

    fn draw_labels(&self, ug: &UGraphic) {
        for (i, change) in self.changes().iter().enumerate() {
            let Some(comment) = &change.comment else {
                continue;
            };
            let point = self.get_points(i)[0];
            let label = self.base.create_text_block(comment);
            let dim = label.calculate_dimension(ug.string_bounder());
            label.draw_u(&ug.translated(point.x + 2.0, point.y - dim.height));
        }
    }

    fn get_constraint_delta_y(&self, constraint: &TimeConstraint) -> f64 {
        self.changes()
            .iter()
            .filter(|change| constraint.contains_strict(&change.when))
            .fold(self.y_of_time(constraint.tick1()), |y, change| {
                f64::min(y, self.y_of_time(&change.when))
            })
    }

    fn get_initial_point(&self) -> XPoint2D {
        XPoint2D::new(
            -Self::HISTOGRAM_INITIAL_WIDTH,
            self.y_of_state(self.player.initial_state.as_deref().unwrap_or_default()),
        )
    }

    fn get_height_for_constraints(&self, string_bounder: &dyn StringBounder) -> f64 {
        f64::max(
            10.0,
            self.base
                .get_height_for_constraints(string_bounder, |constraint| {
                    self.get_constraint_delta_y(constraint)
                }),
        )
    }

    /// The level of a state; one above the top for a state without a level.
    fn y_of_state(&self, state: &str) -> f64 {
        let index = self
            .all_states
            .iter()
            .position(|existing| existing == state)
            .map_or(-1, |index| index as i64);
        let nb = self.all_states.len() as i64 - 1 - index;
        self.step_height() * nb as f64
    }

    fn y_of_time(&self, when: &TimeTick) -> f64 {
        self.y_of_state(self.get_states_at(when).last().copied().unwrap_or_default())
    }

    fn step_height(&self) -> f64 {
        let levels = self.all_states.len() as i32;
        if self.base.suggested_height == 0 || levels <= 1 {
            20.0
        } else {
            f64::from(self.base.suggested_height / (levels - 1))
        }
    }
}

fn draw_hline_from_point(ug: &UGraphic, start: XPoint2D, length: f64) {
    ug.translated(start.x, start.y).draw(&hline(length));
}

/// The hatched block of a robust player in two states at once.
fn draw_hblock(ug: &UGraphic, point1: XPoint2D, point2: XPoint2D, len: f64) {
    let min_y = f64::min(point1.y, point2.y);
    let max_y = f64::max(point1.y, point2.y);
    let ug = ug.translated(point1.x, min_y);
    ug.draw(&rectangle(len, max_y - min_y));
    let mut x = 0.0;
    while x < len {
        ug.translated(x, 0.0).draw(&vline(max_y - min_y));
        x += 5.0;
    }
}

impl Panels for PanelsRobust<'_> {
    fn draw_left_panel(&self, ug: &UGraphic, full_available_width: f64) {
        let string_bounder = ug.string_bounder();
        let mut width = self.get_states_width(string_bounder);
        if self.player.initial_state.is_some() {
            width += Self::HISTOGRAM_INITIAL_WIDTH;
        }
        let dx = if full_available_width > width + 5.0 {
            full_available_width - width - 5.0
        } else {
            full_available_width - width
        };
        let ug = ug.translated(dx, self.get_height_for_constraints(string_bounder));
        for state in &self.all_states {
            let label = self.base.create_text_block(state);
            let dim = label.calculate_dimension(string_bounder);
            label.draw_u(&ug.translated(0.0, self.y_of_state(state) - dim.height / 2.0 + 1.0));
        }
    }

    fn draw_right_panel(&self, ug: &UGraphic) {
        if self.changes().is_empty() {
            return;
        }
        let ug = self.base.get_context().apply(ug);
        let ug = ug.translated(0.0, self.get_height_for_constraints(ug.string_bounder()));
        self.draw_hlines(&ug);
        self.draw_vlines(&ug);
        self.draw_labels(&ug);
        self.base.draw_constraints(
            &ug.translated(0.0, -TimeConstraint::TOP_MARGIN),
            |constraint| self.get_constraint_delta_y(constraint),
        );
    }

    fn get_full_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        let mut height = self.get_height_for_constraints(string_bounder);
        if !self.all_states.is_empty() {
            height += self.step_height() * (self.all_states.len() - 1) as f64;
        }
        height + Self::HISTOGRAM_BOTTOM_MARGIN + 6.0
    }

    fn get_left_panel_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        let width = self.get_states_width(string_bounder);
        if self.player.initial_state.is_some() {
            width + Self::HISTOGRAM_INITIAL_WIDTH
        } else {
            width
        }
    }

    fn get_time_projection(
        &self,
        string_bounder: &dyn StringBounder,
        tick: &TimeTick,
    ) -> Option<IntricatedPoint> {
        let x = self.base.ruler.get_pos_in_pixel(tick);
        let states = self.get_states_at(tick);
        let height_for_constraints = self.get_height_for_constraints(string_bounder);
        let point = |state: &str| XPoint2D::new(x, self.y_of_state(state) + height_for_constraints);
        match states[..] {
            [] => None,
            [only] => Some(IntricatedPoint::single(point(only))),
            [before, after, ..] => Some(IntricatedPoint {
                a: point(before),
                b: point(after),
            }),
        }
    }
}

/// Clocks: a square wave of the period (`PanelsClock`).
struct PanelsClock<'a> {
    base: PanelsBase<'a>,
    clock: &'a PlayerClock,
}

impl PanelsClock<'_> {
    fn get_line_height(&self) -> f64 {
        self.base.suggested_height() - 2.0 * MARGIN_Y
    }

    fn pulse(&self) -> f64 {
        let pulse = self.clock.pulse.double_value();
        if pulse == 0.0 {
            self.clock.period.double_value() / 2.0
        } else {
            pulse
        }
    }

    /// Whether the clock is high at the time, as drawn.
    fn is_high_at(&self, time: f64) -> bool {
        let period = self.clock.period.double_value();
        let offset = self.clock.offset.double_value();
        if period <= 0.0 || time < offset {
            return false;
        }
        (time - offset).rem_euclid(period) < self.pulse()
    }
}

impl Panels for PanelsClock<'_> {
    fn draw_left_panel(&self, _ug: &UGraphic, _full_available_width: f64) {}

    fn draw_right_panel(&self, ug: &UGraphic) {
        let base = &self.base;
        let ug = base.get_context().apply(ug).translated(0.0, MARGIN_Y);
        let line_height = self.get_line_height();
        let low = ug.translated(0.0, line_height);
        let width = base.ruler.get_width();
        let draw_vline = |time: f64| {
            ug.translated(base.x_of_time(time), 0.0)
                .draw(&vline(line_height));
        };
        let mut value = 0.0;
        let offset = self.clock.offset.double_value();
        if offset != 0.0 {
            base.draw_horizontal_between_times(&low, value, offset);
            value += offset;
        }
        if base.x_of_time(value) > width {
            return;
        }
        draw_vline(value);
        let pulse = self.pulse();
        let remain = self.clock.period.double_value() - pulse;
        for _ in 0..1000 {
            base.draw_horizontal_between_times(&ug, value, value + pulse);
            value += pulse;
            if base.x_of_time(value) > width {
                return;
            }
            draw_vline(value);
            base.draw_horizontal_between_times(&low, value, value + remain);
            value += remain;
            if base.x_of_time(value) > width {
                return;
            }
            draw_vline(value);
        }
    }

    fn get_full_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        self.base.suggested_height()
    }

    fn get_left_panel_width(&self, _string_bounder: &dyn StringBounder) -> f64 {
        LEFT_PANEL_MIN_WIDTH
    }

    fn get_time_projection(
        &self,
        _string_bounder: &dyn StringBounder,
        tick: &TimeTick,
    ) -> Option<IntricatedPoint> {
        let x = self.base.ruler.get_pos_in_pixel(tick);
        let y = if self.is_high_at(tick.time.double_value()) {
            MARGIN_Y
        } else {
            MARGIN_Y + self.get_line_height()
        };
        Some(IntricatedPoint::single(XPoint2D::new(x, y)))
    }
}

/// Binary players: high or low, hatched while both (`PanelsBinary`).
struct PanelsBinary<'a> {
    base: PanelsBase<'a>,
    binary: &'a PlayerBinary,
}

impl PanelsBinary<'_> {
    fn top(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.base
            .get_height_for_constraints(string_bounder, |_| 0.0)
            + self
                .base
                .get_height_for_notes(string_bounder, NotePosition::Top)
    }

    fn get_yhigh(&self, string_bounder: &dyn StringBounder) -> f64 {
        MARGIN_Y + self.top(string_bounder)
    }

    fn get_ylow(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.top(string_bounder) + self.base.suggested_height() - MARGIN_Y
    }

    fn get_ypos(&self, string_bounder: &dyn StringBounder, state: &str) -> f64 {
        if state.eq_ignore_ascii_case(LOW_STRING) {
            self.get_ylow(string_bounder)
        } else {
            self.get_yhigh(string_bounder)
        }
    }
}

impl Panels for PanelsBinary<'_> {
    fn draw_left_panel(&self, _ug: &UGraphic, _full_available_width: f64) {}

    fn draw_right_panel(&self, ug: &UGraphic) {
        let base = &self.base;
        let ug = base.get_context().apply(ug);
        let string_bounder = ug.string_bounder();
        let low_only = [LOW_STRING.to_owned()];
        let mut last_values: &[String] = self.binary.initial_states.as_deref().unwrap_or(&low_only);
        let yhigh = self.get_yhigh(string_bounder);
        let ylow = self.get_ylow(string_bounder);
        let mut lastx = 0.0;
        for (tick, value) in &self.binary.values {
            let x = base.ruler.get_pos_in_pixel(tick);
            if let [only] = last_values {
                ug.translated(lastx, self.get_ypos(string_bounder, only))
                    .draw(&hline(x - lastx));
            } else {
                let mut tmpx = lastx;
                while tmpx < x {
                    ug.translated(tmpx, yhigh).draw(&vline(ylow - yhigh));
                    tmpx += 5.0;
                }
            }
            if last_values != value.states.as_slice() {
                ug.translated(x, yhigh).draw(&vline(ylow - yhigh));
            }
            if let Some(comment) = &value.comment {
                base.create_text_block(comment)
                    .draw_u(&ug.translated(x + 2.0, yhigh));
            }
            lastx = x;
            last_values = &value.states;
        }
        ug.translated(lastx, self.get_ypos(string_bounder, &last_values[0]))
            .draw(&hline(base.ruler.get_width() - lastx));
        let height_for_constraints = base.get_height_for_constraints(string_bounder, |_| 0.0);
        base.draw_constraints(&ug.translated(0.0, height_for_constraints), |_| 0.0);
        base.draw_notes(&ug.translated(0.0, MARGIN_Y), NotePosition::Top);
        base.draw_notes(
            &ug.translated(
                0.0,
                height_for_constraints
                    + base.get_height_for_notes(string_bounder, NotePosition::Top)
                    + base.suggested_height()
                    - MARGIN_Y / 2.0,
            ),
            NotePosition::Bottom,
        );
    }

    fn get_full_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.top(string_bounder)
            + self.base.suggested_height()
            + self
                .base
                .get_height_for_notes(string_bounder, NotePosition::Bottom)
    }

    fn get_left_panel_width(&self, _string_bounder: &dyn StringBounder) -> f64 {
        LEFT_PANEL_MIN_WIDTH
    }

    fn get_time_projection(
        &self,
        string_bounder: &dyn StringBounder,
        tick: &TimeTick,
    ) -> Option<IntricatedPoint> {
        let x = self.base.ruler.get_pos_in_pixel(tick);
        Some(IntricatedPoint::single(XPoint2D::new(
            x,
            self.get_ypos(string_bounder, HIGH_STRING),
        )))
    }
}

/// Analog players: a line through their values, the scale on the left (`PanelsAnalog`).
struct PanelsAnalog<'a> {
    base: PanelsBase<'a>,
    analog: &'a PlayerAnalog,
}

impl PanelsAnalog<'_> {
    fn get_ypos(&self, string_bounder: &dyn StringBounder, value: f64) -> f64 {
        let series = &self.analog.time_series;
        let suggested_height = self.base.suggested_height();
        let y = (value - series.get_min()) * (suggested_height - 2.0 * MARGIN_Y)
            / (series.get_max() - series.get_min());
        self.base
            .get_height_for_constraints(string_bounder, |_| 0.0)
            + suggested_height
            - MARGIN_Y
            - y
    }

    /// The integers of the scale that get a label and a line.
    fn ticks(&self, every: i32) -> impl Iterator<Item = i32> {
        let series = &self.analog.time_series;
        let first = series.get_min().ceil() as i32;
        let last = series.get_max().floor() as i32;
        (first..=last).filter(move |i| i % every == 0)
    }

    /// The values the scale is labelled with.
    fn scale_values(&self) -> Vec<f64> {
        let series = &self.analog.time_series;
        match self.analog.ticks_every {
            None => vec![series.get_min(), series.get_max()],
            Some(every) => self.ticks(every).map(f64::from).collect(),
        }
    }

    fn get_text_block(&self, value: f64) -> SheetBlock2 {
        self.base
            .create_text_block(&self.analog.time_series.get_display_value(value))
    }

    fn draw_scale_label(&self, ug: &UGraphic, value: f64, full_available_width: f64) {
        let label = self.get_text_block(value);
        let dim = label.calculate_dimension(ug.string_bounder());
        let ug = ug.translated(full_available_width - dim.width - 2.0, 0.0);
        label.draw_u(&ug.translated(
            0.0,
            self.get_ypos(ug.string_bounder(), value) - dim.height / 2.0,
        ));
    }
}

impl Panels for PanelsAnalog<'_> {
    fn draw_left_panel(&self, ug: &UGraphic, full_available_width: f64) {
        for value in self.scale_values() {
            self.draw_scale_label(ug, value, full_available_width);
        }
    }

    fn draw_right_panel(&self, ug: &UGraphic) {
        let base = &self.base;
        let string_bounder = ug.string_bounder();
        if let Some(every) = self.analog.ticks_every {
            let ug = apply_for_vlines(ug, &base.style);
            for i in self.ticks(every) {
                ug.translated(0.0, self.get_ypos(string_bounder, f64::from(i)))
                    .draw(&hline(base.ruler.get_width()));
            }
        }
        let ug = base.get_context().apply(ug);
        let mut lastx = 0.0;
        let mut last_value = self.analog.initial_state.unwrap_or(0.0);
        for (tick, &value) in &self.analog.time_series.values {
            let y1 = self.get_ypos(string_bounder, last_value);
            let y2 = self.get_ypos(string_bounder, value);
            let x = base.ruler.get_pos_in_pixel(tick);
            ug.translated(lastx, y1).draw(&UShape::Line {
                dx: x - lastx,
                dy: y2 - y1,
            });
            lastx = x;
            last_value = value;
        }
        ug.translated(lastx, self.get_ypos(string_bounder, last_value))
            .draw(&hline(base.ruler.get_width() - lastx));
        base.draw_constraints(
            &ug.translated(
                0.0,
                base.get_height_for_constraints(string_bounder, |_| 0.0),
            ),
            |_| 0.0,
        );
    }

    fn get_full_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.base
            .get_height_for_constraints(string_bounder, |_| 0.0)
            + self.base.suggested_height()
    }

    fn get_left_panel_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        let widest = self
            .scale_values()
            .into_iter()
            .map(|value| {
                self.get_text_block(value)
                    .calculate_dimension(string_bounder)
                    .width
            })
            .fold(0.0, f64::max);
        LEFT_PANEL_MIN_WIDTH + widest
    }

    fn get_time_projection(
        &self,
        string_bounder: &dyn StringBounder,
        tick: &TimeTick,
    ) -> Option<IntricatedPoint> {
        let x = self.base.ruler.get_pos_in_pixel(tick);
        let value = self.analog.time_series.get_value_at(tick);
        Some(IntricatedPoint::single(XPoint2D::new(
            x,
            self.get_ypos(string_bounder, value),
        )))
    }
}

//! The lines of a timing diagram and what happens on them (PlantUML's `Player` and its subclasses,
//! `ChangeState`, `TimeConstraint`, `TimingNote` and `TimeSeries`).

use std::collections::BTreeMap;

use super::ruler::TimingRuler;
use super::time::{BigDecimal, TimeTick};
use super::timing_style;
use crate::color::{ColorType, Colors, HColor};
use crate::creole::{CreoleMode, CreoleParser, Display, SheetBlock1};
use crate::direction::Direction;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};
use crate::svek::image::Opale;

/// How a state player draws its states (`TimingStyle`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TimingStyle {
    Robust,
    Concise,
    Rectangle,
}

pub(super) struct Player {
    pub(super) title: Display,
    pub(super) compact: bool,
    pub(super) stereotype: Option<Stereotype>,
    pub(super) general_background_color: Option<HColor>,
    pub(super) suggested_height: i32,
    sname: SName,
    pub(super) notes: Vec<TimingNote>,
    pub(super) kind: PlayerKind,
}

pub(super) enum PlayerKind {
    State(AbstractStatePlayer),
    Clock(PlayerClock),
    Binary(PlayerBinary),
    Analog(PlayerAnalog),
}

pub(super) struct AbstractStatePlayer {
    pub(super) style: TimingStyle,
    /// One change per time, the first given.
    pub(super) changes: Vec<ChangeState>,
    pub(super) constraints: Vec<TimeConstraint>,
    /// The labels of the states codes stand for, in the order they were defined.
    pub(super) states_label: Vec<(String, String)>,
    pub(super) initial_state: Option<String>,
    pub(super) initial_colors: Colors,
}

pub(super) struct PlayerClock {
    pub(super) period: BigDecimal,
    pub(super) pulse: BigDecimal,
    pub(super) offset: BigDecimal,
}

#[derive(Default)]
pub(super) struct PlayerBinary {
    pub(super) constraints: Vec<TimeConstraint>,
    pub(super) values: BTreeMap<TimeTick, ChangeState>,
    pub(super) initial_states: Option<Vec<String>>,
}

#[derive(Default)]
pub(super) struct PlayerAnalog {
    pub(super) time_series: TimeSeries,
    pub(super) constraints: Vec<TimeConstraint>,
    pub(super) initial_state: Option<f64>,
    pub(super) ticks_every: Option<i32>,
}

impl Player {
    pub(super) fn new(
        title: &str,
        compact: bool,
        stereotype: Option<Stereotype>,
        general_background_color: Option<HColor>,
        kind: PlayerKind,
    ) -> Self {
        let (sname, suggested_height) = match &kind {
            PlayerKind::State(state) => match state.style {
                TimingStyle::Robust => (SName::Robust, 0),
                TimingStyle::Concise => (SName::Concise, 0),
                TimingStyle::Rectangle => (SName::Rectangle, 0),
            },
            PlayerKind::Clock(_) => (SName::Clock, 30),
            PlayerKind::Binary(_) => (SName::Binary, 30),
            PlayerKind::Analog(_) => (SName::Analog, 100),
        };
        Self {
            title: Display::with_newlines(title),
            compact,
            stereotype,
            general_background_color,
            suggested_height,
            sname,
            notes: Vec::new(),
            kind,
        }
    }

    pub(super) fn get_style(&self, skin: &SkinParam) -> Style {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::TimingDiagram,
            self.sname,
        ])
        .get_merged_style_with(&skin.current_style_builder(), self.stereotype.as_ref())
    }

    pub(super) fn add_note(
        &mut self,
        now: Option<TimeTick>,
        note: Display,
        position: NotePosition,
        stereotype: Option<&Stereotype>,
        skin: &SkinParam,
    ) {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::TimingDiagram,
            SName::Note,
        ])
        .get_merged_style_with(&skin.current_style_builder(), stereotype);
        self.notes.push(TimingNote {
            when: now,
            note,
            position,
            style,
        });
    }

    /// The states codes stand for (`defineState`); only state players have named states.
    pub(super) fn define_state(&mut self, state_code: &str, label: &str) {
        if let PlayerKind::State(player) = &mut self.kind {
            match player
                .states_label
                .iter_mut()
                .find(|(code, _)| code == state_code)
            {
                Some(entry) => label.clone_into(&mut entry.1),
                None => player
                    .states_label
                    .push((state_code.to_owned(), label.to_owned())),
            }
        }
    }

    /// The state from `now` on, the initial one without a time (`setState`); clocks have no states.
    pub(super) fn set_state(
        &mut self,
        now: Option<TimeTick>,
        comment: Option<String>,
        colors: Colors,
        states: Vec<String>,
    ) {
        match &mut self.kind {
            PlayerKind::State(player) => player.set_state(now, comment, colors, states),
            PlayerKind::Binary(player) => player.set_state(now, comment, colors, &states),
            PlayerKind::Analog(player) => player.set_state(now, &states[0]),
            PlayerKind::Clock(_) => {}
        }
    }

    pub(super) fn create_constraint(
        &mut self,
        tick1: TimeTick,
        tick2: TimeTick,
        message: Option<&str>,
        color: Option<HColor>,
    ) {
        let (constraints, margin_x) = match &mut self.kind {
            PlayerKind::State(player) => (
                &mut player.constraints,
                match player.style {
                    TimingStyle::Robust => 2.5,
                    TimingStyle::Concise | TimingStyle::Rectangle => 1.0,
                },
            ),
            PlayerKind::Binary(player) => (&mut player.constraints, 2.5),
            PlayerKind::Analog(player) => (&mut player.constraints, 1.0),
            PlayerKind::Clock(_) => return,
        };
        constraints.push(TimeConstraint {
            margin_x,
            tick1,
            tick2,
            label: Display::with_newlines(message.unwrap_or_default()),
            color,
        });
    }

    /// The player's title, in the diagram's font; nothing for a blank one.
    fn title_block(&self, skin: &SkinParam) -> Option<impl TextBlock + use<>> {
        if self.title.is_white() {
            return None;
        }
        Some(self.title.create0(
            &timing_style(skin, &[]).font_configuration(),
            HorizontalAlignment::Left,
            skin,
            0.0,
            CreoleMode::Full,
        ))
    }

    /// The title, underlined by a line breaking up to the right unless the player is compact
    /// (`PlayerFrame.drawFrameTitle`).
    pub(super) fn draw_frame_title(&self, ug: &UGraphic, skin: &SkinParam) {
        let Some(title) = self.title_block(skin) else {
            return;
        };
        title.draw_u(ug);
        if self.compact {
            return;
        }
        let style = timing_style(skin, &[]);
        let ug = ug
            .with_color(style.value(PName::LineColor).as_color())
            .with_stroke(style.stroke());
        let dim_title = title.calculate_dimension(ug.string_bounder());
        let width = dim_title.width + 1.0;
        let height = dim_title.height + 1.0;
        let coords = [
            (-super::MARGIN_X1, height),
            (width, height),
            (width + 10.0, 0.0),
        ];
        for pair in coords.windows(2) {
            let ((x1, y1), (x2, y2)) = (pair[0], pair[1]);
            ug.translated(x1, y1).draw(&UShape::Line {
                dx: x2 - x1,
                dy: y2 - y1,
            });
        }
    }

    /// `PlayerFrame.getHeight`.
    pub(super) fn get_frame_height(
        &self,
        string_bounder: &dyn StringBounder,
        skin: &SkinParam,
    ) -> f64 {
        let height = self.title_block(skin).map_or(0.0, |title| {
            title.calculate_dimension(string_bounder).height
        });
        if self.compact {
            height - 1.0
        } else {
            height + 1.0
        }
    }
}

impl AbstractStatePlayer {
    pub(super) fn new(style: TimingStyle) -> Self {
        Self {
            style,
            changes: Vec::new(),
            constraints: Vec::new(),
            states_label: Vec::new(),
            initial_state: None,
            initial_colors: Colors::default(),
        }
    }

    fn set_state(
        &mut self,
        now: Option<TimeTick>,
        comment: Option<String>,
        colors: Colors,
        states: Vec<String>,
    ) {
        let states: Vec<String> = states
            .into_iter()
            .map(|state| self.decode_state(state))
            .collect();
        match now {
            None => {
                self.initial_state = states.into_iter().next();
                self.initial_colors = colors;
            }
            Some(when) => {
                if let Err(index) = self
                    .changes
                    .binary_search_by(|change| change.when.cmp(&when))
                {
                    self.changes.insert(
                        index,
                        ChangeState {
                            when,
                            states,
                            comment,
                            colors,
                        },
                    );
                }
            }
        }
    }

    fn decode_state(&self, code: String) -> String {
        self.states_label
            .iter()
            .find(|(state_code, _)| *state_code == code)
            .map_or(code, |(_, label)| label.clone())
    }
}

impl PlayerBinary {
    fn set_state(
        &mut self,
        now: Option<TimeTick>,
        comment: Option<String>,
        colors: Colors,
        states: &[String],
    ) {
        let converted = states.iter().take(2).map(|state| convert(state)).collect();
        match now {
            None => self.initial_states = Some(converted),
            Some(when) => {
                self.values.insert(
                    when.clone(),
                    ChangeState {
                        when,
                        states: converted,
                        comment,
                        colors,
                    },
                );
            }
        }
    }
}

pub(super) const LOW_STRING: &str = "0";
pub(super) const HIGH_STRING: &str = "1";

fn convert(value: &str) -> String {
    if value == "1" || value.eq_ignore_ascii_case("high") {
        HIGH_STRING
    } else {
        LOW_STRING
    }
    .to_owned()
}

impl PlayerAnalog {
    fn set_state(&mut self, now: Option<TimeTick>, value: &str) {
        let value = value.parse().unwrap_or(0.0);
        match now {
            None => self.initial_state = Some(value),
            Some(now) => {
                self.time_series.values.insert(now, value);
            }
        }
        if self.initial_state.is_none() {
            self.initial_state = Some(value);
        }
    }
}

/// A player's states from a time on (`ChangeState`).
pub(super) struct ChangeState {
    pub(super) when: TimeTick,
    pub(super) states: Vec<String>,
    pub(super) comment: Option<String>,
    pub(super) colors: Colors,
}

impl ChangeState {
    pub(super) fn get_state(&self) -> &str {
        &self.states[0]
    }

    pub(super) fn get_back_color(&self, style: &Style) -> HColor {
        self.colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color())
    }

    fn get_line_color(&self, style: &Style) -> HColor {
        self.colors
            .get(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color())
    }

    pub(super) fn get_context(&self, style: &Style) -> Fashion {
        Fashion::new(self.get_back_color(style), self.get_line_color(style))
            .with_stroke(style.stroke())
    }

    pub(super) fn is_blank(&self) -> bool {
        self.states[0] == "{...}"
    }

    pub(super) fn is_completely_hidden(&self) -> bool {
        self.states[0] == "{hidden}"
    }

    pub(super) fn is_flat(&self) -> bool {
        is_flat(&self.states[0])
    }
}

pub(super) fn is_flat(state: &str) -> bool {
    state == "{-}"
}

/// A labelled double arrow between two times of a player (`TimeConstraint`).
pub(super) struct TimeConstraint {
    margin_x: f64,
    tick1: TimeTick,
    tick2: TimeTick,
    label: Display,
    /// The colour the arrow's style gives.
    color: Option<HColor>,
}

impl TimeConstraint {
    pub(super) const TOP_MARGIN: f64 = 5.0;

    pub(super) fn tick1(&self) -> &TimeTick {
        &self.tick1
    }

    pub(super) fn contains_strict(&self, other: &TimeTick) -> bool {
        self.tick1 < *other && self.tick2 > *other
    }

    fn get_style(skin: &SkinParam) -> Style {
        timing_style(skin, &[SName::ConstraintArrow])
    }

    fn get_text_block(&self, skin: &SkinParam) -> impl TextBlock + use<> {
        self.label.create0(
            &Self::get_style(skin).font_configuration(),
            HorizontalAlignment::Left,
            skin,
            0.0,
            CreoleMode::Full,
        )
    }

    pub(super) fn draw_u(&self, ug: &UGraphic, ruler: &TimingRuler, skin: &SkinParam) {
        let style = Self::get_style(skin);
        let arrow_color = self
            .color
            .clone()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        let ug = ug
            .with_color(arrow_color.clone())
            .with_backcolor(arrow_color);
        let x1 = ruler.get_pos_in_pixel(&self.tick1) + self.margin_x;
        let x2 = ruler.get_pos_in_pixel(&self.tick2) - self.margin_x;
        let draw_line = |from: f64, to: f64| {
            ug.translated(from, 0.0)
                .with_stroke(style.stroke())
                .draw(&UShape::Line {
                    dx: to - from,
                    dy: 0.0,
                });
        };
        let simple = ug.with_stroke(UStroke::SIMPLE);
        if x2 - x1 > 20.0 {
            draw_line(x1 + 3.0, x2 - 3.0);
            simple.draw(&get_polygon(Direction::Left, x1));
            simple.draw(&get_polygon(Direction::Right, x2));
        } else {
            draw_line(x1 - 1.0, x2 + 1.0);
            simple.draw(&get_polygon(Direction::Right, x1));
            simple.draw(&get_polygon(Direction::Left, x2));
        }
        let text = self.get_text_block(skin);
        let dim_text = text.calculate_dimension(ug.string_bounder());
        let x = x1 + (x2 - x1 - dim_text.width) / 2.0;
        text.draw_u(&ug.translated(x, -self.get_constraint_height(ug.string_bounder(), skin)));
    }

    pub(super) fn get_constraint_height(
        &self,
        string_bounder: &dyn StringBounder,
        skin: &SkinParam,
    ) -> f64 {
        self.get_text_block(skin)
            .calculate_dimension(string_bounder)
            .height
            + Self::TOP_MARGIN
    }
}

/// An arrow head pointing `direction`, its tip at `x` on the line.
fn get_polygon(direction: Direction, x: f64) -> UShape {
    const DX: f64 = 8.0;
    const DY: f64 = 4.0;
    let back = if direction == Direction::Right {
        -DX
    } else {
        DX
    };
    UShape::polygon(vec![(x + back, DY), (x + back, -DY), (x, 0.0)])
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NotePosition {
    Top,
    Bottom,
}

/// A note above or below a player, at the time it was written (`TimingNote`).
pub(super) struct TimingNote {
    pub(super) when: Option<TimeTick>,
    note: Display,
    pub(super) position: NotePosition,
    style: Style,
}

impl TimingNote {
    const MARGIN_Y: f64 = 10.0;

    fn create_opale(&self, skin: &SkinParam) -> Opale<'static> {
        let font_configuration = self.style.font_configuration();
        let sheet = CreoleParser::with_mode(
            font_configuration.clone(),
            skin.get_default_text_alignment(HorizontalAlignment::Left),
            CreoleMode::Full,
            skin,
        )
        .create_display_sheet(&self.note, &font_configuration);
        Opale::new(
            self.style.value(PName::LineColor).as_color(),
            self.style.value(PName::BackGroundColor).as_color(),
            Box::new(SheetBlock1::new(sheet, skin.get_padding())),
            self.style.stroke(),
            0.0,
        )
    }

    pub(super) fn draw_u(&self, ug: &UGraphic, skin: &SkinParam) {
        let ug = if self.position == NotePosition::Bottom {
            ug.translated(0.0, Self::MARGIN_Y / 2.0)
        } else {
            ug.clone()
        };
        self.create_opale(skin).draw_u(&ug);
    }

    pub(super) fn get_height(&self, string_bounder: &dyn StringBounder, skin: &SkinParam) -> f64 {
        let XDimension2D { height, .. } =
            self.create_opale(skin).calculate_dimension(string_bounder);
        height + Self::MARGIN_Y
    }
}

/// The values of an analog player over time (`TimeSeries`).
#[derive(Default)]
pub(super) struct TimeSeries {
    pub(super) values: BTreeMap<TimeTick, f64>,
    bounds: Option<(f64, f64)>,
    /// The fraction digits the bounds were written with, fewest and most (`DeduceFormat`).
    fraction_digits: Option<(usize, usize)>,
}

impl TimeSeries {
    pub(super) fn get_min(&self) -> f64 {
        if let Some((min, _)) = self.bounds {
            return min;
        }
        self.values
            .values()
            .fold(0.0, |min, &value| f64::min(min, value))
    }

    pub(super) fn get_max(&self) -> f64 {
        if let Some((_, max)) = self.bounds {
            return max;
        }
        let max = self
            .values
            .values()
            .fold(0.0, |max, &value| f64::max(max, value));
        if max == 0.0 { 10.0 } else { max }
    }

    /// The bounds as the command wrote them, which also set how values print.
    pub(super) fn set_bounds(&mut self, min: &str, max: &str) {
        self.bounds = Some((min.parse().unwrap_or(0.0), max.parse().unwrap_or(0.0)));
        let (digits1, digits2) = (fraction_digits(min), fraction_digits(max));
        self.fraction_digits = Some((digits1.min(digits2), digits1.max(digits2)));
    }

    /// A value as the bounds were written, or as Java prints a double.
    pub(super) fn get_display_value(&self, value: f64) -> String {
        match self.fraction_digits {
            Some((min, max)) => decimal_format(value, min, max),
            None => crate::java::double_to_string(value),
        }
    }

    pub(super) fn get_value_at(&self, tick: &TimeTick) -> f64 {
        if let Some(&value) = self.values.get(tick) {
            return value;
        }
        let mut last: Option<(&TimeTick, f64)> = None;
        for (key, &v2) in &self.values {
            if key > tick {
                let Some((last_tick, v1)) = last else {
                    return v2;
                };
                let t2 = key.time.double_value();
                let t1 = last_tick.time.double_value();
                let p = (tick.time.double_value() - t1) / (t2 - t1);
                return v1 + (v2 - v1) * p;
            }
            last = Some((key, v2));
        }
        last.expect("an analog player with a value").1
    }
}

/// How many digits follow the decimal point of a bound like `-4.5` (`DeduceFormat.from`; the command only
/// accepts plain decimals).
fn fraction_digits(number: &str) -> usize {
    number
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len())
}

/// `DecimalFormat` with the pattern `0.0…`: rounded half to even to `max` decimals, trailing zeros dropped
/// down to `min`.
fn decimal_format(value: f64, min: usize, max: usize) -> String {
    let rounded = format!("{:.max$}", value.abs());
    let (integer, fraction) = rounded.split_once('.').unwrap_or((&rounded, ""));
    let mut fraction = fraction.to_owned();
    while fraction.len() > min && fraction.ends_with('0') {
        fraction.pop();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    if fraction.is_empty() {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}.{fraction}")
    }
}

#[cfg(test)]
mod tests {
    use super::decimal_format;

    #[test]
    fn analog_values_print_like_decimal_format() {
        assert_eq!(decimal_format(-3.0, 1, 1), "-3.0");
        assert_eq!(decimal_format(6.0, 1, 1), "6.0");
        assert_eq!(decimal_format(0.125, 2, 2), "0.12");
        assert_eq!(decimal_format(1.5, 0, 2), "1.5");
        assert_eq!(decimal_format(2.0, 0, 2), "2");
    }
}

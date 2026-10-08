//! Where times go across the diagram, and the time axis below it (PlantUML's `TimingRuler`).

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::time::{BigDecimal, TimeTick, TimingFormat};
use super::timing_style;
use crate::creole::{CreoleMode, Display};
use crate::klimt::font::StringBounder;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, ValueReading};

/// What the time axis below the diagram shows (`TimeAxisStategy`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TimeAxisStategy {
    Automatic,
    Hidden,
    /// A tick at each time named in the diagram, labelled with its name when it has one.
    Manual,
}

pub(super) struct TimingRuler {
    /// The diagram fills in a zero when drawing, after it has been measured, as PlantUML does.
    times: RefCell<BTreeSet<TimeTick>>,
    tick_interval_in_pixels: i64,
    forced_tick_unitary: i64,
    format: TimingFormat,
}

impl Default for TimingRuler {
    fn default() -> Self {
        Self {
            times: RefCell::default(),
            tick_interval_in_pixels: 50,
            forced_tick_unitary: 0,
            format: TimingFormat::Decimal,
        }
    }
}

const MAX_DIAGRAM_WIDTH: f64 = 4000.0;
const TICK_HEIGHT: f64 = 5.0;

/// The dashed vertical lines of the time grid (`applyForVLines`).
pub(super) fn apply_for_vlines(ug: &UGraphic, style: &Style) -> UGraphic {
    ug.with_stroke(UStroke {
        dash_visible: 3.0,
        dash_space: 5.0,
        thickness: 0.5,
    })
    .with_color(style.value(PName::LineColor).as_color())
}

impl TimingRuler {
    pub(super) fn ensure_not_empty(&self) {
        let mut times = self.times.borrow_mut();
        let zero = || TimeTick::new(BigDecimal::ZERO, TimingFormat::Decimal);
        if times.is_empty() {
            times.insert(zero());
        }
        let (min, max) = (min(&times), max(&times));
        if max.time.signum() > 0 && min.time.signum() < 0 {
            times.insert(zero());
        }
    }

    pub(super) fn scale_in_pixels(&mut self, tick: i64, pixel: i64) {
        self.tick_interval_in_pixels = pixel;
        self.forced_tick_unitary = tick;
    }

    pub(super) fn add_time(&mut self, time: TimeTick) {
        if time.format != TimingFormat::Decimal {
            self.format = time.format.clone();
        }
        self.times.get_mut().insert(time);
    }

    fn min_time(&self) -> f64 {
        min(&self.times.borrow()).time.double_value()
    }

    fn max_time(&self) -> f64 {
        max(&self.times.borrow()).time.double_value()
    }

    fn get_optimal_tick_unit(&self) -> i64 {
        if self.forced_tick_unitary != 0 {
            return self.forced_tick_unitary;
        }
        let hcf_tick_unit = self.calculate_highest_common_factor();
        if hcf_tick_unit == 1 && self.calculate_diagram_width(hcf_tick_unit) > MAX_DIAGRAM_WIDTH {
            // A tick per time unit would make the diagram too wide: ticks then span what fits.
            let total_time_range = self.max_time() - self.min_time();
            return java_round(
                1.0 + (self.tick_interval_in_pixels as f64 * total_time_range / MAX_DIAGRAM_WIDTH),
            );
        }
        hcf_tick_unit
    }

    pub(super) fn get_width(&self) -> f64 {
        if self.times.borrow().is_empty() {
            return 100.0;
        }
        self.calculate_diagram_width(self.get_optimal_tick_unit())
    }

    fn calculate_diagram_width(&self, tick_unitary: i64) -> f64 {
        let delta = self.max_time() - self.min_time();
        (delta / tick_unitary as f64 + 1.0) * self.tick_interval_in_pixels as f64
    }

    /// The highest common factor of the times' integer parts, -1 when they are all zero.
    fn calculate_highest_common_factor(&self) -> i64 {
        let mut absolutes: Vec<i64> = self
            .times
            .borrow()
            .iter()
            .map(|time| time.time.long_value().abs())
            .filter(|&value| value > 0)
            .collect();
        absolutes.sort_unstable_by(|a, b| b.cmp(a));
        absolutes.dedup();
        absolutes
            .into_iter()
            .reduce(compute_highest_common_factor)
            .unwrap_or(-1)
    }

    fn get_nb_tick(&self) -> i64 {
        let times = self.times.borrow();
        if times.is_empty() {
            return 1;
        }
        let delta = max(&times).time.long_value() - min(&times).time.long_value();
        drop(times);
        1000.min(1 + delta / self.get_optimal_tick_unit())
    }

    pub(super) fn get_pos_in_pixel(&self, when: &TimeTick) -> f64 {
        self.get_pos_in_pixel_internal(when.time.double_value())
    }

    pub(super) fn get_pos_in_pixel_internal(&self, time: f64) -> f64 {
        let time = time - self.min_time();
        time / self.get_optimal_tick_unit() as f64 * self.tick_interval_in_pixels as f64
    }

    pub(super) fn draw_time_axis(
        &self,
        ug: &UGraphic,
        strategy: TimeAxisStategy,
        codes: &[(String, TimeTick)],
        skin: &SkinParam,
    ) {
        if strategy == TimeAxisStategy::Hidden {
            return;
        }
        let style_timeline = timing_style(skin, &[SName::Timeline]);
        let ug = ug
            .with_stroke(style_timeline.stroke())
            .with_color(style_timeline.value(PName::LineColor).as_color());
        if strategy == TimeAxisStategy::Automatic {
            self.draw_time_axis_automatic(&ug, skin);
        } else {
            self.draw_time_axis_manual(&ug, codes, skin);
        }
    }

    /// The horizontal line of the axis through as many tick intervals as fit, and their ticks if asked.
    fn draw_axis_line(&self, ug: &UGraphic, first_tick_position: f64, draw_ticks: bool) {
        let interval = self.tick_interval_in_pixels as f64;
        let mut nb = 0;
        while first_tick_position + f64::from(nb) * interval <= self.get_width() {
            if draw_ticks {
                ug.translated(first_tick_position + f64::from(nb) * interval, 0.0)
                    .draw(&UShape::Line {
                        dx: 0.0,
                        dy: TICK_HEIGHT,
                    });
            }
            nb += 1;
        }
        ug.translated(first_tick_position, 0.0).draw(&UShape::Line {
            dx: f64::from(nb - 1) * interval,
            dy: 0.0,
        });
    }

    fn draw_time_axis_manual(&self, ug: &UGraphic, codes: &[(String, TimeTick)], skin: &SkinParam) {
        let first_tick_position =
            self.get_pos_in_pixel_internal(self.get_first_positive_or_zero_value());
        self.draw_axis_line(ug, first_tick_position, false);
        for tick in self.times.borrow().iter() {
            let x = self.get_pos_in_pixel(tick);
            ug.translated(x, 0.0).draw(&UShape::Line {
                dx: 0.0,
                dy: TICK_HEIGHT,
            });
            let label = codes.iter().find(|(_, coded)| coded == tick).map_or_else(
                || self.format.format_decimal(tick.time),
                |(code, _)| code.clone(),
            );
            if label.is_empty() {
                continue;
            }
            let text = time_text_block(&label, skin);
            let dim = text.calculate_dimension(ug.string_bounder());
            text.draw_u(&ug.translated(x - dim.width / 2.0, TICK_HEIGHT + 1.0));
        }
    }

    fn draw_time_axis_automatic(&self, ug: &UGraphic, skin: &SkinParam) {
        let first_tick_position =
            self.get_pos_in_pixel_internal(self.get_first_positive_or_zero_value());
        self.draw_axis_line(ug, first_tick_position, true);
        for round in self.round_values() {
            let text = time_text_block(&self.format.format_long(round), skin);
            let dim = text.calculate_dimension(ug.string_bounder());
            text.draw_u(&ug.translated(
                self.get_pos_in_pixel_internal(round as f64) - dim.width / 2.0,
                TICK_HEIGHT + 1.0,
            ));
        }
    }

    fn get_first_positive_or_zero_value(&self) -> f64 {
        self.times
            .borrow()
            .iter()
            .find(|time| time.time.signum() >= 0)
            .expect("a time from zero on")
            .time
            .double_value()
    }

    fn round_values(&self) -> BTreeSet<i64> {
        let mut result: BTreeSet<i64> = if self.forced_tick_unitary == 0 {
            self.times
                .borrow()
                .iter()
                .map(|tick| tick.time.long_value())
                .collect()
        } else {
            let min = min(&self.times.borrow()).time.long_value();
            (0..=self.get_nb_tick())
                .map(|i| self.forced_tick_unitary * i + min)
                .collect()
        };
        if result.first().is_some_and(|&first| first < 0)
            && result.last().is_some_and(|&last| last > 0)
        {
            result.insert(0);
        }
        result
    }

    pub(super) fn draw_vlines(&self, ug: &UGraphic, height: f64, skin: &SkinParam) {
        let ug = apply_for_vlines(ug, &timing_style(skin, &[SName::Timegrid]));
        for i in 0..=self.get_nb_tick() {
            ug.translated((self.tick_interval_in_pixels * i) as f64, 0.0)
                .draw(&UShape::Line {
                    dx: 0.0,
                    dy: height,
                });
        }
    }

    pub(super) fn get_height(&self, string_bounder: &dyn StringBounder, skin: &SkinParam) -> f64 {
        time_text_block(&self.format.format_long(0), skin)
            .calculate_dimension(string_bounder)
            .height
    }
}

/// A time as the axis writes it.
fn time_text_block(text: &str, skin: &SkinParam) -> impl TextBlock + use<> {
    Display::with_newlines(text).create0(
        &timing_style(skin, &[SName::Timeline]).font_configuration(),
        HorizontalAlignment::Left,
        skin,
        0.0,
        CreoleMode::Full,
    )
}

fn min(times: &BTreeSet<TimeTick>) -> &TimeTick {
    times.first().expect("the ruler has times")
}

fn max(times: &BTreeSet<TimeTick>) -> &TimeTick {
    times.last().expect("the ruler has times")
}

fn compute_highest_common_factor(mut a: i64, mut b: i64) -> i64 {
    let mut r = a;
    while r != 0 {
        r = a % b;
        a = b;
        b = r;
    }
    a.abs()
}

/// `Math.round`: halves round up.
fn java_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

//! How much work a task gets done when: calendars as piecewise constant functions of time, integrated
//! until a task's load is done (PlantUML's `gantt.ngm` and `gantt.solver` packages).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use super::time::TimePoint;
use crate::local_date::{DayOfWeek, LocalDate, LocalDateTime};

/// A fraction of `long`s, reduced, its denominator positive (`Fraction`). Like Java, its arithmetic wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fraction {
    num: i64,
    den: i64,
}

impl Fraction {
    pub(super) const ZERO: Self = Self { num: 0, den: 1 };
    pub(super) const ONE: Self = Self { num: 1, den: 1 };

    pub(super) fn new(numerator: i64, denominator: i64) -> Self {
        let (numerator, denominator) = if denominator < 0 {
            (numerator.wrapping_neg(), denominator.wrapping_neg())
        } else {
            (numerator, denominator)
        };
        let g = gcd(numerator.wrapping_abs(), denominator);
        Self {
            num: numerator / g,
            den: denominator / g,
        }
    }

    fn add(self, other: Self) -> Self {
        Self::new(
            self.num
                .wrapping_mul(other.den)
                .wrapping_add(other.num.wrapping_mul(self.den)),
            self.den.wrapping_mul(other.den),
        )
    }

    fn subtract(self, other: Self) -> Self {
        Self::new(
            self.num
                .wrapping_mul(other.den)
                .wrapping_sub(other.num.wrapping_mul(self.den)),
            self.den.wrapping_mul(other.den),
        )
    }

    fn multiply(self, other: Self) -> Self {
        Self::new(
            self.num.wrapping_mul(other.num),
            self.den.wrapping_mul(other.den),
        )
    }

    fn multiply_by_long(self, scalar: i64) -> Self {
        Self::new(self.num.wrapping_mul(scalar), self.den)
    }

    fn divide(self, other: Self) -> Self {
        self.multiply(Self::new(other.den, other.num))
    }

    fn whole_part(self) -> i64 {
        self.num / self.den
    }

    pub(super) fn is_zero(self) -> bool {
        self.num == 0
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.num
            .wrapping_mul(other.den)
            .cmp(&other.num.wrapping_mul(self.den))
    }
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimeDirection {
    Forward,
    Backward,
}

impl TimeDirection {
    fn adjust_by_seconds(self, time: LocalDateTime, seconds: i64) -> LocalDateTime {
        match self {
            Self::Forward => time.plus_seconds(seconds),
            Self::Backward => time.plus_seconds(-seconds),
        }
    }
}

/// A stretch of time with a constant value, walked forward from its start or backward from it (`Segment`).
#[derive(Clone, Copy, Debug)]
pub(super) struct Segment {
    start: LocalDateTime,
    end: LocalDateTime,
    value: Fraction,
    direction: TimeDirection,
}

impl Segment {
    fn new(
        direction: TimeDirection,
        start: LocalDateTime,
        end: LocalDateTime,
        value: Fraction,
    ) -> Self {
        Self {
            start,
            end,
            value,
            direction,
        }
    }

    fn forward(start: LocalDateTime, end: LocalDateTime, value: Fraction) -> Self {
        Self::new(TimeDirection::Forward, start, end, value)
    }

    fn backward(start: LocalDateTime, end: LocalDateTime, value: Fraction) -> Self {
        Self::new(TimeDirection::Backward, start, end, value)
    }

    /// The overlap of segments of the same direction, their values combined (`intersection`).
    fn intersection(segments: &[Segment], operation: Operation) -> Self {
        let first = segments[0];
        let (mut start, mut end, mut value) = (first.start, first.end, first.value);
        for segment in &segments[1..] {
            if first.direction == TimeDirection::Forward {
                start = start.max(segment.start);
                end = end.min(segment.end);
            } else {
                start = start.min(segment.start);
                end = end.max(segment.end);
            }
            value = operation.apply(value, segment.value);
        }
        Self::new(first.direction, start, end, value)
    }

    fn compute_clamped_start(&self, current: LocalDateTime) -> LocalDateTime {
        match self.direction {
            TimeDirection::Forward => current.max(self.start),
            TimeDirection::Backward => current.min(self.start),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Operation {
    Sum,
    Product,
    Max,
}

impl Operation {
    fn apply(self, a: Fraction, b: Fraction) -> Fraction {
        match self {
            Self::Sum => a.add(b),
            Self::Product => a.multiply(b),
            Self::Max => {
                if a >= b {
                    a
                } else {
                    b
                }
            }
        }
    }
}

/// How much of a full-time person works at each instant (`PiecewiseConstant` and its implementations).
#[derive(Clone, Debug)]
pub(super) enum PiecewiseConstant {
    /// A value per day of the week (`PiecewiseConstantWeekday`).
    Weekday([Fraction; 7]),
    /// A value for some days, another for the others (`PiecewiseConstantSpecificDays`).
    SpecificDays {
        default_value: Fraction,
        days: Rc<HashMap<LocalDate, Fraction>>,
    },
    /// One from `off_before` to `off_after`, zero outside (`PiecewiseConstantTimeWindow`).
    TimeWindow {
        off_before: LocalDateTime,
        off_after: LocalDateTime,
    },
    /// Functions combined instant by instant (`Combiner`).
    Combiner {
        operation: Operation,
        elements: Rc<[PiecewiseConstant]>,
    },
}

/// After this many segments PlantUML gives up a walk through time.
const MAX_SEGMENTS: usize = 9999;

impl PiecewiseConstant {
    pub(super) fn weekday(value: Fraction) -> Self {
        Self::Weekday([value; 7])
    }

    #[must_use]
    pub(super) fn with(&self, day: DayOfWeek, value: Fraction) -> Self {
        let Self::Weekday(values) = self else {
            unreachable!("only a weekday pattern has values per day")
        };
        let mut values = *values;
        values[day.ordinal()] = value;
        Self::Weekday(values)
    }

    pub(super) fn specific_days(
        default_value: Fraction,
        days: HashMap<LocalDate, Fraction>,
    ) -> Self {
        Self::SpecificDays {
            default_value,
            days: Rc::new(days),
        }
    }

    fn combine(operation: Operation, elements: Vec<PiecewiseConstant>) -> Self {
        if elements.len() == 1 {
            return elements.into_iter().next().expect("one element");
        }
        Self::Combiner {
            operation,
            elements: elements.into(),
        }
    }

    pub(super) fn sum(elements: Vec<PiecewiseConstant>) -> Self {
        Self::combine(Operation::Sum, elements)
    }

    pub(super) fn product(elements: Vec<PiecewiseConstant>) -> Self {
        Self::combine(Operation::Product, elements)
    }

    pub(super) fn max(elements: Vec<PiecewiseConstant>) -> Self {
        Self::combine(Operation::Max, elements)
    }

    fn segment_at(&self, instant: LocalDateTime, direction: TimeDirection) -> Segment {
        match self {
            Self::Weekday(values) => day_segment(instant, direction, |day| {
                values[day.day_of_week().ordinal()]
            }),
            Self::SpecificDays {
                default_value,
                days,
            } => day_segment(instant, direction, |day| {
                days.get(&day).copied().unwrap_or(*default_value)
            }),
            Self::TimeWindow {
                off_before,
                off_after,
            } => time_window_segment(*off_before, *off_after, instant, direction),
            Self::Combiner {
                operation,
                elements,
            } => {
                let segments: Vec<Segment> = elements
                    .iter()
                    .map(|element| element.segment_at(instant, direction))
                    .collect();
                Segment::intersection(&segments, *operation)
            }
        }
    }

    /// The segments one after the other from `instant` (`iterateSegmentsFrom`).
    fn segments_from(
        &self,
        instant: LocalDateTime,
        direction: TimeDirection,
    ) -> impl Iterator<Item = Segment> + '_ {
        let mut current = instant;
        std::iter::repeat_with(move || {
            let segment = self.segment_at(current, direction);
            current = segment.end;
            segment
        })
        .take(MAX_SEGMENTS)
    }

    /// Whether nobody works on the day (`PiecewiseConstantUtils.isZeroOnDay`).
    pub(super) fn is_zero_on_day(&self, day: LocalDate) -> bool {
        let start_of_next_day = day.plus_days(1).at_start_of_day();
        self.segments_from(day.at_start_of_day(), TimeDirection::Forward)
            .take_while(|segment| segment.start < start_of_next_day)
            .all(|segment| segment.value.is_zero())
    }
}

/// The day around `instant`, or for a walk backward the day before a midnight.
fn day_segment(
    instant: LocalDateTime,
    direction: TimeDirection,
    value_of: impl Fn(LocalDate) -> Fraction,
) -> Segment {
    match direction {
        TimeDirection::Forward => {
            let day = instant.to_local_date();
            let start = day.at_start_of_day();
            Segment::forward(start, start.plus_days(1), value_of(day))
        }
        TimeDirection::Backward => {
            let day = if instant.is_midnight() {
                instant.to_local_date().plus_days(-1)
            } else {
                instant.to_local_date()
            };
            let end = day.at_start_of_day();
            Segment::backward(end.plus_days(1), end, value_of(day))
        }
    }
}

fn time_window_segment(
    off_before: LocalDateTime,
    off_after: LocalDateTime,
    instant: LocalDateTime,
    direction: TimeDirection,
) -> Segment {
    let min = LocalDateTime::of_epoch_second(i64::MIN);
    let max = LocalDateTime::of_epoch_second(i64::MAX);
    match direction {
        TimeDirection::Forward if instant < off_before => {
            Segment::forward(instant, off_before, Fraction::ZERO)
        }
        TimeDirection::Forward if instant >= off_after => {
            Segment::forward(instant, max, Fraction::ZERO)
        }
        TimeDirection::Forward => Segment::forward(instant, off_after, Fraction::ONE),
        TimeDirection::Backward if instant > off_after => {
            Segment::backward(instant, off_after, Fraction::ZERO)
        }
        TimeDirection::Backward if instant <= off_before => {
            Segment::backward(instant, min, Fraction::ZERO)
        }
        TimeDirection::Backward => Segment::backward(instant, off_before, Fraction::ONE),
    }
}

/// The instant a load is done when worked from `start` at the rates of `load_function`
/// (`LoadIntegrator`).
fn integrate(
    load_function: &PiecewiseConstant,
    load_seconds: i64,
    start: LocalDateTime,
    direction: TimeDirection,
) -> LocalDateTime {
    let mut remaining = Fraction::new(load_seconds, 1);
    let mut current = start;
    for segment in load_function.segments_from(start, direction) {
        if remaining == Fraction::ZERO {
            break;
        }
        let rate = segment.value;
        if rate == Fraction::ZERO {
            current = segment.end;
            continue;
        }
        let effective_start = segment.compute_clamped_start(current);
        let seconds = (segment.end.epoch_second() - effective_start.epoch_second()).abs();
        let segment_load = rate.multiply_by_long(seconds);
        if segment_load >= remaining {
            let seconds_needed = remaining.divide(rate);
            return direction.adjust_by_seconds(effective_start, seconds_needed.whole_part());
        }
        remaining = remaining.subtract(segment_load);
        current = segment.end;
    }
    current
}

/// What a task's dates are computed from (`Solver`): the last two of its start, end and load that were set.
/// A start set before a later one wins over it.
#[derive(Clone, Debug, Default)]
pub(super) struct Solver {
    values: Vec<SolverValue>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SolverValue {
    Start(TimePoint),
    End(TimePoint),
    /// The work, in seconds of a full-time person.
    Load(i64),
}

impl SolverValue {
    fn same_attribute(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

impl Solver {
    pub(super) fn set_data(&mut self, value: SolverValue) {
        let mut value = value;
        if let Some(index) = self.values.iter().position(|v| v.same_attribute(value)) {
            let previous = self.values.remove(index);
            if let (SolverValue::Start(previous), SolverValue::Start(new)) = (previous, value)
                && previous > new
            {
                value = SolverValue::Start(previous);
            }
        }
        self.values.push(value);
        if self.values.len() > 2 {
            self.values.remove(0);
        }
    }

    fn start(&self) -> Option<TimePoint> {
        self.values.iter().find_map(|value| match value {
            SolverValue::Start(start) => Some(*start),
            _ => None,
        })
    }

    fn end(&self) -> Option<TimePoint> {
        self.values.iter().find_map(|value| match value {
            SolverValue::End(end) => Some(*end),
            _ => None,
        })
    }

    /// The load set, else one day's.
    pub(super) fn load(&self) -> i64 {
        self.values
            .iter()
            .find_map(|value| match value {
                SolverValue::Load(load) => Some(*load),
                _ => None,
            })
            .unwrap_or(86_400)
    }

    pub(super) fn get_start(&self, allocation: &PiecewiseConstant) -> TimePoint {
        self.start().unwrap_or_else(|| {
            let end = self.end().expect("a task has two of start, end and load");
            TimePoint::of(integrate(
                allocation,
                self.load(),
                end.to_local_date_time(),
                TimeDirection::Backward,
            ))
        })
    }

    pub(super) fn get_end(&self, allocation: &PiecewiseConstant) -> TimePoint {
        self.end().unwrap_or_else(|| {
            let start = self.start().expect("a task has two of start, end and load");
            TimePoint::of(integrate(
                allocation,
                self.load(),
                start.to_local_date_time(),
                TimeDirection::Forward,
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i64, month: i64, day: i64) -> LocalDate {
        LocalDate::of(year, month, day).unwrap()
    }

    #[test]
    fn fractions_reduce_and_compare() {
        assert_eq!(Fraction::new(50, 100), Fraction::new(1, 2));
        assert_eq!(Fraction::new(1, -2), Fraction::new(-1, 2));
        assert!(Fraction::new(1, 3) < Fraction::new(1, 2));
        assert_eq!(Fraction::new(7, 2).whole_part(), 3);
    }

    #[test]
    fn loads_skip_closed_days() {
        let calendar = PiecewiseConstant::weekday(Fraction::ONE)
            .with(DayOfWeek::Saturday, Fraction::ZERO)
            .with(DayOfWeek::Sunday, Fraction::ZERO);
        let friday = day(2021, 1, 1).at_start_of_day();
        let end = integrate(&calendar, 2 * 86_400, friday, TimeDirection::Forward);
        assert_eq!(end, day(2021, 1, 5).at_start_of_day());
        let start = integrate(&calendar, 2 * 86_400, end, TimeDirection::Backward);
        assert_eq!(start, day(2021, 1, 1).at_start_of_day());
        assert!(calendar.is_zero_on_day(day(2021, 1, 2)));
        assert!(!calendar.is_zero_on_day(day(2021, 1, 4)));
    }

    #[test]
    fn half_time_takes_twice_as_long() {
        let half = PiecewiseConstant::product(vec![
            PiecewiseConstant::weekday(Fraction::ONE),
            PiecewiseConstant::specific_days(Fraction::new(1, 2), HashMap::new()),
        ]);
        let start = day(2020, 7, 1).at_start_of_day();
        let end = integrate(&half, 86_400, start, TimeDirection::Forward);
        assert_eq!(end, day(2020, 7, 3).at_start_of_day());
    }

    #[test]
    fn the_solver_keeps_the_last_two_values() {
        let mut solver = Solver::default();
        let start = TimePoint::of_start_of_day(day(2020, 7, 1));
        solver.set_data(SolverValue::Start(start));
        solver.set_data(SolverValue::Load(3 * 86_400));
        let calendar = PiecewiseConstant::weekday(Fraction::ONE);
        assert_eq!(
            solver.get_end(&calendar),
            TimePoint::of_start_of_day(day(2020, 7, 4))
        );
        solver.set_data(SolverValue::End(TimePoint::of_start_of_day(day(
            2020, 7, 10,
        ))));
        assert_eq!(
            solver.get_start(&calendar),
            TimePoint::of_start_of_day(day(2020, 7, 7))
        );
        assert_eq!(solver.load(), 3 * 86_400);
    }
}

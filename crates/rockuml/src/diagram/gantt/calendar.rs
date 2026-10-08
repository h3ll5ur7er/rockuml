//! Which days people work, and how days are coloured and named (PlantUML's `OpenClose` and
//! `DayCalendarData`).

use std::collections::{HashMap, HashSet};

use super::ngm::{Fraction, PiecewiseConstant};
use super::time::TimePoint;
use crate::color::HColor;
use crate::local_date::{DayOfWeek, LocalDate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DayStatus {
    Open,
    Close,
}

/// Days of the week and dates opened or closed, and a time before or after which nobody works
/// (`OpenClose`).
#[derive(Clone, Debug, Default)]
pub(super) struct OpenClose {
    weekday_status: [Option<DayStatus>; 7],
    day_status: HashMap<LocalDate, DayStatus>,
    off_before: Option<LocalDate>,
    off_after: Option<LocalDate>,
}

impl OpenClose {
    /// A copy with the days `except` sets overriding these (`mutateMe`).
    pub(super) fn mutate_me(&self, except: Option<&OpenClose>) -> Self {
        let mut result = self.clone();
        if let Some(except) = except {
            for (status, other) in result.weekday_status.iter_mut().zip(except.weekday_status) {
                if other.is_some() {
                    *status = other;
                }
            }
            result.day_status.extend(
                except
                    .day_status
                    .iter()
                    .map(|(day, status)| (*day, *status)),
            );
        }
        result
    }

    pub(super) fn days_in_week(&self) -> i64 {
        7 - self
            .weekday_status
            .iter()
            .filter(|status| **status == Some(DayStatus::Close))
            .count() as i64
    }

    pub(super) fn is_closed(&self, day: LocalDate) -> bool {
        self.local_status(day) == Some(DayStatus::Close)
    }

    fn local_status(&self, day: LocalDate) -> Option<DayStatus> {
        if self.off_before.is_some_and(|off_before| day < off_before)
            || self.off_after.is_some_and(|off_after| day > off_after)
        {
            return Some(DayStatus::Close);
        }
        self.day_status
            .get(&day)
            .copied()
            .or(self.weekday_status[day.day_of_week().ordinal()])
    }

    pub(super) fn close_day_of_week(&mut self, day: DayOfWeek) {
        self.weekday_status[day.ordinal()] = Some(DayStatus::Close);
    }

    pub(super) fn open_day_of_week(&mut self, day: DayOfWeek) {
        self.weekday_status[day.ordinal()] = Some(DayStatus::Open);
    }

    pub(super) fn close(&mut self, day: LocalDate) {
        self.day_status.insert(day, DayStatus::Close);
    }

    pub(super) fn open(&mut self, day: LocalDate) {
        self.day_status.insert(day, DayStatus::Open);
    }

    pub(super) fn set_off_before_date(&mut self, day: LocalDate) {
        self.off_before = Some(day);
    }

    pub(super) fn set_off_after_date(&mut self, day: LocalDate) {
        self.off_after = Some(day);
    }

    /// Full time on open days, nothing on closed ones (`asPiecewiseConstant`).
    pub(super) fn as_piecewise_constant(&self) -> PiecewiseConstant {
        let mut week_pattern = PiecewiseConstant::weekday(Fraction::ONE);
        for day in DayOfWeek::ALL {
            if self.weekday_status[day.ordinal()] == Some(DayStatus::Close) {
                week_pattern = week_pattern.with(day, Fraction::ZERO);
            }
        }
        let mut result = week_pattern.clone();
        if !self.day_status.is_empty() {
            let days = |status, value| {
                self.day_status
                    .iter()
                    .filter(|(_, s)| **s == status)
                    .map(|(day, _)| (*day, value))
                    .collect()
            };
            let closed_days = PiecewiseConstant::specific_days(
                Fraction::ONE,
                days(DayStatus::Close, Fraction::ZERO),
            );
            let open_days = PiecewiseConstant::specific_days(
                Fraction::ZERO,
                days(DayStatus::Open, Fraction::ONE),
            );
            result = PiecewiseConstant::max(vec![
                open_days,
                PiecewiseConstant::product(vec![week_pattern, closed_days]),
            ]);
        }
        if self.off_before.is_none() && self.off_after.is_none() {
            return result;
        }
        let off_before = self.off_before.map_or(
            crate::local_date::LocalDateTime::of_epoch_second(i64::MIN),
            LocalDate::at_start_of_day,
        );
        let off_after = self.off_after.map_or(
            crate::local_date::LocalDateTime::of_epoch_second(i64::MAX),
            |day| day.plus_days(1).at_start_of_day(),
        );
        PiecewiseConstant::product(vec![
            result,
            PiecewiseConstant::TimeWindow {
                off_before,
                off_after,
            },
        ])
    }
}

/// The calendar of the project: open and closed days, their colours and names, and the vertical
/// separators between them (`DayCalendarData`).
#[derive(Default)]
pub(super) struct DayCalendarData {
    pub(super) open_close: OpenClose,
    name_days: HashMap<TimePoint, String>,
    color_days_today: HashMap<TimePoint, HColor>,
    color_days_internal: HashMap<TimePoint, HColor>,
    color_days_of_week: HashMap<DayOfWeek, HColor>,
    vertical_separator_before: HashSet<LocalDate>,
    open_close_for_task: HashMap<String, OpenClose>,
}

impl DayCalendarData {
    pub(super) fn is_open(&self, day: LocalDate) -> bool {
        !self.open_close.is_closed(day)
    }

    pub(super) fn day_color(&self, day: TimePoint) -> Option<&HColor> {
        self.color_days_today
            .get(&day)
            .or_else(|| self.color_days_internal.get(&day))
    }

    pub(super) fn day_of_week_color(&self, day: DayOfWeek) -> Option<&HColor> {
        self.color_days_of_week.get(&day)
    }

    pub(super) fn day_name(&self, day: TimePoint) -> Option<&str> {
        self.name_days.get(&day).map(String::as_str)
    }

    pub(super) fn color_days(&self) -> impl Iterator<Item = TimePoint> + '_ {
        self.color_days_internal.keys().copied()
    }

    pub(super) fn name_days(&self) -> &HashMap<TimePoint, String> {
        &self.name_days
    }

    pub(super) fn has_separator_before(&self, day: LocalDate) -> bool {
        self.vertical_separator_before.contains(&day)
    }

    pub(super) fn add_separator_before(&mut self, day: LocalDate) {
        self.vertical_separator_before.insert(day);
    }

    pub(super) fn put_name_day(&mut self, day: TimePoint, name: &str) {
        self.name_days.insert(day, name.to_owned());
    }

    pub(super) fn put_color_day_today(&mut self, day: TimePoint, color: HColor) {
        self.color_days_today.insert(day, color);
    }

    pub(super) fn put_color_day(&mut self, day: TimePoint, color: HColor) {
        self.color_days_internal.insert(day, color);
    }

    pub(super) fn put_color_day_of_week(&mut self, day: DayOfWeek, color: HColor) {
        self.color_days_of_week.insert(day, color);
    }

    /// The days closed or opened for one task only.
    pub(super) fn open_close_for_task(&mut self, task: &str) -> &mut OpenClose {
        self.open_close_for_task.entry(task.to_owned()).or_default()
    }

    /// The calendar of a task: the project's, with the task's own days over it (`getLoadPlanableForTask`).
    pub(super) fn load_planable_for_task(&self, task: &str) -> PiecewiseConstant {
        self.open_close
            .mutate_me(self.open_close_for_task.get(task))
            .as_piecewise_constant()
    }

    pub(super) fn default_plan(&self) -> PiecewiseConstant {
        self.open_close.as_piecewise_constant()
    }
}

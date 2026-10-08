//! The tasks of a project, the people working on them and the links between them (PlantUML's `gantt.core`
//! package, `GanttModelData` and `GanttConstraint`).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};

use super::calendar::{DayCalendarData, OpenClose};
use super::ngm::{Fraction, PiecewiseConstant, Solver, SolverValue};
use super::time::TimePoint;
use crate::color::HColor;
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType, WithLinkType};
use crate::klimt::url::Url;
use crate::local_date::{DayOfWeek, LocalDate};
use crate::stereo::Stereotype;

pub(super) type TaskId = usize;

/// How a task is known: its id, by which other lines name it, and what it shows (`TaskCode`).
#[derive(Clone, Debug)]
pub(super) struct TaskCode {
    pub(super) id: String,
    pub(super) display: String,
}

impl TaskCode {
    pub(super) fn from_id(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            display: id.to_owned(),
        }
    }

    /// `[Display] as [id]`: without an id, the display is the id.
    pub(super) fn from_id_and_display(id: Option<&str>, display: &str) -> Self {
        Self {
            id: id.unwrap_or(display).to_owned(),
            display: display.to_owned(),
        }
    }
}

/// A fill colour and an outline colour (`CenterBorderColor`).
#[derive(Clone, Debug)]
pub(super) struct CenterBorderColor {
    pub(super) center: Option<HColor>,
    pub(super) border: Option<HColor>,
    pub(super) style: Option<String>,
}

impl CenterBorderColor {
    pub(super) fn new(center: Option<HColor>, border: Option<HColor>) -> Self {
        Self {
            center,
            border,
            style: None,
        }
    }

    /// The colours as far between this and `other` as the task is completed.
    fn unlinear_to(&self, other: &Self, completion: i32) -> Self {
        let blend = |a: &Option<HColor>, b: &Option<HColor>| match (completion, a, b) {
            (0, _, _) => a.clone(),
            (100, _, _) => b.clone(),
            (_, Some(a), Some(b)) => Some(HColor::unlinear(a, b, completion)),
            _ => a.clone(),
        };
        Self {
            center: blend(&self.center, &other.center),
            border: blend(&self.border, &other.border),
            style: self.style.clone(),
        }
    }
}

pub(super) struct Task {
    pub(super) code: TaskCode,
    /// The task whose row this one is drawn on.
    pub(super) row: Option<TaskId>,
    /// What a milestone shows instead of its diamond.
    pub(super) display_string: Option<String>,
    pub(super) stereotype: Option<Stereotype>,
    pub(super) kind: TaskKind,
}

pub(super) enum TaskKind {
    Impl(Box<TaskImpl>),
    Group(TaskGroup),
    Separator(TaskSeparator),
}

/// A task with a start, an end and a load (`TaskImpl`). Like PlantUML, it computes its calendar, start and
/// end once and keeps them until the task itself changes.
pub(super) struct TaskImpl {
    paused_day: BTreeSet<LocalDate>,
    paused_day_of_week: BTreeSet<DayOfWeek>,
    solver: Solver,
    /// The people on the task and their percentage, in the order they joined.
    pub(super) resources: Vec<(String, i32)>,
    pub(super) diamond: bool,
    cached_piecewise_constant: RefCell<Option<PiecewiseConstant>>,
    cached_start: Cell<Option<TimePoint>>,
    cached_end: Cell<Option<TimePoint>>,
    pub(super) completion: i32,
    pub(super) note: Option<(Display, Option<Stereotype>)>,
    pub(super) url: Option<Url>,
    colors: Vec<CenterBorderColor>,
}

/// Tasks shown together under one bar (`TaskGroup`).
pub(super) struct TaskGroup {
    pub(super) parent: Option<TaskId>,
    pub(super) children: Vec<TaskId>,
}

/// A line across the chart, with an optional comment (`TaskSeparator`).
pub(super) struct TaskSeparator {
    pub(super) comment: Option<String>,
}

impl TaskImpl {
    pub(super) fn new(start: TimePoint, completion: i32) -> Self {
        let mut solver = Solver::default();
        solver.set_data(SolverValue::Start(start));
        solver.set_data(SolverValue::Load(86_400));
        Self {
            paused_day: BTreeSet::new(),
            paused_day_of_week: BTreeSet::new(),
            solver,
            resources: Vec::new(),
            diamond: false,
            cached_piecewise_constant: RefCell::new(None),
            cached_start: Cell::new(None),
            cached_end: Cell::new(None),
            completion,
            note: None,
            url: None,
            colors: Vec::new(),
        }
    }

    fn invalidate_cache(&mut self) {
        *self.cached_piecewise_constant.get_mut() = None;
        self.cached_start.set(None);
        self.cached_end.set(None);
    }

    pub(super) fn set_solver(&mut self, value: SolverValue) {
        self.solver.set_data(value);
        self.invalidate_cache();
    }

    pub(super) fn add_pause(&mut self, day: LocalDate) {
        self.paused_day.insert(day);
        self.invalidate_cache();
    }

    pub(super) fn add_pause_day_of_week(&mut self, day: DayOfWeek) {
        self.paused_day_of_week.insert(day);
        self.invalidate_cache();
    }

    /// Joins `resource`, or changes its percentage.
    pub(super) fn add_resource(&mut self, resource: &str, percentage: i32) {
        match self.resources.iter_mut().find(|(name, _)| name == resource) {
            Some(entry) => entry.1 = percentage,
            None => self.resources.push((resource.to_owned(), percentage)),
        }
        self.invalidate_cache();
    }

    pub(super) fn set_colors(&mut self, colors: Vec<CenterBorderColor>) {
        self.colors = colors;
    }

    /// The colours set, between the first and second as far as the task is completed.
    pub(super) fn colors(&self) -> Option<CenterBorderColor> {
        match self.colors.as_slice() {
            [] => None,
            [only] => Some(only.clone()),
            [first, second, ..] => Some(first.unlinear_to(second, self.completion)),
        }
    }

    fn is_paused(&self, instant: TimePoint) -> bool {
        self.paused_day.contains(&instant.to_day())
            || self.paused_day_of_week.contains(&instant.to_day_of_week())
    }

    /// The task's own pauses (`localPause`).
    fn local_pause(&self) -> PiecewiseConstant {
        let mut week_pattern = PiecewiseConstant::weekday(Fraction::ONE);
        for day in &self.paused_day_of_week {
            week_pattern = week_pattern.with(*day, Fraction::ZERO);
        }
        if self.paused_day.is_empty() {
            return week_pattern;
        }
        let closed_days = self
            .paused_day
            .iter()
            .map(|day| (*day, Fraction::ZERO))
            .collect();
        PiecewiseConstant::product(vec![
            week_pattern,
            PiecewiseConstant::specific_days(Fraction::ONE, closed_days),
        ])
    }
}

/// Somebody working on tasks, with their own days off (`Resource`).
#[derive(Default)]
pub(super) struct Resource {
    pub(super) open_close: OpenClose,
}

/// Where a task instant is taken: on a task, or on the days named with `are named [name]`.
#[derive(Clone, Copy, Debug)]
pub(super) enum Moment {
    Task(TaskId),
    Days { start: TimePoint, end: TimePoint },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TaskAttribute {
    Start,
    End,
}

/// Whether closed days count when moving an instant by days (`GanttConstraintMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GanttConstraintMode {
    IgnoreCalendar,
    DoNotCountCloseDay,
}

/// The start or end of a moment, maybe some days later or earlier (`TaskInstant`).
#[derive(Clone, Debug)]
pub(super) struct TaskInstant {
    pub(super) moment: Moment,
    pub(super) attribute: TaskAttribute,
    delta: i64,
    mode: GanttConstraintMode,
    calendar: Option<PiecewiseConstant>,
}

impl TaskInstant {
    pub(super) fn new(moment: Moment, attribute: TaskAttribute) -> Self {
        Self {
            moment,
            attribute,
            delta: 0,
            mode: GanttConstraintMode::IgnoreCalendar,
            calendar: None,
        }
    }

    #[must_use]
    pub(super) fn with_delta(
        &self,
        delta: i64,
        mode: GanttConstraintMode,
        calendar: PiecewiseConstant,
    ) -> Self {
        Self {
            delta,
            mode,
            calendar: Some(calendar),
            ..self.clone()
        }
    }

    pub(super) fn is_task(&self) -> bool {
        matches!(self.moment, Moment::Task(_))
    }

    pub(super) fn task(&self) -> Option<TaskId> {
        match self.moment {
            Moment::Task(task) => Some(task),
            Moment::Days { .. } => None,
        }
    }

    fn manage_delta(&self, mut value: TimePoint) -> TimePoint {
        let mut added = 0;
        while added < self.delta {
            value = value.increment();
            let closed = self.mode == GanttConstraintMode::DoNotCountCloseDay
                && self
                    .calendar
                    .as_ref()
                    .is_some_and(|calendar| calendar.is_zero_on_day(value.to_day()));
            if !closed {
                added += 1;
            }
        }
        for _ in 0..-self.delta {
            value = value.decrement();
        }
        value
    }
}

/// A link from one task instant to another, drawn as an arrow (`GanttConstraint`).
pub(super) struct GanttConstraint {
    pub(super) source: TaskInstant,
    pub(super) dest: TaskInstant,
    pub(super) link_type: LinkType,
    pub(super) specific_color: Option<HColor>,
}

impl GanttConstraint {
    pub(super) fn new(
        source: TaskInstant,
        dest: TaskInstant,
        forced_color: Option<HColor>,
    ) -> Self {
        Self {
            source,
            dest,
            link_type: LinkType::new(LinkDecor::None, LinkDecor::None),
            specific_color: forced_color,
        }
    }

    pub(super) fn is_on(&self, task: TaskId) -> bool {
        self.source.task() == Some(task) || self.dest.task() == Some(task)
    }

    /// Whether the arrow ends on the task's right side, where its label would go.
    pub(super) fn is_there_right_arrow(&self, task: TaskId) -> bool {
        (self.dest.task() == Some(task) && self.dest.attribute == TaskAttribute::End)
            || (self.source.task() == Some(task)
                && self.dest.attribute == TaskAttribute::End
                && self.source.attribute == TaskAttribute::End)
    }
}

impl WithLinkType for GanttConstraint {
    fn link_type_mut(&mut self) -> &mut LinkType {
        &mut self.link_type
    }

    fn set_specific_color(&mut self, color: HColor, i: usize) {
        if i == 0 {
            self.specific_color = Some(color);
        }
    }
}

/// Tasks in the order they were declared, people, links and the calendar (`GanttModelData` with what the
/// tasks compute their dates from).
#[derive(Default)]
pub(super) struct GanttModel {
    pub(super) tasks: Vec<Task>,
    /// By name, in the order they were named.
    pub(super) resources: Vec<(String, Resource)>,
    pub(super) constraints: Vec<GanttConstraint>,
    pub(super) calendar: DayCalendarData,
}

/// After this many days PlantUML would still look for an open day; rockuml gives up.
const MAX_CLOSED_DAYS: usize = 100_000;

impl GanttModel {
    pub(super) fn task_by_id(&self, id: &str) -> Option<TaskId> {
        self.tasks.iter().position(|task| task.code.id == id)
    }

    pub(super) fn task_impl(&self, task: TaskId) -> Option<&TaskImpl> {
        match &self.tasks[task].kind {
            TaskKind::Impl(task) => Some(task),
            _ => None,
        }
    }

    pub(super) fn task_impl_mut(&mut self, task: TaskId) -> Option<&mut TaskImpl> {
        match &mut self.tasks[task].kind {
            TaskKind::Impl(task) => Some(task),
            _ => None,
        }
    }

    pub(super) fn resource(&self, name: &str) -> Option<&Resource> {
        self.resources
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, resource)| resource)
    }

    /// The task's calendar: the project's and its own days, times its pauses or its people's time
    /// (`asPiecewiseConstant`).
    pub(super) fn task_piecewise_constant(&self, task: TaskId) -> PiecewiseConstant {
        let task_impl = self.task_impl(task).expect("only tasks have calendars");
        if let Some(cached) = task_impl.cached_piecewise_constant.borrow().as_ref() {
            return cached.clone();
        }
        let default_plan = self.default_plan_for(task);
        let result = if task_impl.resources.is_empty() {
            PiecewiseConstant::product(vec![default_plan, task_impl.local_pause()])
        } else {
            PiecewiseConstant::product(vec![default_plan, self.all_resources(task_impl)])
        };
        *task_impl.cached_piecewise_constant.borrow_mut() = Some(result.clone());
        result
    }

    pub(super) fn default_plan_for(&self, task: TaskId) -> PiecewiseConstant {
        self.calendar
            .load_planable_for_task(&self.tasks[task].code.id)
    }

    /// The time of all the task's people together, each at their percentage (`allRessources`).
    fn all_resources(&self, task_impl: &TaskImpl) -> PiecewiseConstant {
        let contributions = task_impl
            .resources
            .iter()
            .map(|(name, percentage)| {
                let availability = self
                    .resource(name)
                    .map(|resource| resource.open_close.as_piecewise_constant())
                    .expect("a task's people are known");
                PiecewiseConstant::product(vec![
                    availability,
                    PiecewiseConstant::specific_days(
                        Fraction::new(i64::from(*percentage), 100),
                        HashMap::new(),
                    ),
                ])
            })
            .collect();
        PiecewiseConstant::product(vec![
            task_impl.local_pause(),
            PiecewiseConstant::sum(contributions),
        ])
    }

    /// When the task starts: a group when its first task does (`getStart`).
    pub(super) fn start(&self, task: TaskId) -> TimePoint {
        match &self.tasks[task].kind {
            TaskKind::Impl(task_impl) => {
                if let Some(start) = task_impl.cached_start.get() {
                    return start;
                }
                let calendar = self.task_piecewise_constant(task);
                let mut result = task_impl.solver.get_start(&calendar);
                if !task_impl.diamond {
                    for _ in 0..MAX_CLOSED_DAYS {
                        if !calendar.is_zero_on_day(result.to_day()) {
                            break;
                        }
                        result = result.increment();
                    }
                }
                task_impl.cached_start.set(Some(result));
                result
            }
            TaskKind::Group(group) => group
                .children
                .iter()
                .map(|&child| self.start(child))
                .min()
                .expect("a group's dates come from its tasks"),
            TaskKind::Separator(_) => unreachable!("a separator has no dates"),
        }
    }

    /// When the task ends, exclusive: a group when its last task does (`getEnd`).
    pub(super) fn end(&self, task: TaskId) -> TimePoint {
        match &self.tasks[task].kind {
            TaskKind::Impl(task_impl) => {
                if let Some(end) = task_impl.cached_end.get() {
                    return end;
                }
                let result = task_impl
                    .solver
                    .get_end(&self.task_piecewise_constant(task));
                task_impl.cached_end.set(Some(result));
                result
            }
            TaskKind::Group(group) => group
                .children
                .iter()
                .map(|&child| self.end(child))
                .max()
                .expect("a group's dates come from its tasks"),
            TaskKind::Separator(_) => unreachable!("a separator has no dates"),
        }
    }

    pub(super) fn end_minus_one_day(&self, task: TaskId) -> TimePoint {
        self.end(task).decrement()
    }

    pub(super) fn moment_start(&self, moment: Moment) -> TimePoint {
        match moment {
            Moment::Task(task) => self.start(task),
            Moment::Days { start, .. } => start,
        }
    }

    pub(super) fn moment_end(&self, moment: Moment) -> TimePoint {
        match moment {
            Moment::Task(task) => self.end(task),
            Moment::Days { end, .. } => end,
        }
    }

    /// The instant itself (`getInstantPrecise`).
    pub(super) fn instant_precise(&self, instant: &TaskInstant) -> TimePoint {
        let value = match instant.attribute {
            TaskAttribute::Start => self.moment_start(instant.moment),
            TaskAttribute::End => self.moment_end(instant.moment),
        };
        instant.manage_delta(value)
    }

    /// Whether both instants are on tasks drawn on the same row.
    pub(super) fn same_row(&self, source: &TaskInstant, dest: &TaskInstant) -> bool {
        match (source.task(), dest.task()) {
            (Some(t1), Some(t2)) => {
                self.tasks[t2].row == Some(t1) || self.tasks[t1].row == Some(t2)
            }
            _ => false,
        }
    }

    /// The days the task is paused or closed on (`getAllPaused`).
    pub(super) fn all_paused(&self, task: TaskId) -> BTreeSet<LocalDate> {
        let task_impl = self.task_impl(task).expect("only tasks pause");
        let mut result = task_impl.paused_day.clone();
        if !task_impl.paused_day_of_week.is_empty() {
            let end = self.end_minus_one_day(task);
            let mut current = self.start(task);
            while current <= end {
                if task_impl
                    .paused_day_of_week
                    .contains(&current.to_day_of_week())
                {
                    result.insert(current.to_day());
                }
                current = current.increment();
            }
        }
        result
    }

    /// The percentage of `resource` the task takes on the day (`loadForResource`).
    fn load_for_resource(&self, task: TaskId, resource: &str, instant: TimePoint) -> i32 {
        let Some(task_impl) = self.task_impl(task) else {
            return 0;
        };
        let Some(&(_, percentage)) = task_impl
            .resources
            .iter()
            .find(|(name, _)| name == resource)
        else {
            return 0;
        };
        if instant < self.start(task) || instant > self.end_minus_one_day(task) {
            return 0;
        }
        let closed = self
            .resource(resource)
            .is_some_and(|resource| resource.open_close.is_closed(instant.to_day()));
        if task_impl.is_paused(instant) || closed {
            0
        } else {
            percentage
        }
    }

    /// The percentage `resource` works on the day, all tasks together (`getLoadForResource`).
    pub(super) fn load_for_resource_at(&self, resource: &str, instant: TimePoint) -> i32 {
        (0..self.tasks.len())
            .map(|task| self.load_for_resource(task, resource, instant))
            .sum()
    }

    pub(super) fn constraints_for_task(&self, task: TaskId) -> Vec<&GanttConstraint> {
        self.constraints
            .iter()
            .filter(|constraint| constraint.is_on(task))
            .collect()
    }
}

//! Gantt charts, `@startgantt` (PlantUML's `gantt` package).

mod calendar;
mod commands;
mod draw;
mod header;
mod i18n_time_data;
mod lang;
mod model;
mod ngm;
mod table;
#[cfg(test)]
mod tests;
mod time;

use std::collections::BTreeSet;
use std::rc::Rc;

use model::{
    CenterBorderColor, GanttConstraint, GanttModel, Moment, Resource, Task, TaskAttribute,
    TaskCode, TaskGroup, TaskId, TaskImpl, TaskInstant, TaskKind, TaskSeparator,
};
use ngm::SolverValue;
use time::TimePoint;

use super::builder::CommandFactory;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::HColor;
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult, ParserPass};
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::local_date::{DayOfWeek, LocalDate, LocalDateTime};
use crate::stereo::Stereotype;
use crate::style::SName;

/// How much time a column of the chart stands for (`PrintScale`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrintScale {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl PrintScale {
    fn from_string(value: &str) -> Self {
        match value.chars().next() {
            Some('w') => Self::Weekly,
            Some('m') => Self::Monthly,
            Some('q') => Self::Quarterly,
            Some('y') => Self::Yearly,
            _ => Self::Daily,
        }
    }

    /// How narrow a day is drawn (`getDefaultScale`).
    fn default_scale(self) -> f64 {
        let compress = match self {
            Self::Daily => 1,
            Self::Weekly => 4,
            Self::Monthly => 15,
            Self::Quarterly => 40,
            Self::Yearly => 60,
        };
        1.0 / f64::from(compress)
    }
}

/// What the weekly scale writes for each week instead of its number in the year (`WeeklyHeaderStrategy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WeeklyHeaderStrategy {
    DayOfMonth,
    FromN,
}

/// The columns of the task table (`GanttTaskTableColumn`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Column {
    Task,
    Start,
    End,
    Duration,
}

impl Column {
    const ALL: [Self; 4] = [Self::Task, Self::Start, Self::End, Self::Duration];

    fn name(self) -> &'static str {
        match self {
            Self::Task => "TASK",
            Self::Start => "START",
            Self::End => "END",
            Self::Duration => "DURATION",
        }
    }

    fn of(what: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|column| column.name().eq_ignore_ascii_case(what))
    }
}

/// What `hide footbox` and `hide resources names` or `footbox` hide (`DisplayConfigData`).
struct DisplayConfig {
    show_footbox: bool,
    hide_resource_name: bool,
    hide_resource_footbox: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_footbox: true,
            hide_resource_name: false,
            hide_resource_footbox: false,
        }
    }
}

pub(super) struct GanttDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    model: GanttModel,
    /// The first day of the project, 1970-01-01 when it has none.
    min_day: LocalDate,
    print_start: Option<LocalDate>,
    print_end: Option<LocalDate>,
    print_scale: PrintScale,
    factor_scale: f64,
    hide_closed: bool,
    language: String,
    week_number_strategy: (DayOfWeek, i64),
    weekly_header_strategy: Option<WeeklyHeaderStrategy>,
    week_starting_number: i64,
    display: DisplayConfig,
    displayed_columns: BTreeSet<Column>,
    today: Option<TimePoint>,
    default_completion: i32,
    /// The task `it` stands for: the last one a sentence named.
    it: Option<TaskId>,
    /// Who `they` stands for.
    they: Option<String>,
    current_group: Option<TaskId>,
}

/// Reads gantt charts (PlantUML's `GanttDiagramFactory`).
pub(super) struct GanttDiagramFactory;

impl CommandFactory for GanttDiagramFactory {
    type Diagram = GanttDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Gantt;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> GanttDiagram {
        GanttDiagram {
            source: source.clone(),
            titled: Titled::new(SName::GanttDiagram, "GANTT", source),
            model: GanttModel::default(),
            min_day: LocalDate::EPOCH,
            print_start: None,
            print_end: None,
            print_scale: PrintScale::Daily,
            factor_scale: 1.0,
            hide_closed: false,
            language: "en".to_owned(),
            week_number_strategy: (DayOfWeek::Monday, 4),
            weekly_header_strategy: None,
            week_starting_number: 0,
            display: DisplayConfig::default(),
            displayed_columns: [Column::Start, Column::End, Column::Duration].into(),
            today: None,
            default_completion: 100,
            it: None,
            they: None,
            current_group: None,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<GanttDiagram>>> {
        commands::all()
    }
}

/// The first and last days drawn.
#[derive(Clone, Copy)]
struct TimeBounds {
    pub(super) min_day: LocalDate,
    pub(super) max_day: LocalDate,
    /// Whether `print between` cut the chart.
    pub(super) printed_interval: bool,
}

impl TimeBounds {
    /// Whether the task lies outside the printed days (`isHidden`).
    pub(super) fn is_hidden(&self, model: &GanttModel, task: TaskId) -> bool {
        if !self.printed_interval || matches!(model.tasks[task].kind, TaskKind::Separator(_)) {
            return false;
        }
        model.end_minus_one_day(task) < TimePoint::of_start_of_day(self.min_day)
            || model.start(task) > TimePoint::of_end_of_day_minus_one_second(self.max_day)
    }

    /// Where a task starts drawing, cut to the printed days (`getStartForDrawing`).
    pub(super) fn start_for_drawing(&self, model: &GanttModel, task: TaskId) -> TimePoint {
        let start = model.start(task);
        if self.printed_interval {
            TimePoint::of_start_of_day(self.min_day).max(start)
        } else {
            start
        }
    }

    pub(super) fn end_for_drawing(&self, model: &GanttModel, task: TaskId) -> TimePoint {
        let end = model.end(task);
        if self.printed_interval {
            TimePoint::of_start_of_day(self.max_day.plus_days(1)).min(end)
        } else {
            end
        }
    }
}

impl GanttDiagram {
    fn is_relative(&self) -> bool {
        self.min_day == LocalDate::EPOCH
    }

    fn min_day(&self) -> LocalDate {
        self.min_day
    }

    fn min_time_point(&self) -> TimePoint {
        TimePoint::of_start_of_day(self.min_day)
    }

    /// The last day of the project: the end of its last task, or a later coloured or named day
    /// (`TimeBoundsData.initMinMax`).
    fn max_day(&self) -> LocalDate {
        let mut max_day = None;
        if self.model.tasks.is_empty() {
            max_day = Some(self.min_day);
        }
        for (task, data) in self.model.tasks.iter().enumerate() {
            if matches!(data.kind, TaskKind::Impl(_)) {
                let last_day = self.model.end(task).minus_one_second().to_day();
                max_day = Some(max_day.map_or(last_day, |max: LocalDate| max.max(last_day)));
            }
        }
        let mut max_day = max_day.unwrap_or(self.min_day);
        let calendar = &self.model.calendar;
        for day in calendar
            .color_days()
            .chain(calendar.name_days().keys().copied())
        {
            max_day = max_day.max(day.to_day());
        }
        max_day
    }

    /// The days the chart shows.
    fn time_bounds(&self) -> TimeBounds {
        match (self.print_start, self.print_end) {
            (Some(min_day), Some(max_day)) => TimeBounds {
                min_day,
                max_day,
                printed_interval: true,
            },
            _ => TimeBounds {
                min_day: self.min_day,
                max_day: self.max_day(),
                printed_interval: false,
            },
        }
    }

    /// Today: the day a `today is` line sets, else the day the diagram is drawn (in UTC, where PlantUML
    /// takes the JVM's time zone).
    fn today(&self) -> TimePoint {
        self.today.unwrap_or_else(|| {
            let now =
                LocalDateTime::of_epoch_second(self.source.current_time_millis().div_euclid(1000));
            TimePoint::of_start_of_day(now.to_local_date())
        })
    }

    fn today_day(&self) -> LocalDate {
        self.today().to_day()
    }

    fn set_today_colors(&mut self, colors: CenterBorderColor) {
        let today = self.today();
        self.today = Some(today);
        if let Some(color) = colors.center {
            self.model.calendar.put_color_day_today(today, color);
        }
    }

    /// The last day coloured or named, from the start on (`getThenDate`).
    fn then_date(&self) -> TimePoint {
        let calendar = &self.model.calendar;
        calendar
            .color_days()
            .chain(calendar.name_days().keys().copied())
            .fold(self.min_time_point(), TimePoint::max)
    }

    fn update_starting_point(&mut self, start: LocalDate) -> CommandResult {
        if !self.model.tasks.is_empty() {
            return Err(CommandError::new(
                "Starting point must be set before task definition",
            ));
        }
        self.min_day = start;
        Ok(())
    }

    fn existing_task(&self, id: &str) -> Option<TaskId> {
        self.model.task_by_id(id)
    }

    /// A task, or the days named `id` (`getExistingMoment`).
    fn existing_moment(&self, id: &str) -> Option<Moment> {
        if let Some(task) = self.existing_task(id) {
            return Some(Moment::Task(task));
        }
        let named: Vec<TimePoint> = self
            .model
            .calendar
            .name_days()
            .iter()
            .filter(|(_, name)| name.eq_ignore_ascii_case(id))
            .map(|(day, _)| *day)
            .collect();
        let start = named.iter().min()?;
        let end = named.iter().max()?;
        Some(Moment::Days {
            start: *start,
            end: end.increment(),
        })
    }

    /// Declares a task; one declared again keeps its place.
    fn put_task(&mut self, task: Task) -> TaskId {
        if let Some(existing) = self.existing_task(&task.code.id) {
            self.model.tasks[existing] = task;
            return existing;
        }
        self.model.tasks.push(task);
        self.model.tasks.len() - 1
    }

    /// The task with the code, created at the project start and after the previous task for `then`
    /// (`getOrCreateTask`).
    fn get_or_create_task(&mut self, code: TaskCode, linked_to_previous: bool) -> TaskId {
        if let Some(task) = self.existing_task(&code.id) {
            return task;
        }
        let previous = linked_to_previous
            .then(|| {
                self.model
                    .tasks
                    .iter()
                    .rposition(|task| matches!(task.kind, TaskKind::Impl(_)))
            })
            .flatten();
        let task = self.put_task(Task {
            code,
            row: None,
            display_string: None,
            stereotype: None,
            kind: TaskKind::Impl(Box::new(TaskImpl::new(
                self.min_time_point(),
                self.default_completion,
            ))),
        });
        if let Some(group) = self.current_group
            && let TaskKind::Group(group) = &mut self.model.tasks[group].kind
        {
            group.children.push(task);
        }
        if let Some(previous) = previous {
            self.force_task_order(previous, task)
                .expect("a new task can start after another");
        }
        task
    }

    /// `task2` starts when `task1` ends, with an arrow between them; returns the arrow (`forceTaskOrder`).
    fn force_task_order(&mut self, task1: TaskId, task2: TaskId) -> Result<usize, CommandError> {
        let end1 = TaskInstant::new(Moment::Task(task1), TaskAttribute::End);
        let start = self.model.instant_precise(&end1);
        self.set_task_solver(task2, SolverValue::Start(start))?;
        self.model.constraints.push(GanttConstraint::new(
            end1,
            TaskInstant::new(Moment::Task(task2), TaskAttribute::Start),
            None,
        ));
        Ok(self.model.constraints.len() - 1)
    }

    fn task_impl_mut(&mut self, task: TaskId) -> Result<&mut TaskImpl, CommandError> {
        self.model
            .task_impl_mut(task)
            .ok_or_else(|| CommandError::new("Only tasks can do this"))
    }

    fn set_task_solver(&mut self, task: TaskId, value: SolverValue) -> CommandResult {
        self.task_impl_mut(task)?.set_solver(value);
        Ok(())
    }

    fn set_task_colors(&mut self, task: TaskId, colors: Vec<CenterBorderColor>) -> CommandResult {
        self.task_impl_mut(task)?.set_colors(colors);
        Ok(())
    }

    fn set_diamond(&mut self, task: TaskId) -> CommandResult {
        self.task_impl_mut(task)?.diamond = true;
        Ok(())
    }

    fn pause_days(&mut self, task: TaskId, days: lang::DaysAsDates) -> CommandResult {
        let task = self.task_impl_mut(task)?;
        for day in days.days() {
            task.add_pause(day);
        }
        Ok(())
    }

    fn add_separator(&mut self, comment: Option<&str>) {
        let id = format!("##{}", self.model.tasks.len());
        self.put_task(Task {
            code: TaskCode::from_id(&id),
            row: None,
            display_string: None,
            stereotype: None,
            kind: TaskKind::Separator(TaskSeparator {
                comment: comment.map(str::to_owned),
            }),
        });
    }

    fn add_group(&mut self, code: TaskCode) {
        let group = self.put_task(Task {
            code,
            row: None,
            display_string: None,
            stereotype: None,
            kind: TaskKind::Group(TaskGroup {
                parent: self.current_group,
                children: Vec::new(),
            }),
        });
        if let Some(parent) = self.current_group
            && let TaskKind::Group(parent) = &mut self.model.tasks[parent].kind
        {
            parent.children.push(group);
        }
        self.current_group = Some(group);
    }

    fn end_group(&mut self) -> CommandResult {
        let group = self
            .current_group
            .ok_or_else(|| CommandError::new("No group to be closed"))?;
        let TaskKind::Group(group) = &self.model.tasks[group].kind else {
            unreachable!("the current group is a group")
        };
        self.current_group = group.parent;
        Ok(())
    }

    /// The note goes on the last task declared (`addNote`).
    fn add_note(&mut self, note: Display, stereotype: Option<Stereotype>) -> CommandResult {
        let last = self
            .model
            .tasks
            .len()
            .checked_sub(1)
            .ok_or_else(|| CommandError::new("No task defined"))?;
        if let Some(task) = self.model.task_impl_mut(last) {
            task.note = Some((note, stereotype));
        }
        Ok(())
    }

    /// Somebody, known from now on (`getResource`).
    fn get_resource(&mut self, name: &str) {
        if self.model.resource(name).is_none() {
            self.model
                .resources
                .push((name.to_owned(), Resource::default()));
        }
    }

    fn resource_mut(&mut self, name: &str) -> &mut Resource {
        self.get_resource(name);
        self.model
            .resources
            .iter_mut()
            .find(|(existing, _)| existing == name)
            .map(|(_, resource)| resource)
            .expect("the resource was just made known")
    }

    fn add_resource(&mut self, task: TaskId, resource: &str, percentage: i32) -> CommandResult {
        self.get_resource(resource);
        self.task_impl_mut(task)?.add_resource(resource, percentage);
        Ok(())
    }

    /// `Alice` or `Bob:50` on a task; false for 0% (`affectResource`).
    fn affect_resource(&mut self, task: TaskId, description: &str) -> bool {
        let (name, percentage) = match description.split_once(':') {
            Some((name, rest)) => {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                (name, digits.parse().unwrap_or(100))
            }
            None => (description, 100),
        };
        if percentage == 0 {
            return false;
        }
        self.add_resource(task, name, percentage).is_ok()
    }

    fn days_in_week(&self) -> i64 {
        self.model.calendar.open_close.days_in_week()
    }

    fn open_day_of_week(&mut self, day: DayOfWeek, task: &str) {
        if task.is_empty() {
            self.model.calendar.open_close.open_day_of_week(day);
        } else {
            self.model
                .calendar
                .open_close_for_task(task)
                .open_day_of_week(day);
        }
    }

    fn open_day_as_date(&mut self, day: LocalDate, task: &str) {
        if task.is_empty() {
            self.model.calendar.open_close.open(day);
        } else {
            self.model.calendar.open_close_for_task(task).open(day);
        }
    }

    fn close_day_as_date(&mut self, day: LocalDate, task: &str) {
        if task.is_empty() {
            self.model.calendar.open_close.close(day);
        } else {
            self.model.calendar.open_close_for_task(task).close(day);
        }
    }

    fn color_day(&mut self, day: LocalDate, color: Option<HColor>) {
        if let Some(color) = color {
            self.model
                .calendar
                .put_color_day(TimePoint::of_start_of_day(day), color);
        }
    }
}

impl AbstractDiagram for GanttDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}

    fn check_final_error(&mut self) -> Option<String> {
        let days = self.max_day().epoch_day() - self.min_day.epoch_day();
        (days > 365 * 20).then(|| "Gantt diagrams cannot last more than 20 years".to_owned())
    }
}

impl TitledDiagram for GanttDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for GanttDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        if let Some(not_ported) = self.titled.not_ported_part() {
            return Err(not_ported);
        }
        let main_block = draw::GanttDiagramMainBlock::new(self, string_bounder.as_ref());
        Ok(self.titled.add_chrome(Box::new(main_block), string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::none())
    }
}

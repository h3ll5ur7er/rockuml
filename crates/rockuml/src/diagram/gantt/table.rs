//! The table left of the chart, with each task's start, end and duration (PlantUML's `GanttTaskTable` and
//! `GanttTaskTableColumn`).

use super::model::TaskId;
use super::time::{TimePoint, i18n};
use super::{Column, GanttDiagram, TimeBounds};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::shape::UShape;
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::local_date::LocalDate;
use crate::style::{PName, SName, ValueReading};

const CELL_PADDING: f64 = 5.0;

pub(super) struct GanttTaskTable {
    columns: Vec<Column>,
    headers: Vec<String>,
    /// The tasks' cells and the middle of their rows.
    rows: Vec<(Vec<String>, f64)>,
    column_edges: Vec<f64>,
    header_height: f64,
    font: FontConfiguration,
    line_color: crate::color::HColor,
}

/// How the cells write instants (`GanttTaskTableColumn.Context`).
struct Context<'a> {
    language: &'a str,
    relative_mode: bool,
    min_day: LocalDate,
}

impl Context<'_> {
    fn format_day(&self, point: TimePoint) -> String {
        if self.relative_mode {
            let day = point.absolute_day_num()
                - TimePoint::of_start_of_day(self.min_day).absolute_day_num()
                + 1;
            return i18n::day_number(self.language, day);
        }
        point.to_string_short(self.language)
    }

    fn format_day_with_time(&self, point: TimePoint) -> String {
        let day = self.format_day(point);
        let time = point.to_local_date_time();
        if time.is_midnight() {
            return day;
        }
        if time.second() == 0 {
            format!("{day} {:02}:{:02}", time.hour(), time.minute())
        } else {
            format!(
                "{day} {:02}:{:02}:{:02}",
                time.hour(),
                time.minute(),
                time.second()
            )
        }
    }

    fn format_end(&self, end: TimePoint) -> String {
        if end.to_local_date_time().is_midnight() {
            self.format_day(end.minus_one_second())
        } else {
            self.format_day_with_time(end)
        }
    }
}

fn header(column: Column, language: &str) -> String {
    match column {
        Column::Task => i18n::task(language),
        Column::Start => i18n::start(language),
        Column::End => i18n::end(language),
        Column::Duration => i18n::duration(language),
    }
    .to_owned()
}

fn value_of(diagram: &GanttDiagram, column: Column, task: TaskId, context: &Context) -> String {
    let model = &diagram.model;
    match column {
        Column::Task => model.tasks[task].code.display.clone(),
        Column::Start => context.format_day_with_time(model.start(task)),
        Column::End => context.format_end(model.end(task)),
        Column::Duration => {
            let seconds = model.end(task).to_local_date_time().epoch_second()
                - model.start(task).to_local_date_time().epoch_second();
            i18n::duration_human_readable(context.language, seconds)
        }
    }
}

fn text_block(text: &str, font: &FontConfiguration) -> SheetBlock2 {
    Display::with_newlines(text).create0(
        font,
        HorizontalAlignment::Left,
        &SpriteContainerEmpty,
        0.0,
        CreoleMode::Full,
    )
}

impl GanttTaskTable {
    /// `rows` are the tasks with a row in the chart and the middle of their row.
    pub(super) fn new(
        diagram: &GanttDiagram,
        bounds: TimeBounds,
        rows: Vec<(TaskId, f64)>,
        header_height: f64,
        string_bounder: &dyn StringBounder,
    ) -> Self {
        let style = diagram.style(&[SName::Timeline]);
        let font = style.font_configuration();
        let language = diagram.language.as_str();
        let context = Context {
            language,
            relative_mode: bounds.min_day == LocalDate::EPOCH,
            min_day: bounds.min_day,
        };
        let columns: Vec<Column> = diagram.displayed_columns.iter().copied().collect();
        let headers: Vec<String> = columns
            .iter()
            .map(|column| header(*column, language))
            .collect();
        let rows: Vec<(Vec<String>, f64)> = rows
            .into_iter()
            .map(|(task, y_center)| {
                let cells = columns
                    .iter()
                    .map(|column| value_of(diagram, *column, task, &context))
                    .collect();
                (cells, y_center)
            })
            .collect();
        let width_of = |text: &str| {
            text_block(text, &font)
                .calculate_dimension(string_bounder)
                .width
        };
        let mut column_edges = vec![0.0];
        for (i, header) in headers.iter().enumerate() {
            let width = rows.iter().fold(width_of(header), |width, (cells, _)| {
                width.max(width_of(&cells[i]))
            });
            let last = *column_edges.last().expect("edges start at 0");
            column_edges.push(last + width + 2.0 * CELL_PADDING);
        }
        Self {
            columns,
            headers,
            rows,
            column_edges,
            header_height,
            font,
            line_color: style.value(PName::LineColor).as_color(),
        }
    }

    pub(super) fn width(&self) -> f64 {
        *self.column_edges.last().expect("edges start at 0")
    }

    pub(super) fn draw_u(&self, ug: &UGraphic, total_height_without_footer: f64) {
        self.draw_grid(
            &ug.with_color(self.line_color.clone()),
            total_height_without_footer,
        );
        self.draw_row(ug, self.header_height / 2.0, &self.headers);
        for (cells, y_center) in &self.rows {
            self.draw_row(ug, *y_center, cells);
        }
    }

    fn draw_grid(&self, ug: &UGraphic, total_height_without_footer: f64) {
        let width = self.width();
        let hline = UShape::Line { dx: width, dy: 0.0 };
        ug.draw(&hline);
        ug.translated(0.0, self.header_height).draw(&hline);
        ug.translated(0.0, total_height_without_footer).draw(&hline);
        for x in &self.column_edges {
            ug.translated(*x, 0.0).draw(&UShape::Line {
                dx: 0.0,
                dy: total_height_without_footer,
            });
        }
    }

    fn draw_row(&self, ug: &UGraphic, y_center: f64, cells: &[String]) {
        debug_assert_eq!(cells.len(), self.columns.len());
        for (x_left, text) in self.column_edges.iter().zip(cells) {
            let block = text_block(text, &self.font);
            let dim = block.calculate_dimension(ug.string_bounder());
            block.draw_u(&ug.translated(x_left + CELL_PADDING, y_center - dim.height / 2.0));
        }
    }
}

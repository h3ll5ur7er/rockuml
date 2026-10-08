use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::{GanttDiagram, GanttDiagramFactory};
use crate::command::factory::{Created, create_system};
use crate::diagram::UmlSource;
use crate::diagram::builder::CommandFactory;
use crate::host::Host;
use crate::local_date::LocalDate;
use crate::text::{LineLocation, StringLocated};

/// A host that only tells the time.
struct Clock(i64);

impl Host for Clock {
    fn read_file(&self, _path: &Path) -> Option<Vec<u8>> {
        None
    }

    fn read_url(&self, _url: &str) -> Option<Vec<u8>> {
        None
    }

    fn file_exists(&self, _path: &Path) -> bool {
        false
    }

    fn current_directory(&self) -> PathBuf {
        PathBuf::new()
    }

    fn home_directory(&self) -> Option<PathBuf> {
        None
    }

    fn getenv(&self, _name: &str) -> Option<String> {
        None
    }

    fn current_time_millis(&self) -> i64 {
        self.0
    }

    fn local_time_zone(&self) -> Option<String> {
        None
    }
}

fn parse(texts: &[&str], now_millis: i64) -> GanttDiagram {
    let location = LineLocation::new("test", None);
    let lines = texts
        .iter()
        .map(|text| StringLocated::new(*text, location.clone()))
        .collect();
    let mut source = UmlSource::new(lines, Vec::new());
    source.note_current_time(&Clock(now_millis));
    let source = Rc::new(source);
    match create_system(
        &source,
        || GanttDiagramFactory::create_empty_diagram(&source),
        &GanttDiagramFactory::init_commands_list(),
    ) {
        Created::Diagram(diagram) => diagram,
        _ => panic!("the lines make a gantt diagram"),
    }
}

fn date(year: i64, month: i64, day: i64) -> LocalDate {
    LocalDate::of(year, month, day).unwrap()
}

#[test]
fn today_is_the_day_the_diagram_is_drawn_unless_set() {
    let july_3 = date(2020, 7, 3).at_start_of_day().epoch_second() * 1000 + 5_000_000;
    let lines = [
        "@startgantt",
        "Project starts 2020-07-01",
        "today is colored in #AAF",
        "[Task] lasts 5 days",
        "@endgantt",
    ];
    assert_eq!(parse(&lines, july_3).today_day(), date(2020, 7, 3));
    let set = [
        "@startgantt",
        "Project starts 2020-07-01",
        "today is 2020-07-04 and is colored in #AAF",
        "[Task] lasts 5 days",
        "@endgantt",
    ];
    assert_eq!(parse(&set, july_3).today_day(), date(2020, 7, 4));
}

#[test]
fn tasks_follow_each_other_around_closed_days() {
    let gantt = parse(
        &[
            "@startgantt",
            "Project starts 2021-01-01",
            "saturday are closed",
            "sunday are closed",
            "[Design] lasts 3 days",
            "then [Build] lasts 2 days",
            "@endgantt",
        ],
        0,
    );
    let model = &gantt.model;
    let build = model.task_by_id("Build").unwrap();
    assert_eq!(model.start(build).to_day(), date(2021, 1, 6));
    assert_eq!(model.end(build).to_day(), date(2021, 1, 8));
}

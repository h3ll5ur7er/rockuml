//! The commands of gantt diagrams besides sentences (PlantUML's `gantt.command` package and
//! `GanttDiagramFactory`).

use super::lang::{NaturalGanttCommand, SubjectKind};
use super::model::{CenterBorderColor, TaskCode};
use super::time::day_of_week_from_string;
use super::{Column, GanttDiagram, PrintScale, WeeklyHeaderStrategy};
use crate::color::HColor;
use crate::command::{
    BlocLines, Command, CommandControl, CommandError, CommandResult, Multiline, PatternCommand,
    SingleLine,
};
use crate::decoration::WithLinkType;
use crate::diagram::chrome::Warning;
use crate::diagram::common_commands::{
    add_common_commands2, add_common_scale_commands, add_title_commands,
};
use crate::diagram::description::arrow_style;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stereo::{self, Stereotype};
use crate::text::LineLocation;
use crate::ubrex::builder::UBrexPart;
use crate::ubrex::{UMatcher, UnicodeBracketedExpression};

/// The commands in PlantUML's order.
pub(super) fn all() -> Vec<Box<dyn Command<GanttDiagram>>> {
    let mut commands = add_title_commands();
    commands.extend(add_common_commands2());
    commands.extend(add_common_scale_commands());
    for subject in SubjectKind::ALL {
        commands.push(Box::new(NaturalGanttCommand::new(subject)));
    }
    commands.extend([
        gantt_arrow(),
        gantt_arrow2(),
        color_task(),
        separator(),
        week_number_strategy(),
        group_start(),
        group_end(),
        language(),
        print_scale(),
        print_between(),
        note_bottom(),
        footbox(),
        label_on_column(),
        hide_resource_name(),
        hide_resource_footbox(),
        hide_show_columns(),
        hide_closed(),
        task_complete_default(),
    ]);
    commands
}

fn command(
    parts: Vec<RegexTree>,
    apply: fn(&mut GanttDiagram, &RegexResult) -> CommandResult,
) -> Box<dyn Command<GanttDiagram>> {
    let mut pattern = vec![RegexTree::start()];
    pattern.extend(parts);
    pattern.push(RegexTree::end());
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(pattern),
        move |diagram: &mut GanttDiagram, _: &LineLocation, arg: &RegexResult| apply(diagram, arg),
    )))
}

/// A one-line command matched whole by a Unicode bracketed expression (`UBrexSingleLineCommand2`).
struct UBrexCommand {
    pattern: UnicodeBracketedExpression,
    apply: fn(&mut GanttDiagram, &UMatcher) -> CommandResult,
}

impl Command<GanttDiagram> for UBrexCommand {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        match (lines.first(), lines.len()) {
            (Some(first), 1) if self.pattern.exact_match(first.trimmed().text()) => {
                CommandControl::Ok
            }
            _ => CommandControl::NotOk,
        }
    }

    fn execute(&self, diagram: &mut GanttDiagram, lines: BlocLines) -> CommandResult {
        let line = lines.first().expect("a command has its line").trimmed();
        let matcher = self
            .pattern
            .match_at(line.text())
            .expect("the line matched when the command was chosen");
        (self.apply)(diagram, &matcher)
    }
}

fn ubrex_command(
    parts: Vec<UBrexPart>,
    apply: fn(&mut GanttDiagram, &UMatcher) -> CommandResult,
) -> Box<dyn Command<GanttDiagram>> {
    let mut parts = parts;
    parts.push(UBrexPart::end());
    Box::new(UBrexCommand {
        pattern: UBrexPart::concat(parts).build(),
        apply,
    })
}

fn no_such_task(code: &str) -> CommandError {
    CommandError::new(format!("No such task {code}"))
}

/// PlantUML's `CommandGanttArrow`: `T1 -> T2`.
fn gantt_arrow() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::named(1, "CODE1", r"([%pLN_.]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::counted(1, r"(-+)"),
            RegexTree::named(1, "ARROW_STYLE", arrow_style()),
            RegexTree::counted(1, r"(-*)"),
            RegexTree::leaf(r"\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "CODE2", r"([%pLN_.]+)"),
            RegexTree::spaces_zero_or_more(),
        ],
        |diagram, arg| {
            let code1 = arg.get("CODE1", 0).unwrap_or_default();
            let code2 = arg.get("CODE2", 0).unwrap_or_default();
            let task1 = diagram
                .existing_task(code1)
                .ok_or_else(|| no_such_task(code1))?;
            let task2 = diagram
                .existing_task(code2)
                .ok_or_else(|| no_such_task(code2))?;
            let link = diagram.force_task_order(task1, task2)?;
            diagram.model.constraints[link].apply_style(arg.get("ARROW_STYLE", 0));
            Ok(())
        },
    )
}

/// PlantUML's `CommandGanttArrow2`: `[Task1] -> [Task2]`.
fn gantt_arrow2() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf(r"\["),
            RegexTree::named(1, "TASK1", r"([^\[\]]+?)"),
            RegexTree::leaf(r"\]"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::counted(1, r"(-+)"),
            RegexTree::named(1, "ARROW_STYLE", arrow_style()),
            RegexTree::counted(1, r"(-*)"),
            RegexTree::leaf(r"\>"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\["),
            RegexTree::named(1, "TASK2", r"([^\[\]]+?)"),
            RegexTree::leaf(r"\]"),
            RegexTree::spaces_zero_or_more(),
        ],
        |diagram, arg| {
            let task1 = diagram.get_or_create_task(
                TaskCode::from_id(arg.get("TASK1", 0).unwrap_or_default()),
                false,
            );
            let task2 = diagram.get_or_create_task(
                TaskCode::from_id(arg.get("TASK2", 0).unwrap_or_default()),
                false,
            );
            let link = diagram.force_task_order(task1, task2)?;
            diagram.model.constraints[link].apply_style(arg.get("ARROW_STYLE", 0));
            Ok(())
        },
    )
}

/// PlantUML's `CommandColorTask`: `[code] #Fill/Border`.
fn color_task() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::named(1, "CODE", r"\[([%pLN_.]+)\]"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(2, "COLORS", r"#(\w+)(?:/(#?\w+))?"),
            RegexTree::spaces_zero_or_more(),
        ],
        |diagram, arg| {
            let code = arg.get("CODE", 0).unwrap_or_default();
            let task = diagram
                .existing_task(code)
                .ok_or_else(|| no_such_task(code))?;
            let color = |index| {
                arg.get("COLORS", index)
                    .map(|name| {
                        HColor::parse(name)
                            .ok()
                            .flatten()
                            .ok_or_else(CommandError::bad_color)
                    })
                    .transpose()
            };
            let colors = CenterBorderColor::new(color(0)?, color(1)?);
            diagram.set_task_colors(task, vec![colors])
        },
    )
}

/// PlantUML's `CommandSeparator`: `-- comment --`.
fn separator() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("--"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(2, "COMMENT", r"((.+?)[%s]*--)?"),
        ],
        |diagram, arg| {
            diagram.add_separator(arg.get("COMMENT", 1));
            Ok(())
        },
    )
}

/// PlantUML's `CommandWeekNumberStrategy`: `weeks start on Monday and must have at least 4 days`.
fn week_number_strategy() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("weeks?"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("starts?"),
            RegexTree::leaf("[^0-9]*?"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(
                1,
                "WEEKDAY",
                "(MON[a-z]*|TUE[a-z]*|WED[a-z]*|THU[a-z]*|FRI[a-z]*|SAT[a-z]*|SUN[a-z]*)",
            ),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("[^0-9]*?"),
            RegexTree::named(1, "NUM", "([0-9]+)"),
            RegexTree::leaf("[^0-9]*?"),
        ],
        |diagram, arg| {
            let first_day = arg
                .get("WEEKDAY", 0)
                .and_then(day_of_week_from_string)
                .expect("the pattern matched a day of the week");
            let minimal_days = arg
                .get("NUM", 0)
                .and_then(|num| num.parse().ok())
                .unwrap_or_default();
            diagram.week_number_strategy = (first_day, minimal_days);
            Ok(())
        },
    )
}

/// PlantUML's `CommandGroupStart`: `group [name]`.
fn group_start() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("group"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"\["),
            RegexTree::named(1, "NAME", r"([^\[\]]+)"),
            RegexTree::leaf(r"\]"),
            RegexTree::spaces_zero_or_more(),
        ],
        |diagram, arg| {
            diagram.add_group(TaskCode::from_id(arg.get("NAME", 0).unwrap_or_default()));
            Ok(())
        },
    )
}

/// PlantUML's `CommandGroupEnd`: `end group`.
fn group_end() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("end"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("group"),
            RegexTree::spaces_zero_or_more(),
        ],
        |diagram, _| diagram.end_group(),
    )
}

/// PlantUML's `CommandLanguage`: `language de`.
fn language() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("language"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "LANG", r"(\w+)"),
        ],
        |diagram, arg| {
            diagram.language = arg.get("LANG", 0).unwrap_or_default().to_lowercase();
            Ok(())
        },
    )
}

/// PlantUML's `CommandPrintScale`: `printscale weekly zoom 2`.
fn print_scale() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::or(vec![
                RegexTree::leaf("projectscale"),
                RegexTree::leaf("ganttscale"),
                RegexTree::leaf("printscale"),
            ]),
            RegexTree::spaces_one_or_more(),
            RegexTree::named_or(
                "SCALE",
                vec![
                    RegexTree::leaf("yearly"),
                    RegexTree::leaf("quarterly"),
                    RegexTree::leaf("monthly"),
                    RegexTree::leaf("daily"),
                    RegexTree::leaf("weekly"),
                ],
            ),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::named_or(
                    "OPTION",
                    vec![
                        RegexTree::counted(1, r"(with\s+calendar\s+date)"),
                        RegexTree::named(
                            1,
                            "NUMBER",
                            r"(?:with\s+week\s+numbering\s+from\s+(-?\d+))",
                        ),
                    ],
                ),
            ])),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf("zoom"),
                RegexTree::spaces_one_or_more(),
                RegexTree::named(1, "ZOOM", r"([.\d]+)"),
            ])),
        ],
        |diagram, arg| {
            diagram.print_scale = PrintScale::from_string(arg.get("SCALE", 0).unwrap_or_default());
            if let Some(zoom) = arg.get("ZOOM", 0) {
                diagram.factor_scale = zoom.parse().map_err(|_| CommandError::new("Bad zoom"))?;
            }
            if let Some(option) = arg.get("OPTION", 0) {
                if option.contains("date") {
                    diagram.weekly_header_strategy = Some(WeeklyHeaderStrategy::DayOfMonth);
                    diagram.week_starting_number = 0;
                } else if option.contains("numbering") {
                    diagram.weekly_header_strategy = Some(WeeklyHeaderStrategy::FromN);
                    diagram.week_starting_number = arg
                        .get("NUMBER", 0)
                        .and_then(|number| number.parse().ok())
                        .unwrap_or_default();
                }
            }
            Ok(())
        },
    )
}

/// PlantUML's `CommandPrintBetween`: `print between 2020-01-01 and 2020-02-01`.
fn print_between() -> Box<dyn Command<GanttDiagram>> {
    ubrex_command(
        vec![
            UBrexPart::leaf("print"),
            UBrexPart::space_one_or_more(),
            UBrexPart::leaf("between"),
            UBrexPart::space_one_or_more(),
            UBrexPart::named("START", super::lang::any_date_ubrex()),
            UBrexPart::space_one_or_more(),
            UBrexPart::leaf("and"),
            UBrexPart::space_one_or_more(),
            UBrexPart::named("END", super::lang::any_date_ubrex()),
        ],
        |diagram, arg| {
            let start = super::lang::any_date(diagram, &arg.extract_by_prefix("START"))
                .map_err(CommandError::new)?;
            let end = super::lang::any_date(diagram, &arg.extract_by_prefix("END"))
                .map_err(CommandError::new)?;
            diagram.print_start = Some(start);
            diagram.print_end = Some(end);
            Ok(())
        },
    )
}

fn note_bottom_start() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::named(1, "TYPE", "(note)"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "POSITION", "(bottom)"),
        RegexTree::spaces_zero_or_more(),
        stereo::tags_pattern("TAGS"),
        stereo::optional_pattern("STEREO"),
        RegexTree::end(),
    ])
}

/// PlantUML's `CommandNoteBottom`: `note bottom` up to `end note`, on the last task.
fn note_bottom() -> Box<dyn Command<GanttDiagram>> {
    let first_line = note_bottom_start();
    Box::new(
        Multiline::starting_with_owned(
            note_bottom_start(),
            &plantuml_regex(r"^end[%s]*note$"),
            move |diagram: &mut GanttDiagram, lines: &BlocLines| {
                let first = lines.first().expect("a block has its first line").trimmed();
                let arg = first_line
                    .matcher(first.text())
                    .expect("the block's first line matched");
                let note = lines.sub_extract(1, 1).without_empty_columns().to_display();
                if note.lines().is_empty() {
                    return Err(CommandError::new("No note defined"));
                }
                let stereotype = arg.get("STEREO", 0).map(Stereotype::new);
                diagram.add_note(note, stereotype)
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandFootboxGantt`: `hide footbox`.
fn footbox() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::named(1, "TYPE", "(hide|show)?"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("footbox"),
        ],
        |diagram, arg| {
            diagram.display.show_footbox = arg
                .get("TYPE", 0)
                .is_some_and(|kind| kind.eq_ignore_ascii_case("show"));
            Ok(())
        },
    )
}

/// PlantUML's `CommandLabelOnColumn`, which only warns now.
fn label_on_column() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("labels?"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("on"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "POSITION", "(first|last)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("column"),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf("and"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "ALIGNED", "(left|right)"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf("aligned"),
            ])),
        ],
        |diagram, _| {
            diagram
                .titled
                .add_warning(Warning("This command is deprecated".to_owned()));
            Ok(())
        },
    )
}

/// PlantUML's `CommandHideResourceName`: `hide resources names`.
fn hide_resource_name() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("hide"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("ress?ources?"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("names?"),
        ],
        |diagram, _| {
            diagram.display.hide_resource_name = true;
            Ok(())
        },
    )
}

/// PlantUML's `CommandHideResourceFootbox`: `hide resources footbox`.
fn hide_resource_footbox() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("hide"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("ress?ources?"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("footbox"),
        ],
        |diagram, _| {
            diagram.display.hide_resource_footbox = true;
            Ok(())
        },
    )
}

/// PlantUML's `CommandHideShowColumns`: `hide column duration`.
fn hide_show_columns() -> Box<dyn Command<GanttDiagram>> {
    let columns = Column::ALL
        .iter()
        .flat_map(|column| {
            [
                UBrexPart::leaf(column.name()),
                UBrexPart::leaf(&column.name().to_lowercase()),
            ]
        })
        .collect();
    ubrex_command(
        vec![
            UBrexPart::named(
                "COMMAND",
                UBrexPart::or(vec![UBrexPart::leaf("hide"), UBrexPart::leaf("show")]),
            ),
            UBrexPart::space_one_or_more(),
            UBrexPart::leaf("column"),
            UBrexPart::space_one_or_more(),
            UBrexPart::named("WHAT", UBrexPart::or(columns)),
        ],
        |diagram, arg| {
            let show = arg
                .find_first_value_by_key("COMMAND")
                .is_some_and(|command| command.starts_with('s'));
            let what = arg.find_first_value_by_key("WHAT").unwrap_or_default();
            if let Some(column) = Column::of(what) {
                if show {
                    diagram.displayed_columns.insert(column);
                } else {
                    diagram.displayed_columns.remove(&column);
                }
            }
            Ok(())
        },
    )
}

/// PlantUML's `CommandHideClosed`: `hide closed`.
fn hide_closed() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("hide"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("closed"),
        ],
        |diagram, _| {
            diagram.hide_closed = true;
            Ok(())
        },
    )
}

/// PlantUML's `CommandTaskCompleteDefault`: `task default completion to 50`.
fn task_complete_default() -> Box<dyn Command<GanttDiagram>> {
    command(
        vec![
            RegexTree::leaf("task"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::leaf("default[%s]+completion"),
                RegexTree::leaf("completion[%s]+default"),
            ]),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("to"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "VALUE", r"(\d+)"),
            RegexTree::leaf(".*"),
        ],
        |diagram, arg| {
            let value: i32 = arg
                .get("VALUE", 0)
                .and_then(|value| value.parse().ok())
                .unwrap_or_default();
            if value > 100 {
                return Err(CommandError::new("Completetion must between 0 and 100"));
            }
            diagram.default_completion = value;
            Ok(())
        },
    )
}

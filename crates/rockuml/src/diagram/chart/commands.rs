//! The commands of charts (PlantUML's `chart.command` package).

use std::sync::LazyLock;

use regex::Regex;

use super::{
    AnnotationX, ChartAnnotation, ChartAxis, ChartDiagram, ChartSeries, GridMode, LabelPosition,
    LegendPosition, MarkerShape, Orientation, SeriesType, StackMode,
};
use crate::color::HColor;
use crate::command::{Command, CommandError, CommandResult, PatternCommand, SingleLine};
use crate::java::{parse_double, regex_split, split, trim};
use crate::pattern::{RegexResult, RegexTree};
use crate::stereo::Stereotype;
use crate::text::LineLocation;

/// The commands in PlantUML's order, ahead of the common ones.
pub(super) fn all() -> Vec<Box<dyn Command<ChartDiagram>>> {
    vec![
        h_axis(),
        v_axis(),
        series_command("bar", SeriesType::Bar),
        series_command("line", SeriesType::Line),
        series_command("area", SeriesType::Area),
        series_command("scatter", SeriesType::Scatter),
        legend(),
        stack_mode(),
        orientation(),
        annotation(),
    ]
}

/// A one-line command made of `parts`, between optional spaces at the line's start and end.
fn command(
    parts: Vec<RegexTree>,
    apply: fn(&mut ChartDiagram, &RegexResult) -> CommandResult,
) -> Box<dyn Command<ChartDiagram>> {
    let mut pattern = vec![RegexTree::start(), RegexTree::spaces_zero_or_more()];
    pattern.extend(parts);
    pattern.extend([RegexTree::spaces_zero_or_more(), RegexTree::end()]);
    Box::new(SingleLine(
        PatternCommand::new(
            RegexTree::concat(pattern),
            move |diagram: &mut ChartDiagram, _: &LineLocation, arg: &RegexResult| {
                apply(diagram, arg)
            },
        )
        .untrimmed(),
    ))
}

fn optional_named(name: &'static str, pattern: &'static str) -> RegexTree {
    RegexTree::optional(RegexTree::named(1, name, pattern))
}

/// `keyword value`, both optional together.
fn optional_keyword(keyword: &'static str, name: &'static str, value: &'static str) -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::leaf(keyword),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, name, value),
    ]))
}

/// A flag after at least one space.
fn optional_flag(name: &'static str, pattern: &'static str) -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, name, pattern),
    ]))
}

const TITLE: &str = r#""([^"]+)""#;
const RANGE: &str = r"(-?[0-9.]+)\s*-?->\s*(-?[0-9.]+)";

fn invalid_range() -> CommandError {
    CommandError::new("Invalid number format in axis range")
}

fn invalid_spacing() -> CommandError {
    CommandError::new("Invalid number format in spacing value")
}

/// PlantUML's `CommandChartHAxis`: `h-axis "title" [labels]` for categories, `h-axis "title" min --> max`
/// for numbers, with `spacing`, `label-right` and `grid`.
fn h_axis() -> Box<dyn Command<ChartDiagram>> {
    command(
        vec![
            RegexTree::leaf("[hx]-axis"),
            RegexTree::spaces_zero_or_more(),
            optional_named("TITLE", TITLE),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(2, "RANGE", RANGE)),
            RegexTree::spaces_zero_or_more(),
            optional_named("DATA", r"\[(.*)\]"),
            RegexTree::spaces_zero_or_more(),
            optional_keyword("spacing", "SPACING", "([0-9]+)"),
            RegexTree::spaces_zero_or_more(),
            optional_named("LABELRIGHT", "(label-right)"),
            RegexTree::spaces_zero_or_more(),
            optional_named("GRID", "(grid)"),
        ],
        |diagram, arg| {
            let title = arg.get("TITLE", 0);
            if let Some(spacing) = arg.get("SPACING", 0) {
                let spacing: i32 = spacing.parse().map_err(|_| invalid_spacing())?;
                if spacing <= 0 {
                    return Err(CommandError::new("X-axis spacing must be greater than 0"));
                }
                diagram.x_axis_tick_spacing = Some(spacing);
            }
            if arg.get("LABELRIGHT", 0).is_some() {
                diagram.x_axis_label_position = LabelPosition::Right;
            }
            if arg.get("GRID", 0).is_some() {
                diagram.x_grid_mode = GridMode::Major;
            }
            if let (Some(min), Some(max)) = (arg.get("RANGE", 0), arg.get("RANGE", 1)) {
                let min = parse_double(min).ok_or_else(invalid_range)?;
                let max = parse_double(max).ok_or_else(invalid_range)?;
                diagram.x_axis.set(title, Some(min), Some(max));
                return Ok(());
            }
            diagram.x_axis_title = title.map(str::to_owned);
            diagram.x_axis_labels = parse_labels(arg.get("DATA", 0));
            Ok(())
        },
    )
}

/// PlantUML's `CommandChartVAxis`: `v-axis` and `v2-axis` with a title, a range, `ticks [value:"label"]`,
/// `spacing`, `label-top` and `grid`; `v-axis [labels]` gives horizontal bars their categories.
fn v_axis() -> Box<dyn Command<ChartDiagram>> {
    command(
        vec![
            RegexTree::named(1, "AXIS", "([vy]2?-axis)"),
            RegexTree::spaces_zero_or_more(),
            optional_named("TITLE", TITLE),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(2, "RANGE", RANGE)),
            RegexTree::spaces_zero_or_more(),
            optional_named("LABELS", r"\[([^\]]+)\]"),
            RegexTree::spaces_zero_or_more(),
            optional_keyword("ticks", "TICKS", r"\[(.*)\]"),
            RegexTree::spaces_zero_or_more(),
            optional_keyword("spacing", "SPACING", "([0-9.]+)"),
            RegexTree::spaces_zero_or_more(),
            optional_named("LABELTOP", "(label-top)"),
            RegexTree::spaces_zero_or_more(),
            optional_named("GRID", "(grid)"),
        ],
        |diagram, arg| {
            if let Some(labels) = arg.get("LABELS", 0) {
                diagram.y_axis_labels = parse_labels(Some(labels));
                return Ok(());
            }
            let range = match (arg.get("RANGE", 0), arg.get("RANGE", 1)) {
                (Some(min), Some(max)) => Some((
                    parse_double(min).ok_or_else(invalid_range)?,
                    parse_double(max).ok_or_else(invalid_range)?,
                )),
                _ => None,
            };
            let custom_ticks = arg
                .get("TICKS", 0)
                .map(|ticks| {
                    parse_custom_ticks(ticks).ok_or_else(|| {
                        CommandError::new("Invalid tick format. Expected: [value:\"label\", ...]")
                    })
                })
                .transpose()?;
            let tick_spacing = arg
                .get("SPACING", 0)
                .map(|spacing| {
                    let spacing = parse_double(spacing).ok_or_else(invalid_spacing)?;
                    if spacing <= 0.0 {
                        return Err(CommandError::new("Tick spacing must be greater than 0"));
                    }
                    Ok(spacing)
                })
                .transpose()?;
            let title = arg.get("TITLE", 0);
            let (min, max) = range.unzip();
            let secondary = arg
                .get("AXIS", 0)
                .is_some_and(|axis| axis.starts_with("v2") || axis.starts_with("y2"));
            let axis: &mut ChartAxis = if secondary {
                diagram.set_y2_axis(title, min, max);
                diagram.y2_axis.as_mut().expect("just set")
            } else {
                diagram.y_axis.set(title, min, max);
                &mut diagram.y_axis
            };
            if custom_ticks.is_some() {
                axis.custom_ticks = custom_ticks;
            }
            if tick_spacing.is_some() {
                axis.tick_spacing = tick_spacing;
            }
            if arg.get("LABELTOP", 0).is_some() {
                axis.label_position = LabelPosition::Top;
            }
            if arg.get("GRID", 0).is_some() {
                diagram.y_grid_mode = GridMode::Major;
            }
            Ok(())
        },
    )
}

/// Comma-separated labels, each trimmed and taken out of its quotes.
fn parse_labels(data: Option<&str>) -> Vec<String> {
    let Some(data) = data.filter(|data| !trim(data).is_empty()) else {
        return Vec::new();
    };
    split(data, ",")
        .iter()
        .map(|part| {
            let label = trim(part);
            label
                .strip_prefix('"')
                .and_then(|label| label.strip_suffix('"'))
                .unwrap_or(label)
                .to_owned()
        })
        .collect()
}

/// `0:"Low", 50:"Mid"`: a value and a quoted label per tick, a later label replacing an earlier one of the
/// same value in its place.
fn parse_custom_ticks(ticks: &str) -> Option<Vec<(f64, String)>> {
    let mut result: Vec<(f64, String)> = Vec::new();
    if trim(ticks).is_empty() {
        return Some(result);
    }
    for pair in split(ticks, ",") {
        let pair = trim(&pair);
        let (value, label) = pair.split_once(':')?;
        let label = trim(label);
        let label = label
            .strip_prefix('"')
            .and_then(|label| label.strip_suffix('"'))?;
        let value = parse_double(value)?;
        match result
            .iter_mut()
            .find(|(known, _)| known.to_bits() == value.to_bits())
        {
            Some(tick) => label.clone_into(&mut tick.1),
            None => result.push((value, label.to_owned())),
        }
    }
    Some(result)
}

/// PlantUML's `CommandChartBar`, `CommandChartLine`, `CommandChartArea` and `CommandChartScatter`:
/// `bar <<stereo>> "name" [values] #color labels`. Lines, areas and scatters may use the secondary axis
/// (`v2`), lines and scatters coordinate pairs (`[(x,y), ...]`), and scatters a marker (`<<square>>`).
fn series_command(
    keyword: &'static str,
    series_type: SeriesType,
) -> Box<dyn Command<ChartDiagram>> {
    let mut parts = vec![
        RegexTree::leaf(keyword),
        RegexTree::spaces_zero_or_more(),
        optional_named("STEREO", r"(\<\<.+?\>\>)"),
        RegexTree::spaces_zero_or_more(),
        optional_named("NAME", r#""([^"]+)""#),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "DATA", r"\[(.*)\]"),
        RegexTree::spaces_zero_or_more(),
        optional_named("COLOR", r"#([0-9a-fA-F]{6}|[0-9a-fA-F]{3}|\w+)"),
    ];
    if series_type != SeriesType::Bar {
        parts.push(optional_flag("V2", "([vy]2)"));
    }
    parts.push(optional_flag("LABELS", "(labels)"));
    if series_type == SeriesType::Scatter {
        parts.push(optional_flag("MARKER", "<<(circle|square|triangle)>>"));
    }
    let apply: fn(&mut ChartDiagram, &RegexResult) -> CommandResult = match series_type {
        SeriesType::Bar => |diagram, arg| add_series(diagram, arg, "bar", SeriesType::Bar),
        SeriesType::Line => |diagram, arg| add_series(diagram, arg, "line", SeriesType::Line),
        SeriesType::Area => |diagram, arg| add_series(diagram, arg, "area", SeriesType::Area),
        SeriesType::Scatter => {
            |diagram, arg| add_series(diagram, arg, "scatter", SeriesType::Scatter)
        }
    };
    command(parts, apply)
}

fn add_series(
    diagram: &mut ChartDiagram,
    arg: &RegexResult,
    keyword: &str,
    series_type: SeriesType,
) -> CommandResult {
    let data = arg.get("DATA", 0).unwrap_or_default();
    let name = arg.get("NAME", 0).map_or_else(
        || format!("{keyword}{}", diagram.series.len()),
        str::to_owned,
    );
    let takes_pairs = matches!(series_type, SeriesType::Line | SeriesType::Scatter);
    let mut series = if takes_pairs && data.contains('(') {
        let (x_values, y_values) = parse_coordinate_pairs(data).ok_or_else(|| {
            CommandError::new(format!("Invalid coordinate pair format in {keyword} data"))
        })?;
        let mut series = ChartSeries::new(name, series_type, y_values);
        series.x_values = Some(x_values);
        series
    } else {
        let values = parse_values(data)
            .ok_or_else(|| CommandError::new(format!("Invalid number format in {keyword} data")))?;
        ChartSeries::new(name, series_type, values)
    };
    series.stereotype = arg.get("STEREO", 0).map(Stereotype::new);
    if let Some(color) = arg.get("COLOR", 0) {
        series.color = Some(
            HColor::parse(&format!("#{color}"))
                .ok()
                .flatten()
                .ok_or_else(CommandError::bad_color)?,
        );
    }
    if let Some(marker) = arg.get("MARKER", 0) {
        series.marker_shape = MarkerShape::named(marker).unwrap_or(MarkerShape::Circle);
    }
    series.use_secondary_axis = arg.get("V2", 0).is_some();
    series.show_labels = arg.get("LABELS", 0).is_some();
    diagram.add_series(series)
}

fn parse_values(data: &str) -> Option<Vec<f64>> {
    if trim(data).is_empty() {
        return Some(Vec::new());
    }
    split(data, ",")
        .iter()
        .map(|part| parse_double(part))
        .collect()
}

/// `(x,y), (x,y)`: exactly two numbers in each pair.
fn parse_coordinate_pairs(data: &str) -> Option<(Vec<f64>, Vec<f64>)> {
    static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\n\x0B\f\r]+").unwrap());
    static LEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\[(]+").unwrap());
    static TRAILING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\])]+$").unwrap());
    static PAIR_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\),\(").unwrap());
    if trim(data).is_empty() {
        return None;
    }
    let cleaned = WHITESPACE.replace_all(data, "");
    let mut x_values = Vec::new();
    let mut y_values = Vec::new();
    for pair in regex_split(&PAIR_SEPARATOR, &cleaned) {
        let pair = LEADING.replace(trim(&pair), "").into_owned();
        let pair = TRAILING.replace(&pair, "").into_owned();
        let [x, y] = split(&pair, ",").try_into().ok()?;
        x_values.push(parse_double(&x)?);
        y_values.push(parse_double(&y)?);
    }
    Some((x_values, y_values))
}

/// PlantUML's `CommandChartLegend`: `legend left`, `right`, `top` or `bottom` places the series' legend.
fn legend() -> Box<dyn Command<ChartDiagram>> {
    command(
        vec![
            RegexTree::leaf("legend"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "POSITION", "(left|right|top|bottom)"),
        ],
        |diagram, arg| {
            let position = arg.get("POSITION", 0).unwrap_or_default();
            diagram.legend_position = match position.to_lowercase().as_str() {
                "left" => LegendPosition::Left,
                "right" => LegendPosition::Right,
                "top" => LegendPosition::Top,
                _ => LegendPosition::Bottom,
            };
            Ok(())
        },
    )
}

/// PlantUML's `CommandChartStackMode`: `stackMode stacked` piles bars up, `grouped` sets them side by side.
fn stack_mode() -> Box<dyn Command<ChartDiagram>> {
    command(
        vec![
            RegexTree::leaf("stackMode"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "MODE", "(grouped|stacked)"),
        ],
        |diagram, arg| {
            let mode = arg.get("MODE", 0).unwrap_or_default();
            diagram.stack_mode = if mode.eq_ignore_ascii_case("stacked") {
                StackMode::Stacked
            } else {
                StackMode::Grouped
            };
            Ok(())
        },
    )
}

/// PlantUML's `CommandChartOrientation`: `orientation horizontal` draws bars across.
fn orientation() -> Box<dyn Command<ChartDiagram>> {
    command(
        vec![
            RegexTree::leaf("orientation"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "ORIENTATION", "(vertical|horizontal)"),
        ],
        |diagram, arg| {
            let orientation = arg.get("ORIENTATION", 0).unwrap_or_default();
            diagram.orientation = if orientation.eq_ignore_ascii_case("horizontal") {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            };
            Ok(())
        },
    )
}

/// PlantUML's `CommandChartAnnotation`: `annotation "text" at (x, y) <<arrow>>`, `x` a category or a number.
fn annotation() -> Box<dyn Command<ChartDiagram>> {
    command(
        vec![
            RegexTree::leaf("annotation"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "TEXT", r#""([^"]+)""#),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("at"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\("),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "XPOS", "([^,]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(","),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "YPOS", r"([^)]+)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\)"),
            RegexTree::spaces_zero_or_more(),
            optional_named("ARROW", "<<(arrow)>>"),
        ],
        |diagram, arg| {
            let x = trim(arg.get("XPOS", 0).unwrap_or_default());
            let y = trim(arg.get("YPOS", 0).unwrap_or_default());
            let x_position = parse_double(x)
                .map_or_else(|| AnnotationX::Label(x.to_owned()), AnnotationX::Number);
            let y_position = parse_double(y).ok_or_else(|| {
                CommandError::new(format!("Y position must be a numeric value: {y}"))
            })?;
            diagram.annotations.push(ChartAnnotation {
                text: arg.get("TEXT", 0).unwrap_or_default().to_owned(),
                x_position,
                y_position,
                show_arrow: arg.get("ARROW", 0).is_some(),
            });
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_lose_their_quotes_and_trailing_empty_parts() {
        assert_eq!(parse_labels(Some(r#" "Jan", Feb ,,"#)), ["Jan", "Feb"]);
        assert_eq!(parse_labels(Some("  ")), Vec::<String>::new());
    }

    #[test]
    fn ticks_need_a_value_and_a_quoted_label() {
        assert_eq!(
            parse_custom_ticks(r#"0:"Low", 5:"Mid", 0:"Zero""#),
            Some(vec![(0.0, "Zero".to_owned()), (5.0, "Mid".to_owned())])
        );
        assert_eq!(parse_custom_ticks("0:Low"), None);
        assert_eq!(parse_custom_ticks(r#"x:"Low""#), None);
        assert_eq!(parse_custom_ticks("5"), None);
    }

    #[test]
    fn coordinate_pairs_hold_two_numbers() {
        assert_eq!(
            parse_coordinate_pairs("(-1, 2), ( 3,4.5 )"),
            Some((vec![-1.0, 3.0], vec![2.0, 4.5]))
        );
        assert_eq!(parse_coordinate_pairs("(1,2,3)"), None);
        assert_eq!(parse_coordinate_pairs("(1,a)"), None);
    }
}

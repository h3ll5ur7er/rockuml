//! Charts, `@startchart`: bar, line, area and scatter series over category or numeric axes, with legends,
//! grids and annotations (PlantUML's `chart` package).

mod bar;
mod commands;
mod plots;
mod renderer;
mod text;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::HColor;
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult};
use crate::java::double_to_string;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::stereo::Stereotype;
use crate::style::SName;
use renderer::ChartRenderer;

/// Reads charts (PlantUML's `ChartDiagramFactory`).
pub(super) struct ChartDiagramFactory;

impl CommandFactory for ChartDiagramFactory {
    type Diagram = ChartDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Chart;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ChartDiagram {
        ChartDiagram {
            source: source.clone(),
            titled: Titled::new(SName::ChartDiagram, "CHART", source),
            x_axis_labels: Vec::new(),
            x_axis_title: None,
            x_axis_tick_spacing: None,
            x_axis_label_position: LabelPosition::Default,
            x_axis: ChartAxis::default(),
            y_axis_labels: Vec::new(),
            series: Vec::new(),
            y_axis: ChartAxis::default(),
            y2_axis: None,
            legend_position: LegendPosition::None,
            x_grid_mode: GridMode::Off,
            y_grid_mode: GridMode::Off,
            stack_mode: StackMode::Grouped,
            orientation: Orientation::Vertical,
            annotations: Vec::new(),
        }
    }

    /// The chart's commands come first, so that `legend right` is not taken for a legend block.
    fn init_commands_list() -> Vec<Box<dyn Command<ChartDiagram>>> {
        let mut commands = commands::all();
        commands.extend(add_common_commands1());
        commands
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LegendPosition {
    None,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GridMode {
    Off,
    Major,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StackMode {
    Grouped,
    Stacked,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Orientation {
    Vertical,
    Horizontal,
}

/// Where an axis title goes (`ChartAxis.LabelPosition`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelPosition {
    /// Upright beside a vertical axis, centred below a horizontal one.
    Default,
    /// Level above a vertical axis.
    Top,
    /// At the right end of a horizontal axis.
    Right,
}

/// A numeric axis: its range grows to take in the values unless it was given (`ChartAxis`).
#[derive(Clone)]
struct ChartAxis {
    title: String,
    min: f64,
    max: f64,
    auto_scale: bool,
    /// Labels by value, in the order written.
    custom_ticks: Option<Vec<(f64, String)>>,
    tick_spacing: Option<f64>,
    label_position: LabelPosition,
}

impl Default for ChartAxis {
    fn default() -> Self {
        Self {
            title: String::new(),
            min: 0.0,
            max: 100.0,
            auto_scale: true,
            custom_ticks: None,
            tick_spacing: None,
            label_position: LabelPosition::Default,
        }
    }
}

impl ChartAxis {
    fn set_min(&mut self, min: f64) {
        self.min = min;
        self.auto_scale = false;
    }

    fn set_max(&mut self, max: f64) {
        self.max = max;
        self.auto_scale = false;
    }

    fn value_to_pixel(&self, value: f64, pixel_min: f64, pixel_max: f64) -> f64 {
        if self.max == self.min {
            return pixel_min;
        }
        pixel_min + (value - self.min) / (self.max - self.min) * (pixel_max - pixel_min)
    }

    fn include_value(&mut self, value: f64) {
        if self.auto_scale {
            if value < self.min {
                self.min = value;
            }
            if value > self.max {
                self.max = value;
            }
        }
    }

    fn has_custom_ticks(&self) -> bool {
        self.custom_ticks
            .as_ref()
            .is_some_and(|ticks| !ticks.is_empty())
    }

    fn has_tick_spacing(&self) -> bool {
        self.tick_spacing.is_some_and(|spacing| spacing > 0.0)
    }

    fn set(&mut self, title: Option<&str>, min: Option<f64>, max: Option<f64>) {
        if let Some(title) = title {
            title.clone_into(&mut self.title);
        }
        if let Some(min) = min {
            self.set_min(min);
        }
        if let Some(max) = max {
            self.set_max(max);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SeriesType {
    Bar,
    Line,
    Area,
    Scatter,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MarkerShape {
    Circle,
    Square,
    Triangle,
}

impl MarkerShape {
    fn named(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "circle" => Some(Self::Circle),
            "square" => Some(Self::Square),
            "triangle" => Some(Self::Triangle),
            _ => None,
        }
    }
}

/// One series of values, by category or at given x coordinates (`ChartSeries`).
struct ChartSeries {
    name: String,
    series_type: SeriesType,
    values: Vec<f64>,
    /// Only for coordinate pairs.
    x_values: Option<Vec<f64>>,
    color: Option<HColor>,
    use_secondary_axis: bool,
    show_labels: bool,
    marker_shape: MarkerShape,
    stereotype: Option<Stereotype>,
}

impl ChartSeries {
    fn new(name: String, series_type: SeriesType, values: Vec<f64>) -> Self {
        Self {
            name,
            series_type,
            values,
            x_values: None,
            color: None,
            use_secondary_axis: false,
            show_labels: false,
            marker_shape: MarkerShape::Circle,
            stereotype: None,
        }
    }

    fn has_explicit_x_values(&self) -> bool {
        self.x_values.is_some()
    }
}

/// Where an annotation points across: a category, or a number on the value axis.
enum AnnotationX {
    Number(f64),
    Label(String),
}

/// A text at a point of the chart, with an arrow to it or not (`ChartAnnotation`).
struct ChartAnnotation {
    text: String,
    x_position: AnnotationX,
    y_position: f64,
    show_arrow: bool,
}

/// A `@startchart` diagram (PlantUML's `ChartDiagram`).
pub(super) struct ChartDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    x_axis_labels: Vec<String>,
    x_axis_title: Option<String>,
    x_axis_tick_spacing: Option<i32>,
    x_axis_label_position: LabelPosition,
    /// The numeric horizontal axis, of coordinate pairs and horizontal bars.
    x_axis: ChartAxis,
    y_axis_labels: Vec<String>,
    series: Vec<ChartSeries>,
    y_axis: ChartAxis,
    y2_axis: Option<ChartAxis>,
    legend_position: LegendPosition,
    x_grid_mode: GridMode,
    y_grid_mode: GridMode,
    stack_mode: StackMode,
    orientation: Orientation,
    annotations: Vec<ChartAnnotation>,
}

impl ChartDiagram {
    /// Horizontal bars take their categories from the vertical axis and their values from the horizontal one.
    fn get_renderer(&self) -> ChartRenderer<'_> {
        if self.orientation == Orientation::Horizontal {
            return ChartRenderer {
                skin: &self.titled.skin,
                x_axis_labels: &self.y_axis_labels,
                x_axis_title: Some(&self.y_axis.title),
                x_axis_tick_spacing: None,
                x_axis_label_position: self.x_axis_label_position,
                series: &self.series,
                x_axis: &self.x_axis,
                y_axis: &self.x_axis,
                y2_axis: None,
                legend_position: self.legend_position,
                x_grid_mode: self.y_grid_mode,
                y_grid_mode: self.x_grid_mode,
                stack_mode: self.stack_mode,
                orientation: self.orientation,
                annotations: &self.annotations,
            };
        }
        let h_axis_title = if self.x_axis.title.is_empty() {
            self.x_axis_title.as_deref()
        } else {
            Some(self.x_axis.title.as_str())
        };
        ChartRenderer {
            skin: &self.titled.skin,
            x_axis_labels: &self.x_axis_labels,
            x_axis_title: h_axis_title,
            x_axis_tick_spacing: self.x_axis_tick_spacing,
            x_axis_label_position: self.x_axis_label_position,
            series: &self.series,
            x_axis: &self.x_axis,
            y_axis: &self.y_axis,
            y2_axis: self.y2_axis.as_ref(),
            legend_position: self.legend_position,
            x_grid_mode: self.x_grid_mode,
            y_grid_mode: self.y_grid_mode,
            stack_mode: self.stack_mode,
            orientation: self.orientation,
            annotations: &self.annotations,
        }
    }

    fn set_y2_axis(&mut self, title: Option<&str>, min: Option<f64>, max: Option<f64>) {
        self.y2_axis
            .get_or_insert_with(ChartAxis::default)
            .set(title, min, max);
    }

    /// Coordinate pairs need a numeric horizontal axis with a range that holds them, and series cannot mix
    /// pairs with values by category.
    fn add_series(&mut self, series: ChartSeries) -> CommandResult {
        if let Some(x_values) = &series.x_values {
            if !self.x_axis_labels.is_empty() {
                return Err(CommandError::new(
                    "Coordinate pair notation requires numeric h-axis (e.g., h-axis \"x\" -5 --> 5), not categorical labels",
                ));
            }
            if self.x_axis.auto_scale || self.x_axis.max == self.x_axis.min {
                return Err(CommandError::new(
                    "Coordinate pair notation requires explicit h-axis range (e.g., h-axis \"x\" -5 --> 5)",
                ));
            }
            self.check_same_format(&series)?;
            if let Some(&x) = x_values
                .iter()
                .find(|&&x| x < self.x_axis.min || x > self.x_axis.max)
            {
                return Err(CommandError::new(format!(
                    "X-coordinate {} is outside h-axis range [{}, {}]",
                    double_to_string(x),
                    double_to_string(self.x_axis.min),
                    double_to_string(self.x_axis.max)
                )));
            }
            for &x in x_values {
                self.x_axis.include_value(x);
            }
        } else {
            self.check_same_format(&series)?;
        }
        let axis = match &mut self.y2_axis {
            Some(y2_axis) if series.use_secondary_axis => y2_axis,
            _ => &mut self.y_axis,
        };
        for &value in &series.values {
            axis.include_value(value);
        }
        self.series.push(series);
        Ok(())
    }

    fn check_same_format(&self, series: &ChartSeries) -> CommandResult {
        match self.series.first() {
            Some(first) if first.has_explicit_x_values() != series.has_explicit_x_values() => {
                Err(CommandError::new(
                    "All series must use the same data format (either all coordinate pairs or all index-based)",
                ))
            }
            _ => Ok(()),
        }
    }
}

impl AbstractDiagram for ChartDiagram {}

impl TitledDiagram for ChartDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for ChartDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Ok(self
            .titled
            .add_chrome(Box::new(self.get_renderer()), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

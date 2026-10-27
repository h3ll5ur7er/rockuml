//! The chart drawn: axes with their ticks and titles, grids, series, annotations and the legend (PlantUML's
//! `ChartRenderer`).

use super::bar::BarRenderer;
use super::plots::{Plot, draw_marker};
use super::text::{font_configuration, format_value, text_block};
use super::{
    AnnotationX, ChartAnnotation, ChartAxis, ChartSeries, GridMode, LabelPosition, LegendPosition,
    MarkerShape, Orientation, SeriesType, StackMode,
};
use crate::color::HColor;
use crate::creole::SheetBlock2;
use crate::java::format_fixed;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape, UText};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

const MARGIN: f64 = 20.0;
const AXIS_LABEL_SPACE: f64 = 40.0;
const TITLE_SPACE: f64 = 30.0;
const TICK_SIZE: f64 = 5.0;
const LEGEND_MARGIN: f64 = 10.0;
const LEGEND_SYMBOL_SIZE: f64 = 12.0;
const LEGEND_TEXT_SPACING: f64 = 5.0;
const LEGEND_ITEM_SPACING: f64 = 15.0;
const PLOT_HEIGHT: f64 = 300.0;
/// The intervals between the ticks of a value axis without spacing or ticks of its own.
const NUM_TICKS: i32 = 5;

/// What a chart draws with its axes as the renderer sees them: categories along the x axis, values up
/// the y axis, whichever way the bars go.
pub(super) struct ChartRenderer<'a> {
    pub skin: &'a SkinParam,
    pub x_axis_labels: &'a [String],
    pub x_axis_title: Option<&'a str>,
    pub x_axis_tick_spacing: Option<i32>,
    pub x_axis_label_position: LabelPosition,
    pub series: &'a [ChartSeries],
    pub x_axis: &'a ChartAxis,
    pub y_axis: &'a ChartAxis,
    pub y2_axis: Option<&'a ChartAxis>,
    pub legend_position: LegendPosition,
    pub x_grid_mode: GridMode,
    pub y_grid_mode: GridMode,
    pub stack_mode: StackMode,
    pub orientation: Orientation,
    pub annotations: &'a [ChartAnnotation],
}

/// The ticks from the multiple of `interval` at `start` to the axis's end, a hundredth of an interval
/// allowed beyond either end for rounding.
fn ticks(axis: &ChartAxis, start: f64, interval: f64) -> Vec<f64> {
    let mut ticks = Vec::new();
    let mut value = start;
    while value <= axis.max + interval * 0.01 {
        if value >= axis.min - interval * 0.01 {
            ticks.push(value);
        }
        value += interval;
    }
    ticks
}

impl ChartRenderer<'_> {
    fn style_of(&self, names: &[SName]) -> Style {
        StyleSignature::of(names).get_merged_style(&self.skin.current_style_builder())
    }

    fn chart_style(&self, name: SName) -> Style {
        self.style_of(&[SName::Root, SName::Element, SName::ChartDiagram, name])
    }

    /// A series' style: its stereotype's rules, then those of the stereotype itself over them.
    fn series_style(&self, name: SName, series: &ChartSeries) -> Style {
        let signature =
            StyleSignature::of(&[SName::Root, SName::Element, SName::ChartDiagram, name]);
        let builder = self.skin.current_style_builder();
        match &series.stereotype {
            Some(stereotype) => signature
                .get_merged_style_with(&builder, Some(stereotype))
                .merge_with(
                    &signature.get_merged_style_for_stereotype_itself(&builder, Some(stereotype)),
                ),
            None => signature.get_merged_style(&builder),
        }
    }

    fn axis_style(&self, horizontal: bool) -> Style {
        let axis_type = if horizontal {
            SName::HAxis
        } else {
            SName::VAxis
        };
        self.style_of(&[
            SName::Root,
            SName::Element,
            SName::ChartDiagram,
            SName::Axis,
            axis_type,
        ])
    }

    /// The grid's colour and stroke.
    fn grid(&self) -> (HColor, UStroke) {
        let style = self.chart_style(SName::Grid);
        (
            style.value(PName::LineColor).as_color(),
            UStroke::with_thickness(style.value(PName::LineThickness).as_double()),
        )
    }

    fn plot_width(&self) -> f64 {
        (self.x_axis_labels.len() as f64 * 60.0).max(400.0)
    }

    fn is_horizontal(&self) -> bool {
        self.orientation == Orientation::Horizontal
    }

    fn x_axis_title(&self) -> Option<&str> {
        self.x_axis_title.filter(|title| !title.is_empty())
    }

    fn block(
        &self,
        text: &str,
        font: &FontConfiguration,
        alignment: HorizontalAlignment,
    ) -> SheetBlock2 {
        text_block(self.skin, text, font, alignment)
    }

    /// Where zero is on a vertical extent of `height`, from its top, when the axis's range holds it.
    fn zero_ratio(&self) -> Option<f64> {
        (self.y_axis.min <= 0.0 && self.y_axis.max >= 0.0)
            .then(|| (0.0 - self.y_axis.min) / (self.y_axis.max - self.y_axis.min))
    }

    fn draw_y_axis(&self, ug: &UGraphic, height: f64, axis: &ChartAxis, left_side: bool) {
        let style = self.axis_style(false);
        let font = style.font_configuration();
        let ug = ug
            .with_color(style.value(PName::LineColor).as_color())
            .with_stroke(UStroke::with_thickness(
                style.value(PName::LineThickness).as_double(),
            ));
        ug.draw(&UShape::Line {
            dx: 0.0,
            dy: height,
        });
        // PlantUML's horizontal grid lines come from `drawGridLines`: the ones here are for charts without x
        // axis, and every chart has one.
        let tick = |y: f64, label: &str| {
            let dx = if left_side { -TICK_SIZE } else { TICK_SIZE };
            ug.translated(0.0, y).draw(&UShape::Line { dx, dy: 0.0 });
            let block = self.block(label, &font, HorizontalAlignment::Right);
            let dimension = block.calculate_dimension(ug.string_bounder());
            let x = if left_side {
                -TICK_SIZE - dimension.width - 5.0
            } else {
                TICK_SIZE + 5.0
            };
            block.draw_u(&ug.translated(x, y - dimension.height / 2.0));
        };
        if let Some(custom_ticks) = axis
            .custom_ticks
            .as_ref()
            .filter(|_| axis.has_custom_ticks())
        {
            for (value, label) in custom_ticks {
                let y = height * (1.0 - (value - axis.min) / (axis.max - axis.min));
                if y < 0.0 || y > height {
                    continue;
                }
                tick(y, label);
            }
        } else if let Some(spacing) = axis.tick_spacing.filter(|_| axis.has_tick_spacing()) {
            let range = axis.max - axis.min;
            let mut value = (axis.min / spacing).ceil() * spacing;
            while value <= axis.max {
                let y = height * (1.0 - (value - axis.min) / range);
                tick(y, &format_value(value, 2));
                value += spacing;
            }
        } else {
            for i in 0..=NUM_TICKS {
                let y = height * (1.0 - f64::from(i) / f64::from(NUM_TICKS));
                let value = axis.min + (axis.max - axis.min) * f64::from(i) / f64::from(NUM_TICKS);
                tick(y, &format_value(value, 2));
            }
        }
        if !axis.title.is_empty() {
            if axis.label_position == LabelPosition::Top {
                self.draw_horizontal_axis_title(&ug, &axis.title, left_side, &font);
            } else {
                Self::draw_vertical_text(&ug, &axis.title, height, left_side, &font);
            }
        }
    }

    /// Reading upwards beside the left axis, downwards beside the right one, centred along it.
    fn draw_vertical_text(
        ug: &UGraphic,
        text: &str,
        height: f64,
        left_side: bool,
        font: &FontConfiguration,
    ) {
        const EXTRA_SPACING: f64 = 10.0;
        let orientation = if left_side { 90 } else { 270 };
        let utext = UText::new(text, font.clone()).with_orientation(orientation);
        let dimension = ug.string_bounder().calculate_dimension(&font.font(), text);
        let (x, y) = if left_side {
            (
                -AXIS_LABEL_SPACE + dimension.height / 2.0 - EXTRA_SPACING,
                height / 2.0 + dimension.width / 2.0,
            )
        } else {
            (
                AXIS_LABEL_SPACE - dimension.height / 2.0 + EXTRA_SPACING,
                height / 2.0 - dimension.width / 2.0,
            )
        };
        ug.translated(x, y).draw(&UShape::Text(utext));
    }

    /// Level above a vertical axis.
    fn draw_horizontal_axis_title(
        &self,
        ug: &UGraphic,
        text: &str,
        left_side: bool,
        font: &FontConfiguration,
    ) {
        let block = self.block(text, font, HorizontalAlignment::Center);
        let dimension = block.calculate_dimension(ug.string_bounder());
        let x = if left_side {
            -AXIS_LABEL_SPACE / 2.0 - dimension.width / 2.0
        } else {
            AXIS_LABEL_SPACE / 2.0 - dimension.width / 2.0
        };
        block.draw_u(&ug.translated(x, -dimension.height - 15.0));
    }

    fn draw_x_axis(&self, ug: &UGraphic, width: f64, height: f64) {
        let style = self.axis_style(true);
        let font = style.font_configuration();
        let ug = ug
            .with_color(style.value(PName::LineColor).as_color())
            .with_stroke(UStroke::with_thickness(
                style.value(PName::LineThickness).as_double(),
            ));
        ug.draw(&UShape::Line { dx: width, dy: 0.0 });
        let label_below = |x: f64, label: &str| {
            ug.translated(x, 0.0).draw(&UShape::Line {
                dx: 0.0,
                dy: TICK_SIZE,
            });
            let block = self.block(label, &font, HorizontalAlignment::Center);
            let label_width = block.calculate_dimension(ug.string_bounder()).width;
            block.draw_u(&ug.translated(x - label_width / 2.0, TICK_SIZE + 5.0));
        };
        if self.x_axis_labels.is_empty() {
            let interval = self.x_tick_interval();
            let start = (self.x_axis.min / interval).floor() * interval;
            for value in ticks(self.x_axis, start, interval) {
                let x = self.x_axis.value_to_pixel(value, 0.0, width);
                label_below(x, &format_value(value, 1));
            }
        } else {
            let category_width = width / self.x_axis_labels.len() as f64;
            let spacing = self
                .x_axis_tick_spacing
                .filter(|&spacing| spacing > 0)
                .unwrap_or(1);
            if self.x_grid_mode != GridMode::Off {
                let (grid_color, grid_stroke) = self.grid();
                let (to_top, to_bottom) = match self.zero_ratio() {
                    Some(zero_ratio) => (height * (1.0 - zero_ratio), height * zero_ratio),
                    None => (height, 0.0),
                };
                let grid = ug.with_color(grid_color).with_stroke(grid_stroke);
                for i in 0..self.x_axis_labels.len() {
                    let x = (i as f64 + 0.5) * category_width;
                    if to_top > 0.0 {
                        grid.translated(x, 0.0).draw(&UShape::Line {
                            dx: 0.0,
                            dy: -to_top,
                        });
                    }
                    if to_bottom > 0.0 {
                        grid.translated(x, 0.0).draw(&UShape::Line {
                            dx: 0.0,
                            dy: to_bottom,
                        });
                    }
                }
            }
            for (i, label) in self.x_axis_labels.iter().enumerate() {
                if i % spacing as usize == 0 {
                    label_below((i as f64 + 0.5) * category_width, label);
                }
            }
        }
        if let Some(title) = self.x_axis_title() {
            if self.x_axis_label_position == LabelPosition::Right {
                let block = self.block(title, &font, HorizontalAlignment::Left);
                let title_height = block.calculate_dimension(ug.string_bounder()).height;
                block.draw_u(&ug.translated(width + 10.0, -title_height / 2.0));
            } else {
                let block = self.block(title, &font, HorizontalAlignment::Center);
                let title_width = block.calculate_dimension(ug.string_bounder()).width;
                block.draw_u(&ug.translated(width / 2.0 - title_width / 2.0, TICK_SIZE + 25.0));
            }
        }
    }

    /// The spacing given to a numeric x axis, or a tenth of its range.
    fn x_tick_interval(&self) -> f64 {
        match self.x_axis_tick_spacing.filter(|&spacing| spacing > 0) {
            Some(spacing) => f64::from(spacing),
            None => (self.x_axis.max - self.x_axis.min) / 10.0,
        }
    }

    /// The text of horizontal bars' labels and values, whatever the styles say.
    fn small_font(font_color: &HColor) -> FontConfiguration {
        font_configuration(UFont::new("SansSerif", UFontFace::NORMAL, 10), font_color)
    }

    /// Horizontal bars' categories, down the left side.
    fn draw_categories_vertically_on_left(&self, ug: &UGraphic, height: f64, font_color: &HColor) {
        ug.draw(&UShape::Line {
            dx: 0.0,
            dy: height,
        });
        if self.x_axis_labels.is_empty() {
            return;
        }
        let font = Self::small_font(font_color);
        let category_height = height / self.x_axis_labels.len() as f64;
        for (i, label) in self.x_axis_labels.iter().enumerate() {
            let y = (i as f64 + 0.5) * category_height;
            ug.translated(-TICK_SIZE, y).draw(&UShape::Line {
                dx: TICK_SIZE,
                dy: 0.0,
            });
            let block = self.block(label, &font, HorizontalAlignment::Right);
            let dimension = block.calculate_dimension(ug.string_bounder());
            block.draw_u(&ug.translated(
                -TICK_SIZE - dimension.width - 5.0,
                y - dimension.height / 2.0,
            ));
        }
    }

    /// Horizontal bars' values, along the bottom.
    fn draw_y_axis_horizontally(&self, ug: &UGraphic, width: f64, font_color: &HColor) {
        ug.draw(&UShape::Line { dx: width, dy: 0.0 });
        let font = Self::small_font(font_color);
        let axis = self.y_axis;
        let range = axis.max - axis.min;
        for i in 0..=NUM_TICKS {
            let value = axis.min + f64::from(i) * range / f64::from(NUM_TICKS);
            let x = width * f64::from(i) / f64::from(NUM_TICKS);
            ug.translated(x, 0.0).draw(&UShape::Line {
                dx: 0.0,
                dy: TICK_SIZE,
            });
            let block = self.block(&format_fixed(value, 0), &font, HorizontalAlignment::Center);
            let label_width = block.calculate_dimension(ug.string_bounder()).width;
            block.draw_u(&ug.translated(x - label_width / 2.0, TICK_SIZE + 5.0));
        }
        if !axis.title.is_empty() {
            let block = self.block(&axis.title, &font, HorizontalAlignment::Center);
            let title_width = block.calculate_dimension(ug.string_bounder()).width;
            block.draw_u(&ug.translated(width / 2.0 - title_width / 2.0, TICK_SIZE + 25.0));
        }
    }

    /// Grid lines at the ticks of both numeric axes, a tenth of their range apart unless spaced otherwise.
    fn draw_grid_lines(&self, ug: &UGraphic, plot_width: f64, plot_height: f64) {
        let (grid_color, grid_stroke) = self.grid();
        let grid = ug.with_color(grid_color).with_stroke(grid_stroke);
        if self.x_grid_mode != GridMode::Off {
            let interval = self.x_tick_interval();
            let start = (self.x_axis.min / interval).floor() * interval;
            for value in ticks(self.x_axis, start, interval) {
                let x = self.x_axis.value_to_pixel(value, 0.0, plot_width);
                grid.translated(x, 0.0).draw(&UShape::Line {
                    dx: 0.0,
                    dy: plot_height,
                });
            }
        }
        if self.y_grid_mode != GridMode::Off {
            let axis = self.y_axis;
            let interval = match axis.tick_spacing.filter(|_| axis.has_tick_spacing()) {
                Some(spacing) => spacing,
                None => (axis.max - axis.min) / 10.0,
            };
            let start = (axis.min / interval).ceil() * interval;
            for value in ticks(axis, start, interval) {
                let y = plot_height * (1.0 - (value - axis.min) / (axis.max - axis.min));
                grid.translated(0.0, y).draw(&UShape::Line {
                    dx: plot_width,
                    dy: 0.0,
                });
            }
        }
    }

    fn axis_of(&self, series: &ChartSeries) -> &ChartAxis {
        match self.y2_axis {
            Some(y2_axis) if series.use_secondary_axis => y2_axis,
            _ => self.y_axis,
        }
    }

    fn plot<'b>(&'b self, plot_width: f64, axis: &'b ChartAxis) -> Plot<'b> {
        Plot {
            skin: self.skin,
            width: plot_width,
            height: PLOT_HEIGHT,
            category_count: self.x_axis_labels.len(),
            axis,
            x_axis: self.x_axis,
        }
    }

    /// Bars first, then lines and scatters, then areas stacked on each other.
    fn draw_series(&self, ug: &UGraphic, plot_width: f64) {
        let has_coordinate_pairs = self
            .series
            .first()
            .is_some_and(ChartSeries::has_explicit_x_values);
        if (self.x_axis_labels.is_empty() && !has_coordinate_pairs) || self.series.is_empty() {
            return;
        }
        let bars: Vec<&ChartSeries> = self
            .series
            .iter()
            .filter(|series| series.series_type == SeriesType::Bar)
            .collect();
        if let Some(first) = bars.first() {
            let colors: Vec<HColor> = bars
                .iter()
                .map(|series| self.fill_color(SName::Bar, series))
                .collect();
            let renderer = BarRenderer {
                skin: self.skin,
                plot_width,
                plot_height: PLOT_HEIGHT,
                category_count: self.x_axis_labels.len(),
                axis: self.axis_of(first),
                horizontal: self.is_horizontal(),
            };
            if bars.len() == 1 {
                renderer.draw(ug, first, &colors[0]);
            } else if self.stack_mode == StackMode::Stacked {
                renderer.draw_stacked(ug, &bars, &colors);
            } else {
                renderer.draw_grouped(ug, &bars, &colors);
            }
        }
        let mut areas = Vec::new();
        for series in self.series {
            match series.series_type {
                SeriesType::Bar => {}
                SeriesType::Area => areas.push((series, self.fill_color(SName::Area, series))),
                SeriesType::Line => {
                    let color = self.line_color(SName::Line, series);
                    self.plot(plot_width, self.axis_of(series))
                        .draw_line(ug, series, &color);
                }
                SeriesType::Scatter => self
                    .plot(plot_width, self.axis_of(series))
                    .draw_scatter(ug, series),
            }
        }
        if let Some((first, _)) = areas.first() {
            let plot = self.plot(plot_width, self.axis_of(first));
            let mut cumulative: Option<Vec<f64>> = None;
            for (series, color) in &areas {
                plot.draw_area(ug, series, color, cumulative.as_deref());
                match &mut cumulative {
                    None => cumulative = Some(series.values.clone()),
                    Some(cumulative) => {
                        for (sum, value) in cumulative.iter_mut().zip(&series.values) {
                            *sum += value;
                        }
                        let known = cumulative.len();
                        cumulative.extend(series.values.iter().skip(known));
                    }
                }
            }
        }
    }

    /// The series' own colour, or its style's background.
    fn fill_color(&self, name: SName, series: &ChartSeries) -> HColor {
        series.color.clone().unwrap_or_else(|| {
            self.series_style(name, series)
                .value(PName::BackGroundColor)
                .as_color()
        })
    }

    /// The series' own colour, or its style's line colour.
    fn line_color(&self, name: SName, series: &ChartSeries) -> HColor {
        series.color.clone().unwrap_or_else(|| {
            self.series_style(name, series)
                .value(PName::LineColor)
                .as_color()
        })
    }

    fn is_legend_beside(&self) -> bool {
        matches!(
            self.legend_position,
            LegendPosition::Left | LegendPosition::Right
        )
    }

    fn calculate_legend_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        if self.legend_position == LegendPosition::None || self.series.is_empty() {
            return XDimension2D::new(0.0, 0.0);
        }
        let font = self.chart_style(SName::Legend).font_configuration();
        let mut max_width: f64 = 0.0;
        let mut total_height: f64 = 0.0;
        for series in self.series {
            let dimension = self
                .block(&series.name, &font, HorizontalAlignment::Left)
                .calculate_dimension(string_bounder);
            if self.is_legend_beside() {
                max_width = max_width.max(dimension.width);
                total_height += dimension.height + LEGEND_ITEM_SPACING;
            } else {
                max_width += dimension.width
                    + LEGEND_SYMBOL_SIZE
                    + LEGEND_TEXT_SPACING
                    + LEGEND_ITEM_SPACING;
                total_height = total_height.max(dimension.height);
            }
        }
        if self.is_legend_beside() {
            XDimension2D::new(
                max_width + LEGEND_SYMBOL_SIZE + LEGEND_TEXT_SPACING + LEGEND_MARGIN * 2.0,
                total_height,
            )
        } else {
            XDimension2D::new(max_width, total_height + LEGEND_MARGIN * 2.0)
        }
    }

    fn draw_legend(&self, ug: &UGraphic, left_margin: f64, top_margin: f64, plot_width: f64) {
        if self.series.is_empty() {
            return;
        }
        let (mut x, mut y) = match self.legend_position {
            LegendPosition::None => return,
            LegendPosition::Left => (MARGIN, top_margin),
            LegendPosition::Right => (
                left_margin
                    + plot_width
                    + if self.y2_axis.is_some() {
                        AXIS_LABEL_SPACE
                    } else {
                        0.0
                    }
                    + LEGEND_MARGIN,
                top_margin,
            ),
            LegendPosition::Top => (left_margin, MARGIN),
            LegendPosition::Bottom => (
                left_margin,
                top_margin + PLOT_HEIGHT + AXIS_LABEL_SPACE + LEGEND_MARGIN,
            ),
        };
        let font = self.chart_style(SName::Legend).font_configuration();
        let square = UShape::Rectangle(URectangle::new(LEGEND_SYMBOL_SIZE, LEGEND_SYMBOL_SIZE));
        for series in self.series {
            match series.series_type {
                SeriesType::Bar | SeriesType::Area => {
                    let name = if series.series_type == SeriesType::Bar {
                        SName::Bar
                    } else {
                        SName::Area
                    };
                    let color = self.fill_color(name, series);
                    ug.with_color(color.clone())
                        .with_backcolor(color)
                        .translated(x, y)
                        .draw(&square);
                }
                SeriesType::Line => {
                    ug.with_color(self.line_color(SName::Line, series))
                        .with_stroke(UStroke::with_thickness(2.0))
                        .translated(x, y + LEGEND_SYMBOL_SIZE / 2.0)
                        .draw(&UShape::Line {
                            dx: LEGEND_SYMBOL_SIZE,
                            dy: 0.0,
                        });
                }
                SeriesType::Scatter => {
                    let style = self.series_style(SName::Scatter, series);
                    let shape = MarkerShape::named(&style.value(PName::MarkerShape).as_string())
                        .unwrap_or(series.marker_shape);
                    draw_marker(
                        ug,
                        &style.value(PName::MarkerColor).as_color(),
                        x + LEGEND_SYMBOL_SIZE / 2.0,
                        y + LEGEND_SYMBOL_SIZE / 2.0,
                        LEGEND_SYMBOL_SIZE * 0.7,
                        shape,
                    );
                }
            }
            let block = self.block(&series.name, &font, HorizontalAlignment::Left);
            let dimension = block.calculate_dimension(ug.string_bounder());
            block.draw_u(&ug.translated(x + LEGEND_SYMBOL_SIZE + LEGEND_TEXT_SPACING, y));
            if self.is_legend_beside() {
                y += dimension.height + LEGEND_ITEM_SPACING;
            } else {
                x += LEGEND_SYMBOL_SIZE
                    + LEGEND_TEXT_SPACING
                    + dimension.width
                    + LEGEND_ITEM_SPACING;
            }
        }
    }

    /// Where an annotation's point is on the plot.
    fn annotation_point(
        &self,
        annotation: &ChartAnnotation,
        plot_width: f64,
    ) -> Option<(f64, f64)> {
        let axis = self.y_axis;
        let ratio = |value: f64| (value - axis.min) / (axis.max - axis.min);
        let (mut x, mut y) = (0.0, 0.0);
        match &annotation.x_position {
            AnnotationX::Number(value) => {
                if self.is_horizontal() {
                    y = PLOT_HEIGHT * (1.0 - ratio(*value));
                } else {
                    x = plot_width * ratio(*value);
                }
            }
            AnnotationX::Label(label) => {
                let index = self.x_axis_labels.iter().position(|known| known == label)?;
                let count = self.x_axis_labels.len() as f64;
                if self.is_horizontal() {
                    y = PLOT_HEIGHT * (index as f64 + 0.5) / count;
                } else {
                    x = plot_width * (index as f64 + 0.5) / count;
                }
            }
        }
        if self.is_horizontal() {
            x = plot_width * ratio(annotation.y_position);
        } else {
            y = PLOT_HEIGHT * (1.0 - ratio(annotation.y_position));
        }
        Some((x, y))
    }

    /// Each text above its point, or 40 pixels away with an arrow, below it when the top is too near.
    fn draw_annotations(&self, ug: &UGraphic, plot_width: f64) {
        if self.annotations.is_empty() {
            return;
        }
        let style = self.chart_style(SName::Annotation);
        let font = style.font_configuration();
        let arrow_color = style.value(PName::LineColor).as_color();
        let arrow = ug
            .with_color(arrow_color.clone())
            .with_stroke(UStroke::with_thickness(
                style.value(PName::LineThickness).as_double(),
            ));
        let arrowhead = ug
            .with_color(arrow_color.clone())
            .with_backcolor(arrow_color);
        for annotation in self.annotations {
            let Some((x, y)) = self.annotation_point(annotation, plot_width) else {
                continue;
            };
            let block = self.block(&annotation.text, &font, HorizontalAlignment::Left);
            let dimension = block.calculate_dimension(ug.string_bounder());
            let text_x = x - dimension.width / 2.0;
            if !annotation.show_arrow {
                block.draw_u(&ug.translated(text_x, y - dimension.height - 5.0));
                continue;
            }
            if y > dimension.height + 40.0 {
                let text_y = y - dimension.height - 40.0;
                block.draw_u(&ug.translated(text_x, text_y));
                let start_y = text_y + dimension.height + 5.0;
                let end_y = y - 5.0;
                let length = end_y - start_y;
                if length > 0.0 {
                    arrow.translated(x, start_y).draw(&UShape::Line {
                        dx: 0.0,
                        dy: length,
                    });
                    arrowhead.translated(x, end_y).draw(&UShape::polygon(vec![
                        (0.0, 0.0),
                        (-5.0, -8.0),
                        (5.0, -8.0),
                    ]));
                }
            } else {
                let text_y = y + 40.0;
                block.draw_u(&ug.translated(text_x, text_y));
                let start_y = text_y - 5.0;
                let end_y = y + 5.0;
                let length = start_y - end_y;
                if length > 0.0 {
                    arrow.translated(x, start_y).draw(&UShape::Line {
                        dx: 0.0,
                        dy: -length,
                    });
                    arrowhead.translated(x, end_y).draw(&UShape::polygon(vec![
                        (0.0, 0.0),
                        (-5.0, 8.0),
                        (5.0, 8.0),
                    ]));
                }
            }
        }
    }
}

impl TextBlock for ChartRenderer<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let legend = self.calculate_legend_dimension(string_bounder);
        let mut width = MARGIN
            + AXIS_LABEL_SPACE
            + self.plot_width()
            + if self.y2_axis.is_some() {
                AXIS_LABEL_SPACE
            } else {
                0.0
            }
            + MARGIN;
        let mut height = MARGIN + TITLE_SPACE + PLOT_HEIGHT + AXIS_LABEL_SPACE + MARGIN;
        if self.x_axis_title().is_some() {
            height += 20.0;
        }
        if self.is_legend_beside() {
            width += legend.width + LEGEND_MARGIN;
        } else if self.legend_position != LegendPosition::None {
            height += legend.height + LEGEND_MARGIN;
        }
        XDimension2D::new(width, height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let style = self.style_of(&[SName::Root, SName::Element, SName::ChartDiagram]);
        let font_color = style.value(PName::FontColor).as_color();
        let ug = ug
            .with_color(style.value(PName::LineColor).as_color())
            .with_stroke(UStroke::with_thickness(1.0));
        let legend = self.calculate_legend_dimension(ug.string_bounder());
        let mut left_margin = MARGIN + AXIS_LABEL_SPACE;
        let mut top_margin = MARGIN + TITLE_SPACE;
        let plot_width = self.plot_width();
        match self.legend_position {
            LegendPosition::Left => left_margin += legend.width + LEGEND_MARGIN,
            LegendPosition::Top => top_margin += legend.height + LEGEND_MARGIN,
            _ => {}
        }
        if self.is_horizontal() {
            self.draw_categories_vertically_on_left(
                &ug.translated(left_margin, top_margin),
                PLOT_HEIGHT,
                &font_color,
            );
            self.draw_y_axis_horizontally(
                &ug.translated(left_margin, top_margin + PLOT_HEIGHT),
                plot_width,
                &font_color,
            );
        } else {
            let x_axis_y = match self.zero_ratio() {
                Some(zero_ratio) => top_margin + PLOT_HEIGHT * (1.0 - zero_ratio),
                None => top_margin + PLOT_HEIGHT,
            };
            let y_axis_x = if self.x_axis.min <= 0.0 && self.x_axis.max >= 0.0 {
                left_margin + self.x_axis.value_to_pixel(0.0, 0.0, plot_width)
            } else {
                left_margin
            };
            self.draw_y_axis(
                &ug.translated(y_axis_x, top_margin),
                PLOT_HEIGHT,
                self.y_axis,
                true,
            );
            if let Some(y2_axis) = self.y2_axis {
                self.draw_y_axis(
                    &ug.translated(left_margin + plot_width, top_margin),
                    PLOT_HEIGHT,
                    y2_axis,
                    false,
                );
            }
            self.draw_x_axis(
                &ug.translated(left_margin, x_axis_y),
                plot_width,
                PLOT_HEIGHT,
            );
        }
        let ug_plot = ug.translated(left_margin, top_margin);
        self.draw_grid_lines(&ug_plot, plot_width, PLOT_HEIGHT);
        self.draw_series(&ug_plot, plot_width);
        self.draw_annotations(&ug_plot, plot_width);
        self.draw_legend(&ug, left_margin, top_margin, plot_width);
    }
}

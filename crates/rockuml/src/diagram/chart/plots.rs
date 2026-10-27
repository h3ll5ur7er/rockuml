//! Lines, areas and scatters (PlantUML's `LineRenderer`, `AreaRenderer` and `ScatterRenderer`).

use super::text::draw_value_label;
use super::{ChartAxis, ChartSeries, MarkerShape};
use crate::color::HColor;
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};

/// What the plots of a chart share: the plot's size, its categories and the axes values go by.
pub(super) struct Plot<'a> {
    pub skin: &'a SkinParam,
    pub width: f64,
    pub height: f64,
    pub category_count: usize,
    pub axis: &'a ChartAxis,
    pub x_axis: &'a ChartAxis,
}

impl Plot<'_> {
    fn category_width(&self) -> f64 {
        self.width / self.category_count as f64
    }

    /// The middle of category `index`.
    fn category_x(&self, index: usize) -> f64 {
        (index as f64 + 0.5) * self.category_width()
    }

    fn value_y(&self, value: f64) -> f64 {
        self.height - (value - self.axis.min) / (self.axis.max - self.axis.min) * self.height
    }

    /// The points of a series: at its coordinates, or in the middle of the categories it has values for.
    fn points(&self, series: &ChartSeries) -> Vec<(f64, f64, f64)> {
        match &series.x_values {
            Some(x_values) => x_values
                .iter()
                .zip(&series.values)
                .map(|(&x, &y)| {
                    (
                        self.x_axis.value_to_pixel(x, 0.0, self.width),
                        self.value_y(y),
                        y,
                    )
                })
                .collect(),
            None => series
                .values
                .iter()
                .take(self.category_count)
                .enumerate()
                .map(|(i, &value)| (self.category_x(i), self.value_y(value), value))
                .collect(),
        }
    }

    /// A thick segment from one point to the next.
    fn draw_segments(ug: &UGraphic, color: &HColor, points: &[(f64, f64)]) {
        for pair in points.windows(2) {
            let [(x1, y1), (x2, y2)] = pair else {
                unreachable!("windows of two")
            };
            ug.with_color(color.clone())
                .with_stroke(UStroke::with_thickness(2.0))
                .translated(*x1, *y1)
                .draw(&UShape::Line {
                    dx: x2 - x1,
                    dy: y2 - y1,
                });
        }
    }

    /// `LineRenderer.draw`: the points joined, without markers.
    pub(super) fn draw_line(&self, ug: &UGraphic, series: &ChartSeries, color: &HColor) {
        if self.category_count == 0 && !series.has_explicit_x_values() {
            return;
        }
        let points: Vec<(f64, f64)> = self
            .points(series)
            .iter()
            .map(|&(x, y, _)| (x, y))
            .collect();
        Self::draw_segments(ug, color, &points);
    }

    /// `AreaRenderer.draw`: the values filled down to `baseline`, the top of the areas below, or to the
    /// axis's minimum.
    pub(super) fn draw_area(
        &self,
        ug: &UGraphic,
        series: &ChartSeries,
        color: &HColor,
        baseline: Option<&[f64]>,
    ) {
        if self.category_count == 0 {
            return;
        }
        let base = |i: usize| baseline.and_then(|baseline| baseline.get(i).copied());
        let count = series.values.len().min(self.category_count);
        let tops: Vec<(f64, f64)> = (0..count)
            .map(|i| {
                let value = series.values[i];
                let top = base(i).map_or(value, |base| base + value);
                (self.category_x(i), self.value_y(top))
            })
            .collect();
        let bottoms = (0..count).rev().map(|i| {
            (
                self.category_x(i),
                self.value_y(base(i).unwrap_or(self.axis.min)),
            )
        });
        let polygon = tops.iter().copied().chain(bottoms).collect();
        ug.with_color(color.clone())
            .with_backcolor(color.clone())
            .draw(&UShape::polygon(polygon));
        Self::draw_segments(ug, color, &tops);
        if series.show_labels {
            for (&(x, y), &value) in tops.iter().zip(&series.values) {
                draw_value_label(ug, self.skin, value, x, y - 8.0);
            }
        }
    }

    /// `ScatterRenderer.draw`: a marker at each point, in the style's marker colour, size and shape.
    pub(super) fn draw_scatter(&self, ug: &UGraphic, series: &ChartSeries) {
        if self.category_count == 0 && !series.has_explicit_x_values() {
            return;
        }
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ChartDiagram,
            SName::Scatter,
        ])
        .get_merged_style_with(
            &self.skin.current_style_builder(),
            series.stereotype.as_ref(),
        );
        let marker_color = style.value(PName::MarkerColor).as_color();
        let size = match style.value(PName::MarkerSize).as_double() {
            size if size > 0.0 => size,
            _ => 8.0,
        };
        let shape = MarkerShape::named(&style.value(PName::MarkerShape).as_string())
            .unwrap_or(series.marker_shape);
        for (x, y, value) in self.points(series) {
            draw_marker(ug, &marker_color, x, y, size, shape);
            if series.show_labels {
                draw_value_label(ug, self.skin, value, x, y - size / 2.0 - 8.0);
            }
        }
    }
}

/// A marker of `size` centred on (`x`, `y`).
pub(super) fn draw_marker(
    ug: &UGraphic,
    color: &HColor,
    x: f64,
    y: f64,
    size: f64,
    shape: MarkerShape,
) {
    let ug = ug.with_color(color.clone()).with_backcolor(color.clone());
    match shape {
        MarkerShape::Circle => ug
            .translated(x - size / 2.0, y - size / 2.0)
            .draw(&UShape::Ellipse(UEllipse::new(size, size))),
        MarkerShape::Square => ug
            .translated(x - size / 2.0, y - size / 2.0)
            .draw(&UShape::Rectangle(URectangle::new(size, size))),
        MarkerShape::Triangle => ug.translated(x, y).draw(&UShape::polygon(vec![
            (0.0, -size / 2.0),
            (-size / 2.0, size / 2.0),
            (size / 2.0, size / 2.0),
        ])),
    }
}

//! Bars: one series, several side by side or stacked, upright or across (PlantUML's `BarRenderer`).

use super::text::{draw_value_label, value_label};
use super::{ChartAxis, ChartSeries};
use crate::color::HColor;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// The part of a category a bar takes unless the style's `BarWidth` says otherwise.
const DEFAULT_BAR_WIDTH_RATIO: f64 = 0.6;

pub(super) struct BarRenderer<'a> {
    pub skin: &'a SkinParam,
    pub plot_width: f64,
    pub plot_height: f64,
    pub category_count: usize,
    pub axis: &'a ChartAxis,
    pub horizontal: bool,
}

impl BarRenderer<'_> {
    fn bar_style(&self, series: Option<&ChartSeries>) -> Style {
        StyleSignature::of(&[SName::Root, SName::Element, SName::ChartDiagram, SName::Bar])
            .get_merged_style_with(
                &self.skin.current_style_builder(),
                series.and_then(|series| series.stereotype.as_ref()),
            )
    }

    fn bar_width_ratio(style: &Style) -> f64 {
        let ratio = style.value(PName::BarWidth).as_double();
        if ratio > 0.0 && ratio <= 1.0 {
            ratio
        } else {
            DEFAULT_BAR_WIDTH_RATIO
        }
    }

    /// Where a value lies on the plot, from its top.
    fn value_y(&self, value: f64) -> f64 {
        self.plot_height
            - (value - self.axis.min) / (self.axis.max - self.axis.min) * self.plot_height
    }

    /// Where bars start: zero, or the plot's edge nearest to it.
    fn zero_y(&self) -> f64 {
        self.value_y(0.0).min(self.plot_height).max(0.0)
    }

    fn draw_bar(ug: &UGraphic, style: &Style, color: &HColor, width: f64, height: f64) {
        ug.with_color(style.value(PName::LineColor).as_color())
            .with_stroke(UStroke::with_thickness(
                style.value(PName::LineThickness).as_double(),
            ))
            .with_backcolor(color.clone())
            .draw(&UShape::Rectangle(URectangle::new(width, height)));
    }

    pub(super) fn draw(&self, ug: &UGraphic, series: &ChartSeries, color: &HColor) {
        if self.category_count == 0 {
            return;
        }
        if self.horizontal {
            self.draw_horizontal(ug, series, color);
        } else {
            self.draw_vertical(ug, series, color);
        }
    }

    fn draw_vertical(&self, ug: &UGraphic, series: &ChartSeries, color: &HColor) {
        let category_width = self.plot_width / self.category_count as f64;
        let style = self.bar_style(Some(series));
        let bar_width = category_width * Self::bar_width_ratio(&style);
        let bar_offset = (category_width - bar_width) / 2.0;
        for (i, &value) in series.values.iter().take(self.category_count).enumerate() {
            let x = i as f64 * category_width + bar_offset;
            let (y, bar_height) = self.bar_extent(value);
            if bar_height > 0.0 {
                Self::draw_bar(&ug.translated(x, y), &style, color, bar_width, bar_height);
            }
            if series.show_labels {
                self.draw_label(ug, value, x + bar_width / 2.0, y, bar_height);
            }
        }
    }

    /// The top of a bar from zero to `value`, and its height.
    fn bar_extent(&self, value: f64) -> (f64, f64) {
        let zero_y = self.zero_y();
        let value_y = self.value_y(value);
        if value < 0.0 {
            (zero_y, value_y - zero_y)
        } else {
            (value_y, zero_y - value_y)
        }
    }

    /// Above a bar, or below it for a negative value.
    fn draw_label(&self, ug: &UGraphic, value: f64, x: f64, y: f64, bar_height: f64) {
        let label_y = if value < 0.0 {
            y + bar_height + 5.0
        } else {
            y - 5.0
        };
        draw_value_label(ug, self.skin, value, x, label_y);
    }

    fn draw_horizontal(&self, ug: &UGraphic, series: &ChartSeries, color: &HColor) {
        let category_height = self.plot_height / self.category_count as f64;
        let style = self.bar_style(Some(series));
        let bar_height = category_height * Self::bar_width_ratio(&style);
        let bar_offset = (category_height - bar_height) / 2.0;
        for (i, &value) in series.values.iter().take(self.category_count).enumerate() {
            let y = i as f64 * category_height + bar_offset;
            let bar_width =
                ((value - self.axis.min) / (self.axis.max - self.axis.min) * self.plot_width).abs();
            Self::draw_bar(&ug.translated(0.0, y), &style, color, bar_width, bar_height);
            if series.show_labels {
                let block = value_label(self.skin, value, HorizontalAlignment::Left);
                let height = block.calculate_dimension(ug.string_bounder()).height;
                block.draw_u(&ug.translated(bar_width + 5.0, y + bar_height / 2.0 - height / 2.0));
            }
        }
    }

    /// Several series side by side in each category, sharing 80% of its width.
    pub(super) fn draw_grouped(&self, ug: &UGraphic, bars: &[&ChartSeries], colors: &[HColor]) {
        if self.category_count == 0 || bars.is_empty() {
            return;
        }
        let category_width = self.plot_width / self.category_count as f64;
        let group_width = category_width * 0.8;
        let bar_width = group_width / bars.len() as f64;
        let group_offset = (category_width - group_width) / 2.0;
        let style = self.bar_style(None);
        for category in 0..self.category_count {
            for (index, (series, color)) in bars.iter().zip(colors).enumerate() {
                let Some(&value) = series.values.get(category) else {
                    continue;
                };
                let x = category as f64 * category_width + group_offset + index as f64 * bar_width;
                let (y, bar_height) = self.bar_extent(value);
                Self::draw_bar(&ug.translated(x, y), &style, color, bar_width, bar_height);
                if series.show_labels {
                    self.draw_label(ug, value, x + bar_width / 2.0, y, bar_height);
                }
            }
        }
    }

    /// Several series piled up in each category: positive values upwards from zero, negative ones downwards.
    pub(super) fn draw_stacked(&self, ug: &UGraphic, bars: &[&ChartSeries], colors: &[HColor]) {
        if self.category_count == 0 || bars.is_empty() {
            return;
        }
        let category_width = self.plot_width / self.category_count as f64;
        let style = self.bar_style(None);
        let bar_width = category_width * Self::bar_width_ratio(&style);
        let bar_offset = (category_width - bar_width) / 2.0;
        for category in 0..self.category_count {
            let zero_y = self.value_y(0.0);
            let mut positive_y = zero_y;
            let mut negative_y = zero_y;
            for (series, color) in bars.iter().zip(colors) {
                let Some(&value) = series.values.get(category) else {
                    continue;
                };
                let x = category as f64 * category_width + bar_offset;
                let value_y = self.value_y(value);
                let (y, bar_height) = if value < 0.0 {
                    let bar_height = value_y - zero_y;
                    let y = negative_y;
                    negative_y += bar_height;
                    (y, bar_height)
                } else {
                    let bar_height = zero_y - value_y;
                    positive_y -= bar_height;
                    (positive_y, bar_height)
                };
                Self::draw_bar(&ug.translated(x, y), &style, color, bar_width, bar_height);
                if series.show_labels {
                    self.draw_label(ug, value, x + bar_width / 2.0, y, bar_height);
                }
            }
        }
    }
}

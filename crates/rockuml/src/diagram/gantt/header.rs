//! Where days go across the chart, and the days, weeks, months, quarters or years written above and below it
//! (PlantUML's `gantt.timescale` and `gantt.draw.header` packages and `TimelineStyleData`).

use std::cell::RefCell;
use std::collections::HashMap;

use super::calendar::OpenClose;
use super::time::{TimePoint, TimePointFormat};
use super::{GanttDiagram, PrintScale, TimeBounds, WeeklyHeaderStrategy};
use crate::color::HColor;
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::local_date::LocalDate;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

const MILLISECONDS_PER_DAY: f64 = 86_400_000.0;

/// Where each instant goes across the chart (`TimeScale` and its implementations).
pub(super) enum TimeScale {
    /// Proportional to time since 1970 (`TimeScaleWink`).
    Wink {
        cell_width: f64,
        print_scale: PrintScale,
    },
    /// Proportional to time since the first day (`TimeScaleDaily`), or a compressed scale breaking weeks on
    /// Sundays (`TimeScaleCompressed`).
    Daily {
        cell_width: f64,
        delta: f64,
        compressed: bool,
    },
    /// Closed days take no room (`TimeScaleDailyHideClosed`).
    DailyHideClosed {
        cell_width: f64,
        starting_day: TimePoint,
        open_close: OpenClose,
        /// The open days before each day, as far as computed.
        starting_int: RefCell<HashMap<TimePoint, i64>>,
        biggest: RefCell<TimePoint>,
    },
}

fn wink_position(cell_width: f64, instant: TimePoint) -> f64 {
    instant.millis() as f64 * cell_width / MILLISECONDS_PER_DAY
}

impl TimeScale {
    fn daily(cell_width: f64, start: TimePoint, compressed: bool) -> Self {
        Self::Daily {
            cell_width,
            delta: wink_position(cell_width, start),
            compressed,
        }
    }

    fn daily_hide_closed(cell_width: f64, starting_day: TimePoint, open_close: OpenClose) -> Self {
        Self::DailyHideClosed {
            cell_width,
            starting_day,
            open_close,
            starting_int: RefCell::new([(starting_day, 0)].into()),
            biggest: RefCell::new(starting_day),
        }
    }

    pub(super) fn position(&self, instant: TimePoint) -> f64 {
        match self {
            Self::Wink { cell_width, .. } => wink_position(*cell_width, instant),
            Self::Daily {
                cell_width, delta, ..
            } => wink_position(*cell_width, instant) - delta,
            Self::DailyHideClosed {
                cell_width,
                starting_day,
                starting_int,
                biggest,
                ..
            } => {
                assert!(
                    instant >= *starting_day,
                    "the chart starts at its first day"
                );
                let mut biggest = biggest.borrow_mut();
                let mut starting_int = starting_int.borrow_mut();
                if instant > *biggest {
                    let mut day = *biggest;
                    let mut current = starting_int[&day];
                    while day < instant {
                        if self.width(day) > 0.0 {
                            current += 1;
                        }
                        day = day.increment();
                        starting_int.insert(day, current);
                    }
                    *biggest = day;
                }
                starting_int.get(&instant).copied().unwrap_or_default() as f64 * cell_width
            }
        }
    }

    pub(super) fn width(&self, instant: TimePoint) -> f64 {
        match self {
            Self::Wink { cell_width, .. } | Self::Daily { cell_width, .. } => *cell_width,
            Self::DailyHideClosed {
                cell_width,
                open_close,
                ..
            } => {
                if open_close.is_closed(instant.to_day()) {
                    0.0
                } else {
                    *cell_width
                }
            }
        }
    }

    /// Whether the resource loads total up to the instant (`isBreaking`).
    pub(super) fn is_breaking(&self, instant: TimePoint) -> bool {
        match self {
            Self::Wink {
                print_scale: PrintScale::Weekly,
                ..
            } => (instant.millis() / 86_400_000) % 7 == 6,
            Self::Daily {
                compressed: true, ..
            } => instant.to_day_of_week() == crate::local_date::DayOfWeek::Sunday,
            _ => true,
        }
    }
}

/// The styles of the time line (`TimelineStyleData`).
pub(super) struct TimelineStyle<'a> {
    diagram: &'a GanttDiagram,
}

impl<'a> TimelineStyle<'a> {
    pub(super) fn new(diagram: &'a GanttDiagram) -> Self {
        Self { diagram }
    }

    pub(super) fn style(&self, names: &[SName]) -> Style {
        self.diagram.style(names)
    }

    fn font_size(&self, name: SName) -> f64 {
        self.style(&[SName::Timeline, name])
            .value(PName::FontSize)
            .as_double()
    }

    pub(super) fn font_size_day(&self) -> f64 {
        self.font_size(SName::Day)
    }

    fn font_size_month(&self) -> f64 {
        self.font_size(SName::Month)
    }

    fn font_size_year(&self) -> f64 {
        self.font_size(SName::Year)
    }

    fn font(&self, name: SName) -> UFont {
        self.style(&[SName::Timeline, name]).ufont()
    }

    fn closed_background_color(&self) -> HColor {
        self.style(&[SName::Closed])
            .value(PName::BackGroundColor)
            .as_color()
    }

    fn closed_font_color(&self) -> HColor {
        self.style(&[SName::Closed])
            .value(PName::FontColor)
            .as_color()
    }

    fn open_font_color(&self) -> HColor {
        self.style(&[SName::Timeline])
            .value(PName::FontColor)
            .as_color()
    }

    pub(super) fn line_color(&self) -> HColor {
        self.style(&[SName::Timeline])
            .value(PName::LineColor)
            .as_color()
    }

    fn apply_vertical_separator_style(&self, ug: &UGraphic) -> UGraphic {
        let style = self.style(&[SName::VerticalSeparator]);
        ug.with_color(style.value(PName::LineColor).as_color())
            .with_stroke(style.stroke())
    }

    pub(super) fn cell_width(&self) -> f64 {
        self.font_size_day() * 1.6
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HeaderKind {
    /// Day numbers, for a project without dates (`TimeHeaderSimple`).
    Simple,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

/// The time line above and below the chart (`TimeHeader` and its implementations).
pub(super) struct TimeHeader<'a> {
    kind: HeaderKind,
    time_scale: TimeScale,
    diagram: &'a GanttDiagram,
    bounds: TimeBounds,
    timeline_style: TimelineStyle<'a>,
    /// How many days the simple header steps by, once known.
    simple_delta: RefCell<i64>,
}

impl<'a> TimeHeader<'a> {
    /// `TimeHeaderFactory.createTimeHeader`.
    pub(super) fn new(diagram: &'a GanttDiagram, bounds: TimeBounds) -> Self {
        let timeline_style = TimelineStyle::new(diagram);
        let cell_width = timeline_style.cell_width()
            * (diagram.print_scale.default_scale() * diagram.factor_scale);
        let start = TimePoint::of_start_of_day(bounds.min_day);
        let zero_day = diagram.print_start.map(TimePoint::of_start_of_day);
        let (kind, time_scale) = if diagram.is_relative() {
            (
                HeaderKind::Simple,
                TimeScale::Wink {
                    cell_width,
                    print_scale: diagram.print_scale,
                },
            )
        } else {
            let kind = match diagram.print_scale {
                PrintScale::Daily => HeaderKind::Daily,
                PrintScale::Weekly => HeaderKind::Weekly,
                PrintScale::Monthly => HeaderKind::Monthly,
                PrintScale::Quarterly => HeaderKind::Quarterly,
                PrintScale::Yearly => HeaderKind::Yearly,
            };
            let scale_start = zero_day.unwrap_or(start);
            let time_scale = if kind == HeaderKind::Daily && diagram.hide_closed {
                TimeScale::daily_hide_closed(
                    cell_width,
                    start,
                    diagram.model.calendar.open_close.clone(),
                )
            } else {
                TimeScale::daily(cell_width, scale_start, kind != HeaderKind::Daily)
            };
            (kind, time_scale)
        };
        Self {
            kind,
            time_scale,
            diagram,
            bounds,
            timeline_style,
            simple_delta: RefCell::new(0),
        }
    }

    pub(super) fn time_scale(&self) -> &TimeScale {
        &self.time_scale
    }

    fn min_day(&self) -> LocalDate {
        self.bounds.min_day
    }

    fn max_day(&self) -> LocalDate {
        self.bounds.max_day
    }

    fn days(&self, inclusive: bool) -> impl Iterator<Item = LocalDate> + use<> {
        let max = self.max_day();
        std::iter::successors(Some(self.min_day()), |day| Some(day.plus_days(1)))
            .take_while(move |day| if inclusive { *day <= max } else { *day < max })
    }

    fn position(&self, day: LocalDate) -> f64 {
        self.time_scale.position(TimePoint::of_start_of_day(day))
    }

    fn language(&self) -> &str {
        &self.diagram.language
    }

    fn has_name_days(&self) -> bool {
        !self.diagram.model.calendar.name_days().is_empty()
    }

    fn header_name_day_height(&self) -> f64 {
        if self.has_name_days() {
            self.timeline_style.font_size_day() + 6.0
        } else {
            0.0
        }
    }

    pub(super) fn time_header_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        let style = &self.timeline_style;
        match self.kind {
            HeaderKind::Simple => style.font_size_day() + 6.0,
            HeaderKind::Daily => {
                style.font_size_month()
                    + 2.0
                    + style.font_size_day()
                    + 2.0
                    + style.font_size_day()
                    + 3.0
            }
            HeaderKind::Weekly => style.font_size_month() + 4.0 + style.font_size_day() + 1.0,
            HeaderKind::Monthly => style.font_size_year() + 2.0 + style.font_size_month() + 2.0,
            HeaderKind::Quarterly => style.font_size_year() + style.font_size_month() + 5.0,
            HeaderKind::Yearly => style.font_size_year() + 3.0,
        }
    }

    pub(super) fn time_footer_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        let style = &self.timeline_style;
        match self.kind {
            HeaderKind::Simple => style.font_size_day() + 6.0,
            HeaderKind::Daily => {
                style.font_size_day() + style.font_size_day() + style.font_size_month() + 8.0
            }
            HeaderKind::Weekly => style.font_size_month() + 4.0,
            HeaderKind::Monthly | HeaderKind::Quarterly => {
                style.font_size_year() + style.font_size_month() + 5.0
            }
            HeaderKind::Yearly => style.font_size_year() + 3.0,
        }
    }

    pub(super) fn full_header_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        let height = self.time_header_height(string_bounder);
        match self.kind {
            HeaderKind::Daily | HeaderKind::Weekly | HeaderKind::Monthly => {
                height + self.header_name_day_height()
            }
            _ => height,
        }
    }

    fn font_configuration(font: &UFont, bold: bool, color: HColor) -> FontConfiguration {
        let font = if bold {
            font.with_face(UFontFace::BOLD)
        } else {
            font.clone()
        };
        FontConfiguration::new(font, color, 8)
    }

    fn font_configuration_of(&self, name: SName, bold: bool, color: HColor) -> FontConfiguration {
        Self::font_configuration(&self.timeline_style.font(name), bold, color)
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

    fn draw_hline(&self, ug: &UGraphic, y: f64) {
        let xmin = self.position(self.min_day());
        let xmax = self.position(self.max_day().plus_days(1));
        ug.with_color(self.timeline_style.line_color())
            .translated(0.0, y)
            .draw(&UShape::Line {
                dx: xmax - xmin,
                dy: 0.0,
            });
    }

    fn draw_vline(ug: &UGraphic, x: f64, y1: f64, y2: f64) {
        ug.translated(x, y1).draw(&UShape::Line {
            dx: 0.0,
            dy: y2 - y1,
        });
    }

    fn is_zero_on_day(&self, instant: TimePoint) -> bool {
        self.kind != HeaderKind::Simple
            && self
                .diagram
                .model
                .calendar
                .open_close
                .as_piecewise_constant()
                .is_zero_on_day(instant.to_day())
    }

    /// The day colours, and closed days in grey, as rectangles under the chart (`drawColorsBackground`).
    fn draw_colors_background(&self, ug: &UGraphic, total_height_without_footer: f64) {
        let string_bounder = ug.string_bounder();
        let height = total_height_without_footer - self.full_header_height(string_bounder);
        let full_header_height = self.full_header_height(string_bounder);
        let calendar = &self.diagram.model.calendar;
        let draw = |color: &HColor, x1: f64, x2: f64| {
            if height == 0.0 {
                return;
            }
            let ug = ug
                .with_backcolor(color.clone())
                .with_color(HColor::NONE)
                .translated(x1, full_header_height);
            if x2 > x1 {
                ug.draw(&UShape::Rectangle(URectangle::new(x2 - x1, height)));
            }
        };
        let mut pending: Option<(HColor, f64, f64)> = None;
        for day in self.days(true) {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.time_scale.position(wink);
            let x2 = x1 + self.time_scale.width(wink);
            let mut back = calendar.day_color(wink).cloned();
            if let Some(day_of_week_color) = calendar.day_of_week_color(wink.to_day_of_week()) {
                back = Some(day_of_week_color.clone());
            }
            if back.is_none() && self.is_zero_on_day(wink) {
                back = Some(self.timeline_style.closed_background_color());
            }
            match back {
                None => {
                    if let Some((color, x1, x2)) = pending.take() {
                        draw(&color, x1, x2);
                    }
                }
                Some(back) => {
                    if pending.as_ref().is_some_and(|(color, ..)| *color != back) {
                        let (color, x1, x2) = pending.take().expect("pending checked");
                        draw(&color, x1, x2);
                    }
                    match &mut pending {
                        None => pending = Some((back, x1, x2)),
                        Some((_, _, pending_x2)) => *pending_x2 = x2,
                    }
                }
            }
        }
        if let Some((color, x1, x2)) = pending {
            draw(&color, x1, x2);
        }
    }

    /// The longest format that fits between `start` and `end`, centred; the first always when
    /// `hide_if_too_big` is false (`printCentered`).
    fn print_centered(
        &self,
        ug: &UGraphic,
        hide_if_too_big: bool,
        (start, end): (f64, f64),
        when: TimePoint,
        font: &FontConfiguration,
        formats: &[TimePointFormat],
    ) {
        let available = end - start;
        for (i, format) in formats.iter().enumerate().rev() {
            let text = Self::text_block(&format.format(when, self.language()), font);
            let width = text.calculate_dimension(ug.string_bounder()).width;
            if (i == 0 && !hide_if_too_big) || width <= available {
                let diff = f64::max(0.0, available - width);
                text.draw_u(&ug.translated(start + diff / 2.0, 0.0));
                return;
            }
        }
    }

    fn print_vertical_separators_base(&self, ug: &UGraphic, total_height_without_footer: f64) {
        let ug = self.timeline_style.apply_vertical_separator_style(ug);
        let top = self.full_header_height(ug.string_bounder());
        for day in self.days(true) {
            if self.diagram.model.calendar.has_separator_before(day) {
                Self::draw_vline(&ug, self.position(day), top, total_height_without_footer);
            }
        }
    }

    pub(super) fn draw_time_header(&self, ug: &UGraphic, total_height_without_footer: f64) {
        self.draw_colors_background(ug, total_height_without_footer);
        match self.kind {
            HeaderKind::Simple => {
                self.draw_small_vlines_day(ug, total_height_without_footer);
                self.print_vertical_separators_base(ug, total_height_without_footer);
                self.draw_simple_day_counter(ug);
            }
            HeaderKind::Daily => self.draw_daily_header(ug, total_height_without_footer),
            HeaderKind::Weekly => self.draw_weekly_header(ug, total_height_without_footer),
            HeaderKind::Monthly => {
                let style = &self.timeline_style;
                let h1 = style.font_size_year() + 2.0;
                let h2 = h1 + style.font_size_month() + 2.0;
                self.draw_years(ug, false);
                self.draw_months_monthly(&ug.translated(0.0, h1));
                self.print_vertical_separators_base(ug, total_height_without_footer);
                self.print_named_days(ug);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h1);
                self.draw_hline(ug, h2);
            }
            HeaderKind::Quarterly => {
                let style = &self.timeline_style;
                let h1 = style.font_size_year();
                let h2 = style.font_size_month();
                self.draw_years(ug, false);
                self.draw_quarters(&ug.translated(0.0, h1 + 2.0));
                self.print_vertical_separators_base(ug, total_height_without_footer);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h1 + 2.0);
                self.draw_hline(ug, h1 + 2.0 + h2 + 2.0);
            }
            HeaderKind::Yearly => {
                self.draw_years(ug, true);
                self.print_vertical_separators_base(ug, total_height_without_footer);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, self.full_header_height(ug.string_bounder()));
            }
        }
    }

    pub(super) fn draw_time_footer(&self, ug: &UGraphic) {
        let style = &self.timeline_style;
        match self.kind {
            HeaderKind::Simple => self.draw_simple_day_counter(&ug.translated(0.0, 3.0)),
            HeaderKind::Daily => {
                let h = style.font_size_day() + 2.0;
                self.draw_texts_day_of_week(ug);
                self.draw_text_day_of_month(&ug.translated(0.0, h + 2.0));
                self.draw_months_daily(&ug.translated(0.0, 2.0 * h + 3.0));
            }
            HeaderKind::Weekly => {
                self.draw_hline(ug, 0.0);
                self.print_months_weekly(ug);
                self.draw_hline(ug, self.time_footer_height(ug.string_bounder()));
            }
            HeaderKind::Monthly => {
                let h1 = style.font_size_year();
                let h2 = style.font_size_month();
                self.draw_months_monthly(ug);
                self.draw_years(&ug.translated(0.0, h2 + 2.0), false);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h2 + 2.0);
                self.draw_hline(ug, h1 + 2.0 + h2 + 2.0);
            }
            HeaderKind::Quarterly => {
                let h1 = style.font_size_year();
                let h2 = style.font_size_month();
                self.draw_quarters(ug);
                self.draw_years(&ug.translated(0.0, h2 + 2.0), false);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h2 + 2.0);
                self.draw_hline(ug, h1 + 2.0 + h2 + 2.0);
            }
            HeaderKind::Yearly => {
                self.draw_years(ug, true);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, self.time_footer_height(ug.string_bounder()));
            }
        }
    }

    /// The names of the named days, under the header (`printNamedDays`).
    fn print_named_days(&self, ug: &UGraphic) {
        if !self.has_name_days() {
            return;
        }
        let font =
            self.font_configuration_of(SName::Month, false, self.timeline_style.open_font_color());
        let mut last: Option<&str> = None;
        for day in self.days(true) {
            let wink = TimePoint::of_start_of_day(day);
            let name = self.diagram.model.calendar.day_name(wink);
            if let Some(name) = name
                && Some(name) != last
            {
                let x1 = self.time_scale.position(wink);
                let position = self.time_header_height(ug.string_bounder());
                Self::text_block(name, &font).draw_u(&ug.translated(x1, position));
            }
            last = name;
        }
    }

    // Daily (`TimeHeaderDaily`).

    fn draw_daily_header(&self, ug: &UGraphic, total_height_without_footer: f64) {
        let style = &self.timeline_style;
        let h1 = style.font_size_month() + 2.0;
        let h2 = h1 + style.font_size_day() + 2.0;
        self.draw_months_daily(ug);
        self.draw_texts_day_of_week(&ug.translated(0.0, h1));
        self.draw_text_day_of_month(&ug.translated(0.0, h2));
        self.print_vertical_separators_daily(ug, total_height_without_footer);
        self.print_named_days(ug);
        self.draw_hline(ug, self.full_header_height(ug.string_bounder()));
        self.draw_hline(ug, total_height_without_footer);
    }

    fn print_vertical_separators_daily(&self, ug: &UGraphic, total_height_without_footer: f64) {
        let ug_vertical_separator = self.timeline_style.apply_vertical_separator_style(ug);
        let ug_line_color = ug.with_color(self.timeline_style.line_color());
        let top = self.full_header_height(ug.string_bounder());
        for day in self.days(true) {
            let wink = TimePoint::of_start_of_day(day);
            let ug = if self.diagram.model.calendar.has_separator_before(day)
                || self.time_scale.width(wink.decrement()) == 0.0
            {
                &ug_vertical_separator
            } else {
                &ug_line_color
            };
            Self::draw_vline(
                ug,
                self.time_scale.position(wink),
                top,
                total_height_without_footer,
            );
        }
        let end = self.position(self.max_day().plus_days(1));
        Self::draw_vline(&ug_line_color, end, top, total_height_without_footer);
    }

    fn is_hidden_day(&self, wink: TimePoint) -> bool {
        self.diagram.hide_closed
            && self
                .diagram
                .model
                .calendar
                .open_close
                .is_closed(wink.to_day())
    }

    fn day_font(&self, wink: TimePoint) -> FontConfiguration {
        let color = if self.is_zero_on_day(wink) {
            self.timeline_style.closed_font_color()
        } else {
            self.timeline_style.open_font_color()
        };
        self.font_configuration_of(SName::Day, false, color)
    }

    fn draw_day_texts(&self, ug: &UGraphic, format: TimePointFormat) {
        for day in self.days(true) {
            let wink = TimePoint::of_start_of_day(day);
            if self.is_hidden_day(wink) {
                continue;
            }
            let x1 = self.time_scale.position(wink);
            let x2 = self.time_scale.position(wink.increment());
            self.print_centered(ug, false, (x1, x2), wink, &self.day_font(wink), &[format]);
        }
    }

    fn draw_texts_day_of_week(&self, ug: &UGraphic) {
        self.draw_day_texts(ug, TimePointFormat::DayOfWeekShort);
    }

    fn draw_text_day_of_month(&self, ug: &UGraphic) {
        self.draw_day_texts(ug, TimePointFormat::DayOfMonth);
    }

    fn draw_months_daily(&self, ug: &UGraphic) {
        let font =
            self.font_configuration_of(SName::Month, true, self.timeline_style.open_font_color());
        let formats = [
            TimePointFormat::MonthShort,
            TimePointFormat::MonthLong,
            TimePointFormat::MonthYearLong,
        ];
        let mut last: Option<TimePoint> = None;
        let mut last_change_month = -1.0;
        for day in self.days(true) {
            let wink = TimePoint::of_start_of_day(day);
            if self.is_hidden_day(wink) {
                continue;
            }
            let x1 = self.time_scale.position(wink);
            if last.is_none_or(|last| wink.month_year() != last.month_year()) {
                if let Some(last) = last {
                    self.print_centered(ug, false, (last_change_month, x1), last, &font, &formats);
                }
                last_change_month = x1;
                last = Some(wink);
            }
        }
        let x1 = self.position(self.max_day().plus_days(1));
        if x1 > last_change_month
            && let Some(last) = last
        {
            self.print_centered(ug, false, (last_change_month, x1), last, &font, &formats);
        }
    }

    // Weekly (`TimeHeaderWeekly`).

    fn draw_weekly_header(&self, ug: &UGraphic, total_height_without_footer: f64) {
        let h1 = self.timeline_style.font_size_month() + 4.0;
        self.print_days_of_month_weekly(ug, h1);
        self.print_vertical_separators_weekly(ug, total_height_without_footer, h1);
        self.print_months_weekly(ug);
        self.print_named_days(ug);
        self.draw_hline(ug, 0.0);
        self.draw_hline(ug, h1);
        self.draw_hline(ug, self.full_header_height(ug.string_bounder()));
    }

    fn print_months_weekly(&self, ug: &UGraphic) {
        let h1 = self.timeline_style.font_size_month() + 4.0;
        let font =
            self.font_configuration_of(SName::Month, true, self.timeline_style.open_font_color());
        let formats = [TimePointFormat::MonthShort, TimePointFormat::MonthYearShort];
        let ug_line = ug.with_color(self.timeline_style.line_color());
        let mut last: Option<TimePoint> = None;
        let mut last_change_month = -1.0;
        for day in self.days(true) {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.time_scale.position(wink);
            if last.is_none_or(|last| wink.month_year() != last.month_year()) {
                Self::draw_vline(&ug_line, x1, 0.0, h1);
                if let Some(last) = last {
                    self.print_centered(ug, false, (last_change_month, x1), last, &font, &formats);
                }
                last_change_month = x1;
                last = Some(wink);
            }
        }
        let end = self.position(self.max_day().plus_days(1));
        Self::draw_vline(&ug_line, end, 0.0, h1);
        if let Some(last) = last
            && end > last_change_month
        {
            self.print_centered(ug, false, (last_change_month, end), last, &font, &formats);
        }
    }

    fn print_vertical_separators_weekly(
        &self,
        ug: &UGraphic,
        total_height_without_footer: f64,
        h1: f64,
    ) {
        let ug_line = ug.with_color(self.timeline_style.line_color());
        let first_day_of_week = self.diagram.week_number_strategy.0;
        for day in self.days(true) {
            if day.day_of_week() == first_day_of_week {
                Self::draw_vline(
                    &ug_line,
                    self.position(day),
                    h1,
                    total_height_without_footer,
                );
            }
        }
        let end = self.position(self.max_day().plus_days(1));
        Self::draw_vline(&ug_line, end, h1, total_height_without_footer);
        self.print_vertical_separators_base(ug, total_height_without_footer);
    }

    fn print_days_of_month_weekly(&self, ug: &UGraphic, h1: f64) {
        let font =
            self.font_configuration_of(SName::Day, false, self.timeline_style.open_font_color());
        let (first_day_of_week, minimal_days) = self.diagram.week_number_strategy;
        let mut counter = self.diagram.week_starting_number;
        let last = self.max_day().plus_days(-1);
        let mut day = self.min_day();
        while day < last {
            if day.day_of_week() == first_day_of_week {
                let num = match self.diagram.weekly_header_strategy {
                    Some(WeeklyHeaderStrategy::FromN) => {
                        counter += 1;
                        (counter - 1).to_string()
                    }
                    Some(WeeklyHeaderStrategy::DayOfMonth) => day.day_of_month().to_string(),
                    _ => day
                        .week_of_year(first_day_of_week, minimal_days)
                        .to_string(),
                };
                Self::text_block(&num, &font).draw_u(&ug.translated(self.position(day) + 5.0, h1));
            }
            day = day.plus_days(1);
        }
    }

    // Monthly, quarterly and yearly (`TimeHeaderMonthly`, `TimeHeaderQuarterly`, `TimeHeaderYearly`).

    /// The years, each between vertical lines; the yearly scale reaches the last day and hides a year that
    /// does not fit.
    fn draw_years(&self, ug: &UGraphic, yearly: bool) {
        let h1 = self.timeline_style.font_size_year();
        let name = if yearly { SName::Year } else { SName::Month };
        let font = self.font_configuration_of(name, true, self.timeline_style.open_font_color());
        self.draw_periods(
            ug,
            h1 + 2.0,
            yearly,
            TimePoint::year,
            |header, ug, start, end, when| {
                header.print_centered(
                    ug,
                    yearly,
                    (start, end),
                    when,
                    &font,
                    &[TimePointFormat::Year],
                );
            },
        );
    }

    fn draw_months_monthly(&self, ug: &UGraphic) {
        let h2 = self.timeline_style.font_size_month();
        let font =
            self.font_configuration_of(SName::Day, false, self.timeline_style.open_font_color());
        self.draw_periods(
            ug,
            h2 + 2.0,
            false,
            |wink| wink.year() * 12 + wink.month_value(),
            |header, ug, start, end, when| {
                header.print_centered(
                    ug,
                    false,
                    (start, end),
                    when,
                    &font,
                    &[TimePointFormat::MonthShort, TimePointFormat::MonthLong],
                );
            },
        );
    }

    fn draw_quarters(&self, ug: &UGraphic) {
        let h2 = self.timeline_style.font_size_month();
        let font =
            self.font_configuration_of(SName::Day, false, self.timeline_style.open_font_color());
        self.draw_periods(
            ug,
            h2 + 2.0,
            false,
            |wink| (wink.month_value() + 2) / 3,
            |header, ug, start, end, when| {
                header.print_centered(
                    ug,
                    false,
                    (start, end),
                    when,
                    &font,
                    &[TimePointFormat::Quarter],
                );
            },
        );
    }

    /// A vertical line where `period_of` changes, the period printed between, and a line at the end.
    fn draw_periods(
        &self,
        ug: &UGraphic,
        height: f64,
        inclusive: bool,
        period_of: impl Fn(TimePoint) -> i64,
        print: impl Fn(&Self, &UGraphic, f64, f64, TimePoint),
    ) {
        let ug_line = ug.with_color(self.timeline_style.line_color());
        let mut last: Option<TimePoint> = None;
        let mut last_change = -1.0;
        for day in self.days(inclusive) {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.time_scale.position(wink);
            if last.is_none_or(|last| period_of(wink) != period_of(last)) {
                Self::draw_vline(&ug_line, x1, 0.0, height);
                if let Some(last) = last {
                    print(self, ug, last_change, x1, last);
                }
                last_change = x1;
                last = Some(wink);
            }
        }
        let end = self.position(self.max_day().plus_days(1));
        if end > last_change
            && let Some(last) = last
        {
            print(self, ug, last_change, end, last);
        }
        Self::draw_vline(&ug_line, end, 0.0, height);
    }

    // Simple (`TimeHeaderSimple`).

    /// How many days a step of the counter covers: enough for 16 pixels (`initDelta`).
    fn simple_delta(&self) -> i64 {
        let mut delta = self.simple_delta.borrow_mut();
        if *delta == 0 {
            if self.diagram.print_scale == PrintScale::Daily {
                let mut day = self.min_day();
                let x1 = self.position(day);
                loop {
                    *delta += 1;
                    day = day.plus_days(1);
                    if self.position(day) >= x1 + 16.0 {
                        break;
                    }
                }
            } else {
                *delta = 1;
            }
        }
        *delta
    }

    fn simple_increment(&self, day: LocalDate) -> LocalDate {
        let step = if self.diagram.print_scale == PrintScale::Weekly {
            7
        } else {
            1
        };
        day.plus_days(self.simple_delta() * step)
    }

    fn simple_days(&self) -> Vec<LocalDate> {
        let last = self.max_day().plus_days(1);
        let mut days = Vec::new();
        let mut day = self.min_day();
        while day <= last {
            days.push(day);
            day = self.simple_increment(day);
        }
        days
    }

    fn draw_small_vlines_day(&self, ug: &UGraphic, total_height_without_footer: f64) {
        let ug = ug
            .with_color(self.timeline_style.line_color())
            .translated(0.0, 6.0);
        for day in self.simple_days() {
            ug.translated(self.position(day), 0.0).draw(&UShape::Line {
                dx: 0.0,
                dy: total_height_without_footer + 2.0,
            });
        }
    }

    fn draw_simple_day_counter(&self, ug: &UGraphic) {
        let font = Self::font_configuration(
            &self.timeline_style.font(SName::Day),
            false,
            self.timeline_style.open_font_color(),
        );
        let weekly = self.diagram.print_scale == PrintScale::Weekly;
        let end_of_chart = TimePoint::of_start_of_day(self.max_day().plus_days(1));
        for day in self.simple_days() {
            let wink = TimePoint::of_start_of_day(day);
            let value = if weekly {
                wink.absolute_day_num() / 7 + 1
            } else {
                wink.absolute_day_num() + 1
            };
            let num = Self::text_block(&value.to_string(), &font);
            let x1 = self.time_scale.position(wink);
            let x2 = if weekly {
                let last_day = wink.add_days(6);
                self.time_scale.position(last_day) + self.time_scale.width(last_day)
            } else {
                self.position(self.simple_increment(day))
            };
            let width = num.calculate_dimension(ug.string_bounder()).width;
            let delta = (x2 - x1) - width;
            if wink < end_of_chart {
                num.draw_u(&ug.translated(x1 + delta / 2.0, 0.0));
            }
        }
    }
}

impl GanttDiagram {
    /// The style of `ganttDiagram` or one of its elements.
    pub(super) fn style(&self, names: &[SName]) -> Style {
        let mut signature = vec![SName::Root, SName::Element, SName::GanttDiagram];
        signature.extend_from_slice(names);
        StyleSignature::of(&signature).get_merged_style(&self.titled.skin.current_style_builder())
    }
}

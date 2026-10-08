//! How the chart is laid out and drawn: one row per task, arrows between them and the load of each person
//! below (PlantUML's `GanttDiagramMainBlock`, `GanttLayout`, `TaskDrawRegistryData`, the task draws of
//! `gantt.draw`, `GanttArrow`, `GArrows` and `ResourceDrawNumbers`).

use std::collections::BTreeSet;

use super::header::{TimeHeader, TimeScale};
use super::model::{
    CenterBorderColor, GanttConstraint, TaskAttribute, TaskId, TaskInstant, TaskKind,
};
use super::table::GanttTaskTable;
use super::time::TimePoint;
use super::{GanttDiagram, TimeBounds};
use crate::color::HColor;
use crate::creole::{CreoleMode, CreoleParser, Display, SheetBlock1, SheetBlock2};
use crate::decoration::LinkType;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UPolygon, URectangle, USegment, UShape};
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::local_date::LocalDate;
use crate::real::Real;
use crate::stereo::Stereotype;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};
use crate::svek::image::Opale;

/// Where an arrow leaves or reaches a task (`GSide`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GSide {
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl GSide {
    fn is_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }

    fn is_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopRight)
    }

    fn reverse_bottom_top(self) -> Self {
        match self {
            Self::BottomLeft => Self::TopLeft,
            Self::BottomRight => Self::TopRight,
            _ => unreachable!("only bottom sides turn up"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GArrowType {
    Incoming,
    Outgoing,
}

/// The box around what a task draws, to find notes in the way (`FingerPrint`).
struct FingerPrint {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl FingerPrint {
    /// How far `other` must move down to clear this, 0 when they do not meet (`overlap`).
    fn overlap(&self, other: &Self) -> f64 {
        if self.x >= other.x + other.width
            || other.x >= self.x + self.width
            || self.y >= other.y + other.height
            || other.y >= self.y + self.height
        {
            return 0.0;
        }
        self.y + self.height - other.y
    }
}

/// What a row draws.
enum DrawKind {
    /// A bar with its pauses (`TaskDrawRegular`).
    Regular {
        end: TimePoint,
        odd_start: bool,
        odd_end: bool,
        paused: BTreeSet<LocalDate>,
        right_arrow: bool,
    },
    /// A milestone (`TaskDrawDiamond`).
    Diamond,
    /// The bar over a group's tasks (`TaskDrawGroup`).
    Group { end: TimePoint },
    /// A line across the chart (`TaskDrawSeparator`).
    Separator {
        name: Option<String>,
        min_day: LocalDate,
        max_day: LocalDate,
    },
}

/// One row of the chart (`TaskDraw` and its implementations).
struct TaskDraw {
    task: TaskId,
    kind: DrawKind,
    y: Real,
    pretty_display: String,
    start: TimePoint,
    colors: Option<CenterBorderColor>,
    completion: i32,
    url: Option<Url>,
    note: Option<(Display, Option<Stereotype>)>,
}

/// What drawing a row needs to know.
struct DrawContext<'a> {
    diagram: &'a GanttDiagram,
    scale: &'a TimeScale,
}

impl DrawContext<'_> {
    fn position(&self, instant: TimePoint) -> f64 {
        self.scale.position(instant)
    }

    fn width(&self, instant: TimePoint) -> f64 {
        self.scale.width(instant)
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

fn hline(length: f64) -> UShape {
    UShape::Line {
        dx: length,
        dy: 0.0,
    }
}

fn path(points: &[(f64, f64)]) -> Vec<USegment> {
    points
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            if i == 0 {
                USegment::MoveTo(x, y)
            } else {
                USegment::LineTo(x, y)
            }
        })
        .collect()
}

impl TaskDraw {
    fn style(&self, ctx: &DrawContext) -> Style {
        match &self.kind {
            DrawKind::Regular { .. } => StyleSignature::of(&[
                SName::Root,
                SName::Element,
                SName::GanttDiagram,
                SName::Task,
            ])
            .get_merged_style_with(
                &ctx.diagram.titled.skin.current_style_builder(),
                ctx.diagram.model.tasks[self.task].stereotype.as_ref(),
            ),
            DrawKind::Diamond => ctx.diagram.style(&[SName::Milestone]),
            DrawKind::Group { .. } => ctx.diagram.style(&[SName::Task]),
            DrawKind::Separator { .. } => ctx.diagram.style(&[SName::Separator]),
        }
    }

    fn font_configuration(&self, ctx: &DrawContext) -> FontConfiguration {
        self.style(ctx).font_configuration()
    }

    fn title(&self, ctx: &DrawContext) -> Option<SheetBlock2> {
        let text = match &self.kind {
            DrawKind::Separator { name, .. } => name.as_deref()?,
            _ => &self.pretty_display,
        };
        Some(text_block(text, &self.font_configuration(ctx)))
    }

    fn title_dimension(
        &self,
        ctx: &DrawContext,
        string_bounder: &dyn StringBounder,
    ) -> XDimension2D {
        self.title(ctx)
            .map_or(XDimension2D::new(0.0, 0.0), |title| {
                title.calculate_dimension(string_bounder)
            })
    }

    fn line_color(&self, ctx: &DrawContext) -> HColor {
        let unstarted = ctx.diagram.style(&[SName::Task, SName::Unstarted]);
        HColor::unlinear(
            &unstarted.value(PName::LineColor).as_color(),
            &self.style(ctx).value(PName::LineColor).as_color(),
            self.completion,
        )
    }

    fn background_color(&self, ctx: &DrawContext) -> HColor {
        let unstarted = ctx.diagram.style(&[SName::Task, SName::Unstarted]);
        HColor::unlinear(
            &unstarted.value(PName::BackGroundColor).as_color(),
            &self.style(ctx).value(PName::BackGroundColor).as_color(),
            self.completion,
        )
    }

    /// The task's colours, or the style's as far as it is completed.
    fn apply_colors(&self, ctx: &DrawContext, ug: &UGraphic) -> UGraphic {
        if let Some(colors) = &self.colors
            && let Some(center) = &colors.center
        {
            let ug = ug.with_backcolor(center.clone());
            return ug.with_color(colors.border.clone().unwrap_or_else(|| center.clone()));
        }
        ug.with_color(self.line_color(ctx))
            .with_backcolor(self.background_color(ctx))
    }

    /// The diamond's size: the font size, rounded down to even (`getDiamondHeight`).
    fn diamond_height(&self, ctx: &DrawContext) -> f64 {
        let mut result = self.font_configuration(ctx).font().size_2d() as i32;
        if result % 2 == 1 {
            result -= 1;
        }
        f64::from(result)
    }

    fn shape_height(&self, ctx: &DrawContext, string_bounder: &dyn StringBounder) -> f64 {
        let title = self.title_dimension(ctx, string_bounder);
        match &self.kind {
            DrawKind::Regular { .. } | DrawKind::Separator { .. } => {
                let padding = self.style(ctx).padding();
                padding.top + title.height + padding.bottom
            }
            DrawKind::Diamond => title.height.max(self.diamond_height(ctx)),
            DrawKind::Group { end } => {
                let pos1 = ctx.position(self.start) + 6.0;
                let pos2 = ctx.position(*end) + ctx.width(*end) - 6.0;
                if pos2 - pos1 > title.width {
                    title.height + 2.0
                } else {
                    title.height
                }
            }
        }
    }

    fn full_height_task(&self, ctx: &DrawContext, string_bounder: &dyn StringBounder) -> f64 {
        let margin = self.style(ctx).margin();
        margin.top + self.shape_height(ctx, string_bounder) + margin.bottom
    }

    fn opale_note(&self, ctx: &DrawContext) -> Option<Opale<'static>> {
        let (note, stereotype) = self.note.as_ref()?;
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::GanttDiagram,
            SName::Note,
        ])
        .get_merged_style_with(
            &ctx.diagram.titled.skin.current_style_builder(),
            stereotype.as_ref(),
        );
        let font = style.font_configuration();
        let alignment = style
            .value(PName::HorizontalAlignment)
            .as_horizontal_alignment()
            .unwrap_or(HorizontalAlignment::Left);
        let sheet = CreoleParser::with_mode(
            font.clone(),
            alignment,
            CreoleMode::Full,
            &ctx.diagram.titled.skin,
        )
        .create_display_sheet(note, &font);
        Some(Opale::new(
            style.value(PName::LineColor).as_color(),
            style.value(PName::BackGroundColor).as_color(),
            Box::new(SheetBlock1::new(sheet, style.padding())),
            style.stroke(),
            0.0,
        ))
    }

    fn y_note_position(&self, ctx: &DrawContext, string_bounder: &dyn StringBounder) -> f64 {
        self.full_height_task(ctx, string_bounder)
    }

    fn height_max(&self, ctx: &DrawContext, string_bounder: &dyn StringBounder) -> f64 {
        match (&self.kind, self.opale_note(ctx)) {
            (DrawKind::Regular { .. } | DrawKind::Diamond, Some(opale)) => {
                self.y_note_position(ctx, string_bounder)
                    + opale.calculate_dimension(string_bounder).height
            }
            _ => self.full_height_task(ctx, string_bounder),
        }
    }

    fn finger_print(
        &self,
        ctx: &DrawContext,
        y: f64,
        string_bounder: &dyn StringBounder,
    ) -> FingerPrint {
        let h = self.full_height_task(ctx, string_bounder);
        let start_pos = ctx.position(self.start);
        match &self.kind {
            DrawKind::Regular { end, .. } => FingerPrint {
                x: start_pos,
                y,
                width: ctx.position(*end) - start_pos,
                height: h,
            },
            // PlantUML passes where the milestone ends as its size.
            DrawKind::Diamond => FingerPrint {
                x: start_pos,
                y,
                width: start_pos + h,
                height: y + h,
            },
            DrawKind::Group { end } => FingerPrint {
                x: start_pos,
                y,
                width: ctx.position(*end) + ctx.width(*end) - start_pos,
                height: h,
            },
            DrawKind::Separator { max_day, .. } => FingerPrint {
                x: 0.0,
                y,
                width: ctx.position(TimePoint::of_start_of_day(max_day.plus_days(1))),
                height: y + h,
            },
        }
    }

    fn finger_print_note(
        &self,
        ctx: &DrawContext,
        y: f64,
        string_bounder: &dyn StringBounder,
    ) -> Option<FingerPrint> {
        if !matches!(self.kind, DrawKind::Regular { .. }) {
            return None;
        }
        let dim = self.opale_note(ctx)?.calculate_dimension(string_bounder);
        Some(FingerPrint {
            x: ctx.position(self.start),
            y: y + self.y_note_position(ctx, string_bounder),
            width: dim.width,
            height: dim.height,
        })
    }

    /// How far the title or note reaches past the bars' column.
    fn label_overflow(
        &self,
        ctx: &DrawContext,
        string_bounder: &dyn StringBounder,
        bars_width: f64,
    ) -> f64 {
        let title = self.title_dimension(ctx, string_bounder);
        let note_overflow = |overflow: f64| {
            self.opale_note(ctx).map_or(overflow, |opale| {
                let note_end =
                    ctx.position(self.start) + opale.calculate_dimension(string_bounder).width;
                overflow.max(note_end - bars_width)
            })
        };
        match &self.kind {
            DrawKind::Regular {
                end, right_arrow, ..
            } => {
                let pos1 = ctx.position(self.start) + 6.0;
                let pos2 = ctx.position(*end) - 6.0;
                let mut overflow = 0.0;
                if pos2 - pos1 <= title.width {
                    let label_end = out_position(pos2, *right_arrow) + title.width;
                    if label_end > bars_width {
                        overflow = label_end - bars_width;
                    }
                }
                note_overflow(overflow)
            }
            DrawKind::Diamond => {
                let padding = self.style(ctx).padding();
                let x2 = ctx.position(self.start) + ctx.width(self.start);
                let delta = x2 - ctx.position(self.start) - self.diamond_height(ctx);
                let label_end = x2 - delta / 2.0 + padding.left + title.width;
                let overflow = if label_end > bars_width {
                    label_end - bars_width
                } else {
                    0.0
                };
                note_overflow(overflow)
            }
            DrawKind::Group { end } => {
                let pos1 = ctx.position(self.start) + 6.0;
                let pos2 = ctx.position(*end) + ctx.width(*end) - 6.0;
                if pos2 - pos1 > title.width {
                    return 0.0;
                }
                let label_end = pos2 + 6.0 + title.width;
                if label_end > bars_width {
                    label_end - bars_width
                } else {
                    0.0
                }
            }
            DrawKind::Separator { .. } => 0.0,
        }
    }

    /// Where an arrow leaves or reaches the row (`getX`).
    fn x(&self, ctx: &DrawContext, side: GSide, arrow_type: GArrowType) -> f64 {
        let start = ctx.position(self.start);
        let start_end = start + ctx.width(self.start);
        match &self.kind {
            DrawKind::Regular { end, .. } => {
                let end_pos = ctx.position(*end);
                let mut x = match side {
                    GSide::Left => start,
                    GSide::Right => end_pos,
                    GSide::TopLeft | GSide::BottomLeft => {
                        let x = start + 8.0;
                        if x > end_pos {
                            f64::midpoint(start, start_end)
                        } else {
                            x
                        }
                    }
                    GSide::TopRight | GSide::BottomRight => end_pos - 8.0,
                };
                if arrow_type == GArrowType::Outgoing {
                    let margin = self.style(ctx).margin();
                    match side {
                        GSide::Left => x += margin.left,
                        GSide::Right => x -= margin.left,
                        _ => {}
                    }
                }
                x
            }
            DrawKind::Diamond => {
                let mut x = match side {
                    GSide::Left => start,
                    GSide::Right => start_end,
                    _ => f64::midpoint(start, start_end),
                };
                if arrow_type == GArrowType::Outgoing {
                    let width = self.diamond_height(ctx);
                    match side {
                        GSide::Left => x += width / 2.0,
                        GSide::Right => x -= width / 2.0,
                        _ => {}
                    }
                }
                x
            }
            DrawKind::Group { end } => {
                let end_end = ctx.position(*end) + ctx.width(*end);
                match side {
                    GSide::Left => start,
                    GSide::Right => end_end,
                    GSide::TopLeft | GSide::BottomLeft => f64::midpoint(start, start_end),
                    GSide::TopRight | GSide::BottomRight => {
                        f64::midpoint(ctx.position(*end), end_end)
                    }
                }
            }
            DrawKind::Separator { .. } => unreachable!("no arrow reaches a separator"),
        }
    }

    /// The height of a side of the row, for arrows (`getY(stringBounder, side)`).
    fn y_side(
        &self,
        ctx: &DrawContext,
        y: f64,
        string_bounder: &dyn StringBounder,
        side: GSide,
    ) -> f64 {
        let margin = self.style(ctx).margin();
        let y1 = margin.top + y;
        let y2 = y1 + self.shape_height(ctx, string_bounder);
        if matches!(self.kind, DrawKind::Diamond)
            && ctx.diagram.model.tasks[self.task].display_string.is_none()
        {
            return y1 + self.diamond_height(ctx) / 2.0;
        }
        if side.is_top() {
            y1
        } else if side.is_bottom() {
            y2
        } else {
            f64::midpoint(y1, y2)
        }
    }

    fn draw_note(&self, ctx: &DrawContext, ug: &UGraphic) {
        if let Some(opale) = self.opale_note(ctx) {
            opale.draw_u(ug);
        }
    }

    fn draw_u(&self, ctx: &DrawContext, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        match &self.kind {
            DrawKind::Regular {
                end,
                odd_start,
                odd_end,
                paused,
                ..
            } => {
                let start_pos = ctx.position(self.start);
                self.draw_note(
                    ctx,
                    &ug.translated(start_pos, self.y_note_position(ctx, string_bounder)),
                );
                self.draw_regular_shape(ctx, ug, *end, *odd_start, *odd_end, paused);
            }
            DrawKind::Diamond => self.draw_diamond(ctx, ug),
            DrawKind::Group { end } => {
                self.draw_group_shape(ctx, &self.apply_colors(ctx, ug), *end);
            }
            DrawKind::Separator {
                min_day, max_day, ..
            } => {
                self.draw_separator(ctx, ug, *min_day, *max_day);
            }
        }
    }

    fn with_url(&self, ug: &UGraphic, draw: impl FnOnce()) {
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        draw();
        if self.url.is_some() {
            ug.close_url();
        }
    }

    fn draw_regular_shape(
        &self,
        ctx: &DrawContext,
        ug: &UGraphic,
        end: TimePoint,
        odd_start: bool,
        odd_end: bool,
        paused: &BTreeSet<LocalDate>,
    ) {
        let ug = self.apply_colors(ctx, ug);
        let style = self.style(ctx);
        let margin = style.margin();
        let start_pos = ctx.position(self.start) + margin.left;
        let end_pos = ctx.position(end) - margin.right;
        self.with_url(&ug, || {
            let ug = ug.translated(0.0, margin.top);
            let round = style.value(PName::RoundCorner).as_double();
            let off: Vec<(f64, f64)> = paused
                .iter()
                .map(|day| {
                    let pause = TimePoint::of_start_of_day(*day);
                    let x1 = ctx.position(pause);
                    (x1, ctx.position(pause) + ctx.width(pause))
                })
                .collect();
            let back_undone = ctx
                .diagram
                .style(&[SName::Undone])
                .value(PName::BackGroundColor)
                .as_color();
            RectangleTask::new(start_pos, end_pos, round, self.completion, &off).draw(
                &ug,
                self.shape_height(ctx, ug.string_bounder()),
                &back_undone,
                odd_start,
                odd_end,
            );
        });
    }

    fn draw_diamond(&self, ctx: &DrawContext, ug: &UGraphic) {
        let x1 = ctx.position(self.start);
        let string_bounder = ug.string_bounder();
        self.draw_note(
            ctx,
            &ug.translated(x1, self.y_note_position(ctx, string_bounder)),
        );
        self.with_url(ug, || {
            let margin = self.style(ctx).margin();
            let ug = ug.translated(x1, margin.top);
            match &ctx.diagram.model.tasks[self.task].display_string {
                None => {
                    let x2 = x1 + ctx.width(self.start);
                    let h = self.diamond_height(ctx);
                    let delta = x2 - x1 - h;
                    self.apply_colors(ctx, &ug.translated(delta / 2.0, 0.0))
                        .draw(&UShape::polygon(vec![
                            (h / 2.0, 0.0),
                            (h, h / 2.0),
                            (h / 2.0, h),
                            (0.0, h / 2.0),
                        ]));
                }
                Some(display) => {
                    text_block(display, &self.font_configuration(ctx)).draw_u(&ug);
                }
            }
        });
    }

    fn draw_group_shape(&self, ctx: &DrawContext, ug: &UGraphic, end: TimePoint) {
        const HEIGHT: f64 = 10.0;
        const THICK: f64 = 2.0;
        const DX: f64 = 6.0;
        let margin = self.style(ctx).margin();
        let start_pos = ctx.position(self.start) + margin.left;
        let end_pos = ctx.position(end) + ctx.width(end) - margin.right;
        self.with_url(ug, || {
            let ug = ug
                .translated(
                    0.0,
                    self.full_height_task(ctx, ug.string_bounder()) - HEIGHT,
                )
                .with_color(HColor::BLACK)
                .with_backcolor(HColor::BLACK);
            let y1 = (HEIGHT - THICK) / 2.0;
            let y2 = HEIGHT - (HEIGHT - THICK) / 2.0;
            ug.draw(&UShape::path(path(&[
                (start_pos, 0.0),
                (start_pos + DX, y1),
                (end_pos - DX, y1),
                (end_pos, 0.0),
                (end_pos, HEIGHT),
                (end_pos - DX, y2),
                (start_pos + DX, y2),
                (start_pos, HEIGHT),
                (start_pos, 0.0),
            ])));
        });
    }

    fn draw_separator(
        &self,
        ctx: &DrawContext,
        ug: &UGraphic,
        min_day: LocalDate,
        max_day: LocalDate,
    ) {
        let string_bounder = ug.string_bounder();
        let style = self.style(ctx);
        let title = self.title_dimension(ctx, string_bounder);
        let start = ctx.position(TimePoint::of_start_of_day(min_day));
        let end = ctx.position(TimePoint::of_start_of_day(max_day.plus_days(1)));
        let padding = style.padding();
        let margin = style.margin();
        let ug = ug.translated(0.0, margin.top);
        let back_color = style.value(PName::BackGroundColor).as_color();
        if !back_color.is_transparent() {
            let height = padding.top + title.height + padding.bottom;
            if height > 0.0 {
                ug.with_backcolor(back_color)
                    .draw(&UShape::Rectangle(URectangle::new(end - start, height)));
            }
        }
        let ug = ug
            .with_color(style.value(PName::LineColor).as_color())
            .translated(0.0, padding.top + title.height / 2.0);
        if title.width == 0.0 {
            ug.draw(&hline(end - start));
        } else {
            if padding.left > 1.0 {
                ug.draw(&hline(padding.left));
            }
            let x1 = padding.left + margin.left + title.width + margin.right;
            let x2 = end - 1.0;
            ug.translated(x1, 0.0).draw(&hline(x2 - x1));
        }
    }

    /// The title, inside the bar when it fits, else after it (`drawTitle` with the legacy strategy).
    fn draw_title(&self, ctx: &DrawContext, ug: &UGraphic) {
        let Some(title) = self.title(ctx) else {
            return;
        };
        let string_bounder = ug.string_bounder();
        let dim = title.calculate_dimension(string_bounder);
        let style = self.style(ctx);
        let margin = style.margin();
        let padding = style.padding();
        match &self.kind {
            DrawKind::Regular {
                end, right_arrow, ..
            } => {
                let ug = ug.translated(0.0, margin.top + padding.top);
                let pos1 = ctx.position(self.start) + 6.0;
                let pos2 = ctx.position(*end) - 6.0;
                let pos = if pos2 - pos1 > dim.width {
                    pos1
                } else {
                    out_position(pos2, *right_arrow)
                };
                title.draw_u(&ug.translated(pos, 0.0));
            }
            DrawKind::Diamond => {
                let ug = ug.translated(0.0, margin.top);
                let h = (self.shape_height(ctx, string_bounder) - dim.height) / 2.0;
                let x1 = ctx.position(self.start);
                let x2 = x1 + ctx.width(self.start);
                let delta = x2 - x1 - self.diamond_height(ctx);
                title.draw_u(&ug.translated(x2 - delta / 2.0 + padding.left, h));
            }
            DrawKind::Group { end } => {
                let pos1 = ctx.position(self.start) + 6.0;
                let pos2 = ctx.position(*end) + ctx.width(*end) - 6.0;
                let (pos, y) = if pos2 - pos1 > dim.width {
                    (pos1 + (pos2 - pos1 - dim.width) / 2.0, 0.0)
                } else {
                    (
                        pos2 + 6.0,
                        self.full_height_task(ctx, string_bounder) - dim.height,
                    )
                };
                title.draw_u(&ug.translated(pos, y));
            }
            DrawKind::Separator { .. } => {
                title.draw_u(&ug.translated(margin.left + padding.left, margin.top + padding.top));
            }
        }
    }
}

/// Where a title goes when it does not fit in its bar: after it, and after an arrow coming in there.
fn out_position(pos2: f64, right_arrow: bool) -> f64 {
    if right_arrow { pos2 + 18.0 } else { pos2 + 8.0 }
}

/// A bar, cut by its pauses and filled as far as completed (`RectangleTask`).
struct RectangleTask {
    segments: Vec<(f64, f64)>,
    round: f64,
    completion: i32,
}

/// The parts of `start..end` between the pauses (`Segment.cutSegmentIfNeed`).
fn cut_segment_if_need(start: f64, end: f64, pauses: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut sorted = pauses.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut result = Vec::new();
    let mut pending_start = start;
    for (pause1, pause2) in sorted {
        if (pause1 - pending_start).abs() < 0.001 {
            pending_start = pause2;
            continue;
        }
        if pause1 < pending_start {
            continue;
        }
        if pause1 > end {
            if pending_start < end {
                result.push((pending_start, end));
            }
            return result;
        }
        if pause1 >= start && pause1 <= end && pause2 >= start && pause2 <= end {
            result.push((pending_start, pause1));
            pending_start = pause2;
        }
    }
    if pending_start < end {
        result.push((pending_start, end));
    }
    result
}

fn u_to_right(width: f64, height: f64, round: f64) -> UShape {
    if round == 0.0 {
        return UShape::path(path(&[
            (0.0, 0.0),
            (width, 0.0),
            (width, height),
            (0.0, height),
        ]));
    }
    let half = round / 2.0;
    UShape::path(vec![
        USegment::MoveTo(0.0, 0.0),
        USegment::LineTo(width - half, 0.0),
        USegment::arc_to((width, half), half, true),
        USegment::LineTo(width, height - half),
        USegment::arc_to((width - half, height), half, true),
        USegment::LineTo(0.0, height),
    ])
}

fn u_to_left(width: f64, height: f64, round: f64) -> UShape {
    if round == 0.0 {
        return UShape::path(path(&[
            (width, height),
            (0.0, height),
            (0.0, 0.0),
            (width, 0.0),
        ]));
    }
    let half = round / 2.0;
    UShape::path(vec![
        USegment::MoveTo(width, height),
        USegment::LineTo(half, height),
        USegment::arc_to((0.0, height - half), half, true),
        USegment::LineTo(0.0, half),
        USegment::arc_to((half, 0.0), half, true),
        USegment::LineTo(width, 0.0),
    ])
}

fn rectangle(width: f64, height: f64) -> UShape {
    UShape::Rectangle(URectangle::new(width, height))
}

impl RectangleTask {
    fn new(start: f64, end: f64, round: f64, completion: i32, paused: &[(f64, f64)]) -> Self {
        let segments = if start < end {
            cut_segment_if_need(start, end, paused)
        } else {
            vec![(start, start + 1.0)]
        };
        Self {
            segments,
            round,
            completion,
        }
    }

    fn completion_width(&self, width: f64) -> f64 {
        width * f64::from(self.completion) / 100.0
    }

    fn draw(
        &self,
        ug: &UGraphic,
        height: f64,
        document_background: &HColor,
        odd_start: bool,
        odd_end: bool,
    ) {
        if self.round == 0.0 {
            self.draw_without_round(ug, height, document_background, odd_start, odd_end);
            return;
        }
        if self.segments.len() != 1 {
            self.draw_with_round(ug, height, document_background);
            return;
        }
        let (pos1, pos2) = self.segments[0];
        let width = pos2 - pos1;
        let ug = ug.translated(pos1, 0.0);
        let partial = UShape::Rectangle(URectangle::new(width, height).rounded(self.round));
        if self.completion == 100 || self.completion == 0 {
            let ug = if self.completion == 0 {
                ug.with_backcolor(document_background.clone())
            } else {
                ug
            };
            if odd_start && !odd_end {
                ug.draw(&u_to_right(width, height, self.round));
            } else if !odd_start && odd_end {
                ug.draw(&u_to_left(width, height, self.round));
            } else {
                ug.draw(&partial);
            }
        } else {
            let x1 = self.completion_width(width);
            ug.with_color(HColor::NONE)
                .draw(&u_to_left(x1, height, self.round));
            ug.with_backcolor(document_background.clone())
                .with_color(HColor::NONE)
                .translated(x1, 0.0)
                .draw(&u_to_right(
                    width * f64::from(100 - self.completion) / 100.0,
                    height,
                    self.round,
                ));
            ug.with_backcolor(HColor::NONE).draw(&partial);
        }
    }

    fn draw_with_round(&self, ug: &UGraphic, height: f64, document_background: &HColor) {
        let (first1, first2) = self.segments[0];
        ug.translated(first1, 0.0)
            .draw(&u_to_left(first2 - first1, height, self.round));
        let count = self.segments.len();
        for i in 1..count - 1 {
            let (pos1, pos2) = self.segments[i];
            self.draw_partly(
                self.completion_width(pos2 - pos1),
                ug,
                (pos1, pos2),
                height,
                document_background,
                i,
                false,
                false,
            );
        }
        let (last1, last2) = self.segments[count - 1];
        ug.translated(last1, 0.0)
            .draw(&u_to_right(last2 - last1, height, self.round));
        self.draw_intermediate_dotted(ug, height);
    }

    fn draw_without_round(
        &self,
        ug: &UGraphic,
        height: f64,
        document_background: &HColor,
        odd_start: bool,
        odd_end: bool,
    ) {
        let sum_width: f64 = self.segments.iter().map(|(pos1, pos2)| pos2 - pos1).sum();
        let lim = if self.completion == 100 {
            sum_width
        } else {
            self.completion_width(sum_width)
        };
        if self.segments.len() == 1 && !odd_start && !odd_end {
            let completion = if self.completion == 100 { -1.0 } else { lim };
            Self::draw_full(
                completion,
                ug,
                self.segments[0],
                height,
                document_background,
            );
            return;
        }
        let count = self.segments.len();
        let mut current = 0.0;
        for (i, &(pos1, pos2)) in self.segments.iter().enumerate() {
            let next = current + (pos2 - pos1);
            let width_completion = if lim >= next {
                -1.0
            } else if current >= lim {
                0.0
            } else {
                lim - current
            };
            self.draw_partly(
                width_completion,
                ug,
                (pos1, pos2),
                height,
                document_background,
                i,
                !odd_start && i == 0,
                !odd_end && i == count - 1,
            );
            current = next;
        }
        self.draw_intermediate_dotted(ug, height);
    }

    fn draw_intermediate_dotted(&self, ug: &UGraphic, height: f64) {
        let ug = ug.with_stroke(UStroke {
            dash_visible: 2.0,
            dash_space: 3.0,
            thickness: 1.0,
        });
        for pair in self.segments.windows(2) {
            let v1 = pair[0].1 + 3.0;
            let v2 = pair[1].0 - 3.0;
            if v2 > v1 {
                let ug = ug.translated(v1, 0.0);
                ug.draw(&hline(v2 - v1));
                ug.translated(0.0, height).draw(&hline(v2 - v1));
            }
        }
    }

    fn draw_full(
        width_completion: f64,
        ug: &UGraphic,
        (pos1, pos2): (f64, f64),
        height: f64,
        document_background: &HColor,
    ) {
        let width = pos2 - pos1;
        let ug = ug.translated(pos1, 0.0);
        draw_background_rect(
            width_completion,
            &ug.with_color(HColor::NONE),
            document_background,
            width,
            height,
        );
        ug.with_backcolor(HColor::NONE)
            .draw(&rectangle(width, height));
    }

    #[expect(clippy::too_many_arguments, reason = "PlantUML's drawPartly")]
    fn draw_partly(
        &self,
        width_completion: f64,
        ug: &UGraphic,
        (pos1, pos2): (f64, f64),
        height: f64,
        document_background: &HColor,
        i: usize,
        with_start_vline: bool,
        with_end_vline: bool,
    ) {
        let length = pos2 - pos1;
        let mut width_back = length;
        if i != self.segments.len() - 1 {
            width_back += 1.0;
        }
        let ug = ug.translated(pos1, 0.0);
        if width_back > 0.0 {
            draw_background_rect(
                width_completion,
                &ug.with_color(HColor::NONE),
                document_background,
                width_back,
                height,
            );
        }
        let ug = ug.with_backcolor(HColor::NONE);
        if with_start_vline {
            ug.draw(&u_to_left(length, height, 0.0));
        } else if with_end_vline {
            ug.draw(&u_to_right(length, height, 0.0));
        } else {
            ug.draw(&hline(length));
            ug.translated(0.0, height).draw(&hline(length));
        }
    }
}

/// The completed part in the bar's colour, the rest in the background's (`drawBackgroundRect`).
fn draw_background_rect(
    width_completion: f64,
    ug: &UGraphic,
    document_background: &HColor,
    width: f64,
    height: f64,
) {
    if width_completion == -1.0 || width_completion == 0.0 {
        let ug = if width_completion == 0.0 {
            ug.with_backcolor(document_background.clone())
        } else {
            ug.clone()
        };
        ug.draw(&rectangle(width, height));
    } else {
        ug.draw(&rectangle(width_completion, height));
        ug.with_backcolor(document_background.clone())
            .translated(width_completion, 0.0)
            .draw(&rectangle(width - width_completion, height));
    }
}

/// The rows of the chart and the people's loads below them (`TaskDrawRegistryData`).
struct TaskDrawRegistry {
    origin: Real,
    /// By task; `None` for no row.
    draws: Vec<Option<TaskDraw>>,
    /// Where each person's loads go, by person.
    resource_ys: Vec<f64>,
    total_height_without_footer: f64,
}

impl TaskDrawRegistry {
    fn draw(&self, task: TaskId) -> Option<&TaskDraw> {
        self.draws.get(task).and_then(Option::as_ref)
    }

    /// The row a task is drawn on: its own, or the row of the task it shares a row with.
    fn y(&self, ctx: &DrawContext, task: TaskId) -> Real {
        match ctx.diagram.model.tasks[task].row {
            Some(row) if self.draw(row).is_some() => self.y(ctx, row),
            _ => self.draw(task).expect("a drawn task").y.clone(),
        }
    }

    fn has_true_row(&self, ctx: &DrawContext, task: TaskId) -> bool {
        ctx.diagram.model.tasks[task]
            .row
            .is_some_and(|row| self.draw(row).is_some())
    }

    /// `buildTaskAndResourceDraws`.
    fn build(
        ctx: &DrawContext,
        bounds: TimeBounds,
        header: &TimeHeader,
        string_bounder: &dyn StringBounder,
    ) -> Self {
        let diagram = ctx.diagram;
        let model = &diagram.model;
        let header_total_height = header.full_header_height(string_bounder);
        let origin = Real::origin();
        let mut registry = Self {
            origin: origin.clone(),
            draws: Vec::new(),
            resource_ys: Vec::new(),
            total_height_without_footer: 0.0,
        };
        let mut y = origin.add_fixed(header_total_height);
        for (task, data) in model.tasks.iter().enumerate() {
            let mut draw = TaskDraw {
                task,
                kind: DrawKind::Diamond,
                y: y.clone(),
                pretty_display: data.code.display.clone(),
                start: TimePoint::of_start_of_day(bounds.min_day),
                colors: None,
                completion: 100,
                url: None,
                note: None,
            };
            match &data.kind {
                TaskKind::Separator(separator) => {
                    draw.kind = DrawKind::Separator {
                        name: separator.comment.clone(),
                        min_day: bounds.min_day,
                        max_day: bounds.max_day,
                    };
                }
                TaskKind::Group(_) => {
                    draw.start = bounds.start_for_drawing(model, task);
                    draw.kind = DrawKind::Group {
                        end: bounds.end_for_drawing(model, task),
                    };
                }
                TaskKind::Impl(task_impl) => {
                    if !diagram.display.hide_resource_name {
                        draw.pretty_display =
                            pretty_display(&data.code.display, &task_impl.resources);
                    }
                    draw.start = bounds.start_for_drawing(model, task);
                    if !task_impl.diamond {
                        draw.kind = regular_kind(ctx, bounds, task, draw.start);
                    }
                    draw.colors = task_impl.colors();
                    draw.completion = task_impl.completion;
                    draw.url.clone_from(&task_impl.url);
                    draw.note.clone_from(&task_impl.note);
                }
            }
            if data.row.is_none() {
                y = y.add_at_least(draw.full_height_task(ctx, string_bounder));
            }
            registry.draws.push(Some(draw));
        }
        origin.compile_now();
        registry.resolve_note_overlaps(ctx, string_bounder);
        registry.build_resource_draws(ctx, string_bounder, header_total_height);
        registry
    }

    /// Rows move down below notes they would cover (`resolveNoteOverlaps`).
    fn resolve_note_overlaps(&self, ctx: &DrawContext, string_bounder: &dyn StringBounder) {
        let mut notes: Vec<TaskId> = Vec::new();
        for draw in self.draws.iter().flatten() {
            let y = self.y(ctx, draw.task);
            let task_print = draw.finger_print(ctx, y.current_value(), string_bounder);
            let has_note = draw
                .finger_print_note(ctx, y.current_value(), string_bounder)
                .is_some();
            if !self.has_true_row(ctx, draw.task) {
                for &note in &notes {
                    let note_draw = self.draw(note).expect("a drawn task");
                    let note_y = self.y(ctx, note);
                    let other_note = note_draw
                        .finger_print_note(ctx, note_y.current_value(), string_bounder)
                        .expect("a task with a note");
                    if other_note.overlap(&task_print) > 0.0 {
                        let bottom = note_y.add_at_least(note_draw.height_max(ctx, string_bounder));
                        self.y(ctx, draw.task).ensure_bigger_than(&bottom);
                        self.origin.compile_now();
                    }
                }
            }
            if has_note {
                notes.push(draw.task);
            }
        }
    }

    fn build_resource_draws(
        &mut self,
        ctx: &DrawContext,
        string_bounder: &dyn StringBounder,
        header_height: f64,
    ) {
        let mut yy = self
            .draws
            .iter()
            .flatten()
            .map(|draw| {
                self.y(ctx, draw.task).current_value() + draw.height_max(ctx, string_bounder)
            })
            .fold(0.0, f64::max);
        if yy == 0.0 {
            yy = header_height;
        } else if !ctx.diagram.display.hide_resource_footbox {
            for _ in &ctx.diagram.model.resources {
                self.resource_ys.push(yy);
                yy += 32.0;
            }
        }
        self.total_height_without_footer = yy;
    }
}

/// The task's name and its people, like `Task {Bob:50%} {Alice}` (`getPrettyDisplay`).
fn pretty_display(display: &str, resources: &[(String, i32)]) -> String {
    if resources.is_empty() {
        return display.to_owned();
    }
    let people: Vec<String> = resources
        .iter()
        .map(|(name, percentage)| {
            if *percentage == 100 {
                format!("{{{name}}}")
            } else {
                format!("{{{name}:{percentage}%}}")
            }
        })
        .collect();
    format!("{display} {}", people.join(" "))
}

/// A regular task's row, its pauses including the closed days of its calendar.
fn regular_kind(ctx: &DrawContext, bounds: TimeBounds, task: TaskId, start: TimePoint) -> DrawKind {
    let model = &ctx.diagram.model;
    let end = bounds.end_for_drawing(model, task);
    let (odd_start, odd_end) = if bounds.printed_interval {
        (
            TimePoint::of_start_of_day(bounds.min_day) == start,
            TimePoint::of_start_of_day(bounds.max_day.plus_days(1)) == end,
        )
    } else {
        (false, false)
    };
    let mut paused = model.all_paused(task);
    let default_plan = model.default_plan_for(task);
    let mut day = start;
    while day <= end {
        if default_plan.is_zero_on_day(day.to_day()) {
            paused.insert(day.to_day());
        }
        day = day.increment();
    }
    let right_arrow = model
        .constraints_for_task(task)
        .iter()
        .any(|constraint| constraint.is_there_right_arrow(task));
    DrawKind::Regular {
        end,
        odd_start,
        odd_end,
        paused,
        right_arrow,
    }
}

/// The arrow of a constraint, around the rows it links (`GanttArrow` and `GArrows`).
fn draw_arrow(
    ctx: &DrawContext,
    registry: &TaskDrawRegistry,
    constraint: &GanttConstraint,
    ug: &UGraphic,
) {
    let (Some(source_task), Some(dest_task)) = (constraint.source.task(), constraint.dest.task())
    else {
        return;
    };
    let (Some(start), Some(end)) = (registry.draw(source_task), registry.draw(dest_task)) else {
        return;
    };
    let model = &ctx.diagram.model;
    let (at_start, at_end) = arrow_sides(model, &constraint.source, &constraint.dest);
    let style = ctx
        .diagram
        .style(&[SName::Arrow])
        .eventually_override(PName::LineColor, constraint.specific_color.as_ref());
    let line_color = style.value(PName::LineColor).as_color();
    let stroke = arrow_stroke(constraint.link_type, &style);
    let ug = ug.with_stroke(stroke).with_color(line_color.clone());
    let string_bounder = ug.string_bounder();
    let source_y = registry.y(ctx, source_task).current_value();
    let dest_y = registry.y(ctx, dest_task).current_value();
    let mut x1 = start.x(ctx, at_start, GArrowType::Outgoing);
    let mut y1 = start.y_side(ctx, source_y, string_bounder, at_start);
    let x2 = end.x(ctx, at_end, GArrowType::Incoming);
    let y2 = end.y_side(ctx, dest_y, string_bounder, at_end);
    if at_start.is_bottom() && y2 < y1 {
        y1 = start.y_side(ctx, source_y, string_bounder, at_start.reverse_bottom_top());
    }
    let points: Vec<(f64, f64)> = if at_start.is_bottom() && at_end == GSide::Left {
        if x2 > x1 + 6.0 {
            vec![(x1, y1), (x1, y2), (x2, y2)]
        } else {
            x1 = start.x(ctx, GSide::Right, GArrowType::Outgoing);
            y1 = start.y_side(ctx, source_y, string_bounder, GSide::Right);
            let y1b = dest_y;
            vec![
                (x1, y1),
                (x1 + 6.0, y1),
                (x1 + 6.0, y1b),
                (x2 - 8.0, y1b),
                (x2 - 8.0, y2),
                (x2, y2),
            ]
        }
    } else if at_start == GSide::Right && at_end == GSide::Right {
        let xmax = x1.max(x2) + 8.0;
        vec![(x1, y1), (xmax, y1), (xmax, y2), (x2, y2)]
    } else if at_start == GSide::Left && at_end == GSide::Left {
        let xmin = x1.min(x2) - 8.0;
        vec![(x1, y1), (xmin, y1), (xmin, y2), (x2, y2)]
    } else {
        vec![(x1, y1), (x1, y2), (x2, y2)]
    };
    draw_g_arrows(&ug, &points, at_end, &line_color);
}

/// The stroke of an arrow: the style's, unless the arrow's own style changes it.
fn arrow_stroke(link_type: LinkType, style: &Style) -> UStroke {
    link_type.get_stroke3(Some(style.stroke()))
}

fn arrow_sides(
    model: &super::model::GanttModel,
    source: &TaskInstant,
    dest: &TaskInstant,
) -> (GSide, GSide) {
    match (source.attribute, dest.attribute) {
        (TaskAttribute::End, TaskAttribute::Start) => (
            if model.same_row(source, dest) {
                GSide::Right
            } else {
                GSide::BottomRight
            },
            GSide::Left,
        ),
        (TaskAttribute::End, TaskAttribute::End) => (GSide::Right, GSide::Right),
        (TaskAttribute::Start, TaskAttribute::Start) => (GSide::Left, GSide::Left),
        (TaskAttribute::Start, TaskAttribute::End) => (
            if model.same_row(source, dest) {
                GSide::Left
            } else {
                GSide::BottomLeft
            },
            GSide::Right,
        ),
    }
}

/// The line through the points, its end pulled into the head, and the head (`GArrows`).
fn draw_g_arrows(ug: &UGraphic, points: &[(f64, f64)], at_end: GSide, back_color: &HColor) {
    const DELTA2: f64 = 4.0;
    let (x, y) = *points.last().expect("an arrow has points");
    let patched_x = match at_end {
        GSide::Left => x - 3.0,
        GSide::Right => x + 3.0,
        _ => x,
    };
    let mut segments: Vec<USegment> = Vec::new();
    for (i, &(px, py)) in points[..points.len() - 1].iter().enumerate() {
        segments.push(if i == 0 {
            USegment::MoveTo(px, py)
        } else {
            USegment::LineTo(px, py)
        });
    }
    segments.push(USegment::LineTo(patched_x, y));
    ug.draw(&UShape::path(segments));
    let head = match at_end {
        GSide::Left => vec![
            (x - 4.0, y - DELTA2),
            (x, y),
            (x - 4.0, y + DELTA2),
            (x - 4.0, y - DELTA2),
        ],
        GSide::Right => vec![
            (x + 4.0, y - DELTA2),
            (x, y),
            (x + 4.0, y + DELTA2),
            (x + 4.0, y - DELTA2),
        ],
        _ => Vec::new(),
    };
    ug.with_stroke(UStroke::SIMPLE)
        .with_backcolor(back_color.clone())
        .draw(&UShape::Polygon(UPolygon::new(head)));
}

/// A person's name and their load per day, or per week on compressed scales (`ResourceDrawNumbers`).
fn draw_resource(
    ctx: &DrawContext,
    ug: &UGraphic,
    name: &str,
    min: TimePoint,
    max_end_of_day: TimePoint,
) {
    let font = |size: i32, color: HColor| FontConfiguration::new(UFont::serif(size), color, 8);
    let title = text_block(name, &font(13, HColor::BLACK));
    title.draw_u(ug);
    let title_height = title.calculate_dimension(ug.string_bounder()).height;
    ug.with_color(HColor::BLACK)
        .translated(0.0, title_height)
        .draw(&hline(ctx.position(max_end_of_day) - ctx.position(min)));
    let mut starting_position = -1.0;
    let mut total_load = 0;
    let mut active_days = 0;
    let mut is_red = false;
    let mut day = min;
    while day <= max_end_of_day {
        let is_breaking = ctx.scale.is_breaking(day);
        let load = ctx.diagram.model.load_for_resource_at(name, day);
        if load > 100 {
            is_red = true;
        }
        total_load += load;
        if load > 0 {
            active_days += 1;
        }
        if is_breaking {
            if total_load > 0 {
                let displayed_load =
                    (f64::from(total_load) / f64::from(active_days) + 0.5).floor() as i64;
                let color = if is_red { HColor::RED } else { HColor::BLACK };
                let value = text_block(&displayed_load.to_string(), &font(9, color));
                if starting_position == -1.0 {
                    starting_position = ctx.position(day);
                }
                let ending_position = ctx.position(day) + ctx.width(day);
                let start = f64::midpoint(starting_position, ending_position)
                    - value.calculate_dimension(ug.string_bounder()).width / 2.0;
                value.draw_u(&ug.translated(start, 16.0));
            }
            starting_position = -1.0;
            total_load = 0;
            active_days = 0;
            is_red = false;
        } else if starting_position == -1.0 {
            starting_position = ctx.position(day);
        }
        day = day.increment();
    }
}

/// The whole chart: the task table, then the time line with the rows over it (`GanttDiagramMainBlock`).
pub(super) struct GanttDiagramMainBlock<'a> {
    diagram: &'a GanttDiagram,
    bounds: TimeBounds,
    header: TimeHeader<'a>,
    registry: TaskDrawRegistry,
    table: GanttTaskTable,
    /// `GanttLayout`.
    timeline_width: f64,
    bars_width: f64,
    header_height: f64,
    footer_height: f64,
    total_height: f64,
}

impl<'a> GanttDiagramMainBlock<'a> {
    pub(super) fn new(diagram: &'a GanttDiagram, string_bounder: &dyn StringBounder) -> Self {
        let bounds = diagram.time_bounds();
        let header = TimeHeader::new(diagram, bounds);
        let ctx = DrawContext {
            diagram,
            scale: header.time_scale(),
        };
        let registry = TaskDrawRegistry::build(&ctx, bounds, &header, string_bounder);
        let timeline_width = ctx.position(TimePoint::of_start_of_day(bounds.max_day).add_days(1))
            - ctx.position(TimePoint::of_start_of_day(bounds.min_day));
        let max_label_overflow = registry
            .draws
            .iter()
            .flatten()
            .filter(|draw| !bounds.is_hidden(&diagram.model, draw.task))
            .map(|draw| draw.label_overflow(&ctx, string_bounder, timeline_width))
            .fold(0.0, f64::max);
        let header_height = header.time_header_height(string_bounder);
        let footer_height = if diagram.display.show_footbox {
            header.time_footer_height(string_bounder)
        } else {
            0.0
        };
        let total_height = registry.total_height_without_footer + footer_height;
        let rows: Vec<(TaskId, f64)> = registry
            .draws
            .iter()
            .flatten()
            .filter(|draw| matches!(diagram.model.tasks[draw.task].kind, TaskKind::Impl(_)))
            .filter(|draw| !bounds.is_hidden(&diagram.model, draw.task))
            .map(|draw| {
                let y = registry.y(&ctx, draw.task).current_value();
                (
                    draw.task,
                    y + draw.full_height_task(&ctx, string_bounder) / 2.0,
                )
            })
            .collect();
        let table = GanttTaskTable::new(
            diagram,
            bounds,
            rows,
            header.full_header_height(string_bounder),
            string_bounder,
        );
        Self {
            diagram,
            bounds,
            header,
            registry,
            table,
            timeline_width,
            bars_width: timeline_width + max_label_overflow,
            header_height,
            footer_height,
            total_height,
        }
    }

    fn ctx(&self) -> DrawContext<'_> {
        DrawContext {
            diagram: self.diagram,
            scale: self.header.time_scale(),
        }
    }
}

impl TextBlock for GanttDiagramMainBlock<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.table.width() + self.bars_width + 20.0,
            self.total_height,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ctx = self.ctx();
        let diagram = self.diagram;
        let total_height_without_footer = self.registry.total_height_without_footer;
        self.table.draw_u(ug, total_height_without_footer);
        let ug = ug.translated(self.table.width(), 0.0);
        let back = diagram
            .style(&[SName::Timeline])
            .value(PName::BackGroundColor)
            .as_color();
        if !back.is_transparent() {
            let ug_back = ug.with_backcolor(back);
            ug_back.draw(&rectangle(self.timeline_width, self.header_height));
            if diagram.display.show_footbox {
                ug_back
                    .translated(0.0, total_height_without_footer)
                    .draw(&rectangle(self.timeline_width, self.footer_height));
            }
        }
        self.header
            .draw_time_header(&ug, total_height_without_footer);
        let model = &diagram.model;
        for constraint in &model.constraints {
            if self.bounds.printed_interval
                && is_constraint_hidden(
                    model,
                    constraint,
                    TimePoint::of_start_of_day(self.bounds.min_day),
                    TimePoint::of_end_of_day_minus_one_second(self.bounds.max_day),
                )
            {
                continue;
            }
            draw_arrow(&ctx, &self.registry, constraint, &ug);
        }
        let visible_draws = || {
            self.registry
                .draws
                .iter()
                .flatten()
                .filter(|draw| !self.bounds.is_hidden(model, draw.task))
        };
        for draw in visible_draws() {
            let y = self.registry.y(&ctx, draw.task).current_value();
            draw.draw_u(&ctx, &ug.translated(0.0, y));
        }
        for draw in visible_draws() {
            let y = self.registry.y(&ctx, draw.task).current_value();
            draw.draw_title(&ctx, &ug.translated(0.0, y));
        }
        if !diagram.display.hide_resource_footbox {
            let min = TimePoint::of_start_of_day(self.bounds.min_day);
            let max = TimePoint::of_end_of_day_minus_one_second(self.bounds.max_day);
            for ((name, _), y) in model.resources.iter().zip(&self.registry.resource_ys) {
                draw_resource(&ctx, &ug.translated(0.0, *y), name, min, max);
            }
        }
        if diagram.display.show_footbox {
            self.header
                .draw_time_footer(&ug.translated(0.0, total_height_without_footer));
        }
    }
}

/// Whether an end of the constraint lies outside the printed days (`isHidden`).
fn is_constraint_hidden(
    model: &super::model::GanttModel,
    constraint: &GanttConstraint,
    min: TimePoint,
    max: TimePoint,
) -> bool {
    [&constraint.source, &constraint.dest]
        .into_iter()
        .any(|instant| {
            let now = model.instant_precise(instant);
            now < min || now > max
        })
}

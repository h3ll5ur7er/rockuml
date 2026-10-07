use crate::color::HColor;
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{XDimension2D, XPoint2D, rotate};
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::arrow::{
    ArrowConfiguration, ArrowDecoration, ArrowDirection, ArrowHead, ArrowPart,
};
use crate::skin::component::{Area, ArrowComponent, Component, TextualPart};
use crate::style::{PName, Style, ValueReading};

pub(crate) const ARROW_DELTA_X: f64 = 10.0;
pub(crate) const ARROW_DELTA_Y: f64 = 4.0;
pub(crate) const SPACE_CROSS_X: f64 = 6.0;
pub(crate) const DIAM_CIRCLE: f64 = 8.0;
pub(crate) const THIN_CIRCLE: f64 = 1.5;

/// What every message arrow shares (PlantUML's `AbstractComponentRoseArrow`): the label, colours, and the
/// configuration with the style's line thickness.
pub(crate) struct ArrowParts {
    pub text: TextualPart,
    pub style: Style,
    pub foreground: HColor,
    pub background: HColor,
    pub configuration: ArrowConfiguration,
}

impl ArrowParts {
    pub(crate) fn new(text: TextualPart, style: Style, configuration: &ArrowConfiguration) -> Self {
        Self {
            foreground: style.value(PName::LineColor).as_color(),
            background: style.value(PName::BackGroundColor).as_color(),
            configuration: configuration.with_thickness(style.stroke().thickness),
            text,
            style,
        }
    }

    pub(crate) fn text_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_height(string_bounder)
    }

    pub(crate) fn text_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text.text_width(string_bounder)
    }

    pub(crate) const PADDING_Y: f64 = 4.0;
}

/// An arrow between two lifelines.
pub(crate) struct ComponentRoseArrow {
    parts: ArrowParts,
    message_position: Option<HorizontalAlignment>,
    nice_arrow: bool,
    below_for_response: bool,
    inclination1: f64,
    inclination2: f64,
}

impl ComponentRoseArrow {
    pub(crate) fn new(
        parts: ArrowParts,
        message_position: Option<HorizontalAlignment>,
        nice_arrow: bool,
        below_for_response: bool,
    ) -> Self {
        Self {
            inclination1: f64::from(parts.configuration.inclination1()),
            inclination2: f64::from(parts.configuration.inclination2()),
            parts,
            message_position,
            nice_arrow,
            below_for_response,
        }
    }

    fn configuration(&self) -> &ArrowConfiguration {
        &self.parts.configuration
    }

    fn is_below_for_response(&self) -> bool {
        self.below_for_response && self.configuration().is_reverse_define()
    }

    fn direction(&self) -> ArrowDirection {
        self.configuration().arrow_direction()
    }

    fn draw_dressing1(
        &self,
        ug: &UGraphic,
        head: ArrowHead,
        part: ArrowPart,
        decoration: ArrowDecoration,
        len_full: f64,
    ) {
        let mut ug = ug.clone();
        if decoration == ArrowDecoration::Circle {
            ug.with_stroke(UStroke::with_thickness(THIN_CIRCLE))
                .with_color(self.parts.foreground.clone())
                .with_backcolor(self.parts.background.clone())
                .translated(
                    -DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                    -DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
                )
                .draw(&circle());
            if head != ArrowHead::CrossX {
                ug = ug.translated(DIAM_CIRCLE / 2.0 + THIN_CIRCLE, 0.0);
            }
        }
        let angle = (-self.inclination1).atan2(len_full);
        match head {
            ArrowHead::Async => {
                let thin = self.configuration().apply_thickness_only(&ug);
                if part != ArrowPart::BottomPart {
                    draw_line(&thin, rotate(ARROW_DELTA_X, -ARROW_DELTA_Y, angle));
                }
                if part != ArrowPart::TopPart {
                    draw_line(&thin, rotate(ARROW_DELTA_X, ARROW_DELTA_Y, angle));
                }
            }
            ArrowHead::CrossX => {
                draw_cross(&ug.with_stroke(UStroke::with_thickness(2.0)), SPACE_CROSS_X)
            }
            ArrowHead::Normal => {
                let points = rotate_all(polygon_reverse(part, self.nice_arrow), angle);
                ug.with_backcolor(self.parts.foreground.clone())
                    .draw(&UShape::Polygon(points));
            }
            ArrowHead::None => {}
        }
    }

    fn draw_dressing2(
        &self,
        ug: &UGraphic,
        head: ArrowHead,
        part: ArrowPart,
        decoration: ArrowDecoration,
        len_full: f64,
    ) {
        let mut ug = ug.clone();
        if decoration == ArrowDecoration::Circle {
            ug = ug
                .with_stroke(UStroke::with_thickness(THIN_CIRCLE))
                .with_color(self.parts.foreground.clone())
                .with_backcolor(self.parts.background.clone());
            ug.translated(
                -DIAM_CIRCLE / 2.0 + THIN_CIRCLE,
                -DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
            )
            .draw(&circle());
            ug = ug
                .with_stroke(UStroke::SIMPLE)
                .translated(-DIAM_CIRCLE / 2.0 - THIN_CIRCLE, 0.0);
        }
        let angle = self.inclination2.atan2(len_full);
        match head {
            ArrowHead::Async => {
                let thin = self.configuration().apply_thickness_only(&ug);
                if part != ArrowPart::BottomPart {
                    draw_line(&thin, rotate(-ARROW_DELTA_X, -ARROW_DELTA_Y, angle));
                }
                if part != ArrowPart::TopPart {
                    draw_line(&thin, rotate(-ARROW_DELTA_X, ARROW_DELTA_Y, angle));
                }
            }
            ArrowHead::CrossX => draw_cross(
                &ug.with_stroke(UStroke::with_thickness(2.0)),
                -SPACE_CROSS_X - ARROW_DELTA_X,
            ),
            ArrowHead::Normal => {
                let points = rotate_all(polygon_normal(part, self.nice_arrow), angle);
                ug.with_backcolor(self.parts.foreground.clone())
                    .draw(&UShape::Polygon(points));
            }
            ArrowHead::None => {}
        }
    }
}

fn circle() -> UShape {
    UShape::Ellipse(UEllipse::new(DIAM_CIRCLE, DIAM_CIRCLE))
}

fn draw_line(ug: &UGraphic, (dx, dy): (f64, f64)) {
    ug.draw(&UShape::Line { dx, dy });
}

/// An `x` head, `ARROW_DELTA_X` wide, starting `x` from the arrow's end.
pub(crate) fn draw_cross(ug: &UGraphic, x: f64) {
    let half = (ARROW_DELTA_X / 2.0).trunc();
    ug.translated(x, -half).draw(&UShape::Line {
        dx: ARROW_DELTA_X,
        dy: ARROW_DELTA_X,
    });
    ug.translated(x, half).draw(&UShape::Line {
        dx: ARROW_DELTA_X,
        dy: -ARROW_DELTA_X,
    });
}

fn rotate_all(points: Vec<(f64, f64)>, angle: f64) -> Vec<(f64, f64)> {
    if angle == 0.0 {
        return points;
    }
    points
        .into_iter()
        .map(|(x, y)| rotate(x, y, angle))
        .collect()
}

/// A head pointing right, its tip at the origin.
fn polygon_normal(part: ArrowPart, nice_arrow: bool) -> Vec<(f64, f64)> {
    match part {
        ArrowPart::TopPart => vec![
            (-ARROW_DELTA_X, -ARROW_DELTA_Y),
            (0.0, 0.0),
            (-ARROW_DELTA_X, 0.0),
        ],
        ArrowPart::BottomPart => vec![
            (-ARROW_DELTA_X, 0.0),
            (0.0, 0.0),
            (-ARROW_DELTA_X, ARROW_DELTA_Y),
        ],
        ArrowPart::Full => {
            let mut points = vec![
                (-ARROW_DELTA_X, -ARROW_DELTA_Y),
                (0.0, 0.0),
                (-ARROW_DELTA_X, ARROW_DELTA_Y),
            ];
            if nice_arrow {
                points.push((-ARROW_DELTA_X + 4.0, 0.0));
            }
            points
        }
    }
}

/// A head pointing left, its tip at the origin.
fn polygon_reverse(part: ArrowPart, nice_arrow: bool) -> Vec<(f64, f64)> {
    match part {
        ArrowPart::TopPart => vec![
            (ARROW_DELTA_X, -ARROW_DELTA_Y),
            (0.0, 0.0),
            (ARROW_DELTA_X, 0.0),
        ],
        ArrowPart::BottomPart => vec![
            (ARROW_DELTA_X, 0.0),
            (0.0, 0.0),
            (ARROW_DELTA_X, ARROW_DELTA_Y),
        ],
        ArrowPart::Full => {
            let mut points = vec![
                (ARROW_DELTA_X, -ARROW_DELTA_Y),
                (0.0, 0.0),
                (ARROW_DELTA_X, ARROW_DELTA_Y),
            ];
            if nice_arrow {
                points.push((ARROW_DELTA_X - 4.0, 0.0));
            }
            points
        }
    }
}

impl Component for ComponentRoseArrow {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.parts.text_width(string_bounder) + ARROW_DELTA_X
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.parts.text_height(string_bounder)
            + ARROW_DELTA_Y
            + 2.0 * self.padding_y()
            + self.inclination1
            + self.inclination2
    }

    fn padding_y(&self) -> f64 {
        ArrowParts::PADDING_Y
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let configuration = self.configuration();
        if configuration.is_hidden() {
            return;
        }
        let dimension = area.dimension;
        let string_bounder = ug.string_bounder();
        let ug = ug.with_color(self.parts.foreground.clone());
        let dressing1 = configuration.dressing1();
        let dressing2 = configuration.dressing2();

        let mut start = 0.0;
        let mut len = dimension.width - 1.0;
        let len_full = dimension.width;
        let pos1 = start + 1.0;
        let pos2 = len - 1.0;

        if configuration.decoration2() == ArrowDecoration::Circle {
            len -= if dressing2.head == ArrowHead::None {
                DIAM_CIRCLE / 2.0
            } else {
                DIAM_CIRCLE / 2.0 + THIN_CIRCLE
            };
        }
        if configuration.decoration1() == ArrowDecoration::Circle {
            let shift = match dressing1.head {
                ArrowHead::None => Some(DIAM_CIRCLE / 2.0),
                ArrowHead::Async | ArrowHead::Normal => Some(DIAM_CIRCLE / 2.0 + THIN_CIRCLE),
                ArrowHead::CrossX => None,
            };
            if let Some(shift) = shift {
                start += shift;
                len -= shift;
            }
        }
        let half_head = (ARROW_DELTA_X / 2.0).trunc();
        if dressing2.part == ArrowPart::Full && dressing2.head == ArrowHead::Normal {
            len -= half_head;
        }
        if dressing1.part == ArrowPart::Full && dressing1.head == ArrowHead::Normal {
            start += half_head;
            len -= half_head;
        }
        if dressing2.head == ArrowHead::CrossX {
            len -= 2.0 * SPACE_CROSS_X;
        }
        if dressing1.head == ArrowHead::CrossX {
            start += 2.0 * SPACE_CROSS_X;
            len -= 2.0 * SPACE_CROSS_X;
        }

        let (pos_arrow, y_text) = if self.is_below_for_response() {
            (0.0, self.parts.text.padding().top)
        } else {
            (self.parts.text_height(string_bounder), 0.0)
        };

        self.draw_dressing1(
            &ug.translated(pos1, pos_arrow + self.inclination1),
            dressing1.head,
            dressing1.part,
            configuration.decoration1(),
            len_full,
        );
        self.draw_dressing2(
            &ug.translated(pos2, pos_arrow + self.inclination2),
            dressing2.head,
            dressing2.part,
            configuration.decoration2(),
            len_full,
        );

        let stroked = configuration.apply_stroke(&ug, &self.parts.style);
        if self.inclination1 == 0.0 && self.inclination2 == 0.0 {
            stroked
                .translated(start, pos_arrow)
                .draw(&UShape::Line { dx: len, dy: 0.0 });
        } else if self.inclination1 != 0.0 {
            draw_segment(
                &stroked,
                (start + len, pos_arrow),
                (0.0, pos_arrow + self.inclination1),
            );
        } else {
            draw_segment(
                &stroked,
                (start, pos_arrow),
                (pos2, pos_arrow + self.inclination2),
            );
        }

        let direction = self.direction();
        let text = self.parts.text.text_block();
        let text_pos = match self.message_position {
            Some(HorizontalAlignment::Center) => {
                let text_width = text.calculate_dimension(string_bounder).width;
                (dimension.width - area.text_delta_x.abs() - text_width) / 2.0
            }
            Some(HorizontalAlignment::Right) => {
                let text_width = text.calculate_dimension(string_bounder).width;
                let head = if direction == ArrowDirection::LeftToRightNormal {
                    ARROW_DELTA_X
                } else {
                    0.0
                };
                dimension.width
                    - area.text_delta_x.abs()
                    - text_width
                    - self.parts.text.padding().right
                    - head
            }
            _ => {
                let head = if matches!(
                    direction,
                    ArrowDirection::RightToLeftReverse | ArrowDirection::BothDirection
                ) {
                    ARROW_DELTA_X
                } else {
                    0.0
                };
                self.parts.text.padding().left + head
            }
        };
        text.draw_u(&ug.translated(text_pos + area.text_delta_x.max(0.0), y_text));
    }
}

fn draw_segment(ug: &UGraphic, (x1, y1): (f64, f64), (x2, y2): (f64, f64)) {
    ug.translated(x1, y1).draw(&UShape::Line {
        dx: x2 - x1,
        dy: y2 - y1,
    });
}

impl ArrowComponent for ComponentRoseArrow {
    fn start_point(&self, string_bounder: &dyn StringBounder, dimension: XDimension2D) -> XPoint2D {
        let y = self.y_point(string_bounder);
        if self.direction() == ArrowDirection::LeftToRightNormal {
            XPoint2D::new(self.padding_x(), y + self.inclination2)
        } else {
            XPoint2D::new(dimension.width + self.padding_x(), y + self.inclination2)
        }
    }

    fn end_point(&self, string_bounder: &dyn StringBounder, dimension: XDimension2D) -> XPoint2D {
        let y = self.y_point(string_bounder);
        if self.direction() == ArrowDirection::LeftToRightNormal {
            XPoint2D::new(dimension.width + self.padding_x(), y)
        } else {
            XPoint2D::new(self.padding_x(), y)
        }
    }

    fn y_point(&self, string_bounder: &dyn StringBounder) -> f64 {
        if self.is_below_for_response() {
            return self.padding_y();
        }
        self.parts.text_height(string_bounder) + self.padding_y()
    }

    fn pos_arrow(&self, string_bounder: &dyn StringBounder) -> f64 {
        if self.is_below_for_response() {
            return 0.0;
        }
        self.parts.text_height(string_bounder) - 2.0 * self.parts.text.padding().top
    }
}

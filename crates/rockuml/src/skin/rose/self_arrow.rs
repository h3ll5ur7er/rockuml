use super::arrow::{
    ARROW_DELTA_X, ARROW_DELTA_Y, ArrowParts, DIAM_CIRCLE, SPACE_CROSS_X, THIN_CIRCLE,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{XDimension2D, XPoint2D};
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::arrow::{ArrowDecoration, ArrowHead, ArrowPart};
use crate::skin::component::{Area, ArrowComponent, Component};

const ARROW_WIDTH: f64 = 45.0;
const X_RIGHT: f64 = ARROW_WIDTH - 3.0;
const ARROW_ONLY_HEIGHT: f64 = 13.0;

/// A message from a participant to itself: out to the right (or left, when written right to left) and
/// back (PlantUML's `ComponentRoseSelfArrow`).
pub(crate) struct ComponentRoseSelfArrow {
    parts: ArrowParts,
    nice_arrow: bool,
}

impl ComponentRoseSelfArrow {
    pub(crate) fn new(parts: ArrowParts, nice_arrow: bool) -> Self {
        Self { parts, nice_arrow }
    }

    /// How far the loop and label reach, which may be less than the width reserved.
    pub(crate) fn drawn_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        let pure_text_width = self.parts.text.pure_text_width(string_bounder);
        let label_reach = if pure_text_width == 0.0 {
            0.0
        } else {
            self.parts.text.padding().left + pure_text_width
        };
        X_RIGHT.max(label_reach)
    }

    fn circle() -> UShape {
        UShape::Ellipse(UEllipse::new(DIAM_CIRCLE, DIAM_CIRCLE))
    }

    fn circle_ug(&self, ug: &UGraphic) -> UGraphic {
        ug.with_stroke(UStroke::with_thickness(THIN_CIRCLE))
            .with_color(self.parts.foreground.clone())
            .with_backcolor(self.parts.background.clone())
    }

    fn cross(ug: &UGraphic, x: f64, y: f64) {
        let ug = ug.with_stroke(UStroke::with_thickness(2.0));
        ug.translated(x, y - ARROW_DELTA_X / 2.0)
            .draw(&UShape::Line {
                dx: ARROW_DELTA_X,
                dy: ARROW_DELTA_X,
            });
        ug.translated(x, y + ARROW_DELTA_X / 2.0)
            .draw(&UShape::Line {
                dx: ARROW_DELTA_X,
                dy: -ARROW_DELTA_X,
            });
    }

    /// Both lines of a two-line head starting at the given point.
    fn async_head(&self, ug: &UGraphic, x: f64, y: f64, dx: f64, first_dy: f64) {
        let configuration = &self.parts.configuration;
        let ug = configuration.apply_thickness_only(ug).translated(x, y);
        if configuration.part() != ArrowPart::BottomPart {
            ug.draw(&UShape::Line { dx, dy: first_dy });
        }
        if configuration.part() != ArrowPart::TopPart {
            ug.draw(&UShape::Line { dx, dy: -first_dy });
        }
    }

    fn draw_right_side(&self, ug: &UGraphic, ug2: &UGraphic, area: &Area, text_height: f64) {
        let configuration = &self.parts.configuration;
        let mut x1 = if area.delta_x1 < 0.0 {
            area.delta_x1
        } else {
            0.0
        };
        let mut x2 = if area.delta_x1 > 0.0 {
            -area.delta_x1
        } else {
            1.0
        };
        if configuration.decoration1() == ArrowDecoration::Circle {
            self.circle_ug(ug2)
                .translated(
                    x1 + 1.0 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                    text_height - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
                )
                .draw(&Self::circle());
            x1 += DIAM_CIRCLE / 2.0 + THIN_CIRCLE + 1.0;
            if configuration.dressing1().head == ArrowHead::None {
                x1 -= THIN_CIRCLE + 1.0;
            }
        }
        if configuration.decoration2() == ArrowDecoration::Circle {
            self.circle_ug(ug2)
                .translated(
                    x2 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                    text_height + ARROW_ONLY_HEIGHT - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
                )
                .draw(&Self::circle());
            x2 += DIAM_CIRCLE / 2.0 + THIN_CIRCLE;
        }
        let starting_cross = configuration.dressing1().head == ArrowHead::CrossX;
        if starting_cross {
            x1 += 2.0 * SPACE_CROSS_X;
        }
        let final_cross = configuration.dressing2().head == ArrowHead::CrossX;
        if final_cross {
            x2 += 2.0 * SPACE_CROSS_X;
        }
        let async2 = configuration.dressing2().head == ArrowHead::Async;
        if async2 {
            x2 -= 1.0;
        }
        ug2.translated(x1, text_height).draw(&UShape::Line {
            dx: X_RIGHT - x1,
            dy: 0.0,
        });
        ug2.translated(X_RIGHT, text_height).draw(&UShape::Line {
            dx: 0.0,
            dy: ARROW_ONLY_HEIGHT,
        });
        ug2.translated(x2, text_height + ARROW_ONLY_HEIGHT)
            .draw(&UShape::Line {
                dx: X_RIGHT - x2,
                dy: 0.0,
            });
        if async2 {
            x2 += 1.0;
        }
        if starting_cross {
            Self::cross(ug, SPACE_CROSS_X, text_height);
        } else if configuration.is_async1() {
            self.async_head(ug, x1, text_height, ARROW_DELTA_X, ARROW_DELTA_Y);
        } else if configuration.dressing1().head == ArrowHead::Normal {
            let polygon = translate(self.polygon(), 0.0, text_height);
            ug.with_backcolor(self.parts.foreground.clone())
                .translated(x1, 0.0)
                .draw(&UShape::Polygon(polygon));
        }
        if final_cross {
            Self::cross(ug, SPACE_CROSS_X, text_height + ARROW_ONLY_HEIGHT);
        } else if configuration.is_async2() {
            self.async_head(
                ug,
                x2,
                text_height + ARROW_ONLY_HEIGHT,
                ARROW_DELTA_X,
                -ARROW_DELTA_Y,
            );
        } else if configuration.dressing2().head == ArrowHead::Normal {
            let polygon = translate(self.polygon(), 0.0, text_height + ARROW_ONLY_HEIGHT);
            ug.with_backcolor(self.parts.foreground.clone())
                .translated(x2, 0.0)
                .draw(&UShape::Polygon(polygon));
        }
    }

    fn draw_left_side(
        &self,
        ug: &UGraphic,
        ug2: &UGraphic,
        area: &Area,
        text_height: f64,
        width: f64,
    ) {
        let configuration = &self.parts.configuration;
        let delta_size = area.live_delta_size;
        let dx = area.delta_x1;
        let level = f64::from(area.level);
        let mut x1 = 0.0;
        let mut x2 = 1.0;
        let extra_indent = level * delta_size;
        if dx < 0.0 {
            x2 += if level > 0.0 {
                -extra_indent
            } else {
                delta_size
            };
        } else if dx > 0.0 {
            x1 += if level > 1.0 {
                delta_size - extra_indent
            } else {
                0.0
            };
            x2 += if area.level == 1 { -delta_size } else { 0.0 };
        } else if level > 1.0 {
            x1 -= extra_indent - delta_size;
            x2 -= extra_indent - delta_size;
        }
        if configuration.decoration1() == ArrowDecoration::Circle {
            self.circle_ug(ug2)
                .translated(
                    width - x1 + 1.0 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                    text_height - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
                )
                .draw(&Self::circle());
            x1 += DIAM_CIRCLE / 2.0 - THIN_CIRCLE;
            if configuration.dressing1().head == ArrowHead::None {
                x1 += THIN_CIRCLE;
            }
        }
        if configuration.decoration2() == ArrowDecoration::Circle {
            self.circle_ug(ug2)
                .translated(
                    width - x2 + 2.0 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                    text_height + ARROW_ONLY_HEIGHT - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
                )
                .draw(&Self::circle());
            x2 += DIAM_CIRCLE / 2.0 + THIN_CIRCLE;
        }
        let cross_shift = SPACE_CROSS_X
            + if level > 0.0 {
                extra_indent
            } else {
                SPACE_CROSS_X
            };
        let starting_cross = configuration.dressing1().head == ArrowHead::CrossX;
        if starting_cross {
            x1 += cross_shift;
        }
        let final_cross = configuration.dressing2().head == ArrowHead::CrossX;
        if final_cross {
            x2 += cross_shift;
        }
        let full_normal2 = configuration.dressing2().part == ArrowPart::Full
            && configuration.dressing2().head == ArrowHead::Normal;
        let extra_line = if full_normal2 { 1.0 } else { 0.0 };
        x1 += 1.0;
        let left = width - X_RIGHT;
        ug2.translated(left, text_height).draw(&UShape::Line {
            dx: X_RIGHT - x1,
            dy: 0.0,
        });
        ug2.translated(left, text_height).draw(&UShape::Line {
            dx: 0.0,
            dy: ARROW_ONLY_HEIGHT,
        });
        ug2.translated(left, text_height + ARROW_ONLY_HEIGHT)
            .draw(&UShape::Line {
                dx: X_RIGHT - x2 - extra_line,
                dy: 0.0,
            });
        if full_normal2 {
            x2 += 1.0;
        }
        if configuration.dressing2().head == ArrowHead::Async {
            x2 += 1.0;
        }
        self.draw_left_side_heads(ug, text_height, width, x1, x2);
    }

    /// The heads or crosses at both ends of an arrow drawn to the left of its lifeline.
    fn draw_left_side_heads(&self, ug: &UGraphic, text_height: f64, width: f64, x1: f64, x2: f64) {
        let configuration = &self.parts.configuration;
        let starting_cross = configuration.dressing1().head == ArrowHead::CrossX;
        let final_cross = configuration.dressing2().head == ArrowHead::CrossX;
        if starting_cross {
            Self::cross(ug, width - x1 - SPACE_CROSS_X / 2.0, text_height);
        } else if configuration.dressing1().head == ArrowHead::Normal {
            let polygon = translate(self.polygon(), width - x1, text_height);
            ug.with_backcolor(self.parts.foreground.clone())
                .translated(x1, 0.0)
                .draw(&UShape::Polygon(polygon));
        }
        if final_cross {
            Self::cross(
                ug,
                width - x2 - SPACE_CROSS_X / 2.0,
                text_height + ARROW_ONLY_HEIGHT,
            );
        } else if configuration.is_async2() {
            self.async_head(
                ug,
                width - x2,
                text_height + ARROW_ONLY_HEIGHT,
                -ARROW_DELTA_X,
                -ARROW_DELTA_Y,
            );
        } else if configuration.dressing2().head == ArrowHead::Normal {
            let polygon = translate(self.polygon(), 0.0, text_height + ARROW_ONLY_HEIGHT);
            ug.with_backcolor(self.parts.foreground.clone())
                .translated(width - x2, 0.0)
                .draw(&UShape::Polygon(polygon));
        }
    }

    /// The head, pointing right, or left when written right to left.
    fn polygon(&self) -> Vec<(f64, f64)> {
        let direction = if self.parts.configuration.is_reverse_define() {
            -1.0
        } else {
            1.0
        };
        let x = direction * ARROW_DELTA_X;
        match self.parts.configuration.part() {
            ArrowPart::TopPart => vec![(x - 1.0, -ARROW_DELTA_Y), (-1.0, 0.0), (x - 1.0, 0.0)],
            ArrowPart::BottomPart => vec![(x - 1.0, 0.0), (-1.0, 0.0), (x - 1.0, ARROW_DELTA_Y)],
            ArrowPart::Full => {
                let mut points = vec![(x, -ARROW_DELTA_Y), (0.0, 0.0), (x, ARROW_DELTA_Y)];
                if self.nice_arrow {
                    points.push((x - direction * 4.0, 0.0));
                }
                points
            }
        }
    }
}

fn translate(points: Vec<(f64, f64)>, dx: f64, dy: f64) -> Vec<(f64, f64)> {
    points.into_iter().map(|(x, y)| (x + dx, y + dy)).collect()
}

impl Component for ComponentRoseSelfArrow {
    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.parts.text_width(string_bounder).max(ARROW_WIDTH + 5.0)
    }

    fn preferred_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.parts.text_height(string_bounder)
            + ARROW_DELTA_Y
            + ARROW_ONLY_HEIGHT
            + 2.0 * self.padding_y()
    }

    fn padding_y(&self) -> f64 {
        ArrowParts::PADDING_Y
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let configuration = &self.parts.configuration;
        if configuration.is_hidden() {
            return;
        }
        let string_bounder = ug.string_bounder();
        let text_height = self.parts.text_height(string_bounder);
        let width = self.preferred_width(string_bounder);
        let ug = ug.with_color(self.parts.foreground.clone());
        let ug2 = configuration.apply_own_stroke(&ug);
        if configuration.is_reverse_define() {
            self.draw_left_side(&ug, &ug2, area, text_height, width);
        } else {
            self.draw_right_side(&ug, &ug2, area, text_height);
        }
        self.parts
            .text
            .text_block()
            .draw_u(&ug.translated(self.parts.text.padding().left, 0.0));
    }
}

impl ArrowComponent for ComponentRoseSelfArrow {
    fn start_point(
        &self,
        string_bounder: &dyn StringBounder,
        _dimension: XDimension2D,
    ) -> XPoint2D {
        XPoint2D::new(
            self.padding_x(),
            self.parts.text_height(string_bounder) + self.padding_y(),
        )
    }

    fn end_point(&self, string_bounder: &dyn StringBounder, _dimension: XDimension2D) -> XPoint2D {
        XPoint2D::new(
            self.padding_x(),
            self.parts.text_height(string_bounder) + ARROW_ONLY_HEIGHT + self.padding_y(),
        )
    }

    fn y_point(&self, string_bounder: &dyn StringBounder) -> f64 {
        let text_height = self.parts.text_height(string_bounder);
        (text_height + text_height + ARROW_ONLY_HEIGHT) / 2.0 + self.padding_x()
    }

    fn pos_arrow(&self, _string_bounder: &dyn StringBounder) -> f64 {
        unreachable!("teoz never asks a self message where its arrow is")
    }
}

//! The curves railroad tracks bend with (PlantUML's `CornerCurved`, `Zigzag` and `Brace`).

use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

/// Which way a corner turns, named by the quarter of a circle it draws.
#[derive(Clone, Copy)]
pub(super) enum CornerType {
    NW,
    NE,
    SE,
    SW,
}

/// A quarter turn `delta` wide (`CornerCurved`).
pub(super) struct CornerCurved {
    corner_type: CornerType,
    delta: f64,
}

impl CornerCurved {
    pub(super) fn new(corner_type: CornerType, delta: f64) -> Self {
        Self { corner_type, delta }
    }

    pub(super) fn draw_u(&self, ug: &UGraphic) {
        let delta = self.delta;
        let a = delta / 4.0;
        let (start, ctrl1, ctrl2, end) = match self.corner_type {
            CornerType::SW => ((0.0, -delta), (0.0, -a), (a, 0.0), (delta, 0.0)),
            CornerType::SE => ((0.0, -delta), (0.0, -a), (-a, 0.0), (-delta, 0.0)),
            CornerType::NE => ((-delta, 0.0), (-a, 0.0), (0.0, a), (0.0, delta)),
            CornerType::NW => ((0.0, delta), (0.0, a), (a, 0.0), (delta, 0.0)),
        };
        ug.draw(&UShape::path(vec![
            USegment::MoveTo(start.0, start.1),
            USegment::CubicTo { ctrl1, ctrl2, end },
        ]));
    }
}

/// An S-shaped track from one height to another (`Zigzag`).
pub(super) struct Zigzag {
    ctrl: f64,
    width: f64,
    height: f64,
}

impl Zigzag {
    pub(super) fn new(ctrl: f64, width: f64, height: f64) -> Self {
        Self {
            ctrl,
            width,
            height,
        }
    }

    pub(super) fn path_down(&self) -> UShape {
        let (ctrl, width, height) = (self.ctrl, self.width, self.height);
        let (xm, ym) = (width / 2.0, height / 2.0);
        UShape::path(vec![
            USegment::MoveTo(0.0, 0.0),
            USegment::CubicTo {
                ctrl1: (ctrl, 0.0),
                ctrl2: (xm, ym - ctrl),
                end: (xm, ym),
            },
            USegment::CubicTo {
                ctrl1: (xm, ym + ctrl),
                ctrl2: (width - ctrl, height),
                end: (width, height),
            },
        ])
    }

    pub(super) fn path_up(&self) -> UShape {
        let (ctrl, width, height) = (self.ctrl, self.width, self.height);
        let (xm, ym) = (width / 2.0, height / 2.0);
        UShape::path(vec![
            USegment::MoveTo(0.0, height),
            USegment::CubicTo {
                ctrl1: (ctrl, height),
                ctrl2: (xm, ym + ctrl),
                end: (xm, ym),
            },
            USegment::CubicTo {
                ctrl1: (xm, ym - ctrl),
                ctrl2: (width - ctrl, 0.0),
                end: (width, 0.0),
            },
        ])
    }
}

/// A horizontal curly brace `width` wide, over the label of a counted repetition (`Brace`).
pub(super) struct Brace {
    width: f64,
}

impl Brace {
    pub(super) fn new(width: f64) -> Self {
        Self { width }
    }

    pub(super) fn draw_u(&self, ug: &UGraphic) {
        const CINQ: f64 = 5.0;
        let ug = ug.with_stroke(UStroke::with_thickness(0.5));
        let half = self.width / 2.0;
        CornerCurved::new(CornerType::NW, CINQ).draw_u(&ug);
        CornerCurved::new(CornerType::SE, CINQ).draw_u(&ug.translated(half, 0.0));
        CornerCurved::new(CornerType::SW, CINQ).draw_u(&ug.translated(half, 0.0));
        CornerCurved::new(CornerType::NE, CINQ).draw_u(&ug.translated(self.width, 0.0));
        let line = UShape::Line {
            dx: half - 2.0 * CINQ,
            dy: 0.0,
        };
        ug.translated(CINQ, 0.0).draw(&line);
        ug.translated(CINQ + half, 0.0).draw(&line);
    }
}

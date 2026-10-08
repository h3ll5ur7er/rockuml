use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::shape::URectangle;

/// Lists what reaches it, one line per shape: its kind, absolute position and size.
#[derive(Default)]
pub(crate) struct Recorder {
    pub(crate) lines: Vec<String>,
}

impl UGraphicBackend for Recorder {
    fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam) {
        let (x, y) = (at.dx, at.dy);
        let line = match shape {
            UShape::Rectangle(rectangle) => {
                format!("rect {x},{y} {}x{}", rectangle.width, rectangle.height)
            }
            UShape::Line { dx, dy } => format!("line {x},{y} {dx},{dy}"),
            UShape::Polygon(polygon) => format!("polygon {x},{y} {:?}", polygon.points()),
            other => format!("{} {x},{y}", other.java_class_name()),
        };
        let line = if param.stroke == UStroke::SIMPLE {
            line
        } else {
            format!("{line} stroke {}", param.stroke)
        };
        self.lines.push(line);
    }
}

/// A surface recording into the returned recorder.
pub(crate) fn recording() -> (UGraphic, Rc<RefCell<Recorder>>) {
    let recorder = Rc::new(RefCell::new(Recorder::default()));
    let ug = UGraphic::new(recorder.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
    (ug, recorder)
}

fn rectangle() -> UShape {
    UShape::Rectangle(URectangle::new(10.0, 5.0))
}

/// Keeps its translation to itself and hands the layer below twice the distance, as the compression layer
/// hands it transformed positions.
struct Doubling {
    ug: UGraphic,
    translate: UTranslate,
}

impl UGraphicLayer for Doubling {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    fn apply(&self, change: UChange) -> UGraphic {
        match change {
            UChange::Translate(translate) => UGraphic::from_layer(Self {
                ug: self.ug.clone(),
                translate: self.translate.compose(translate),
            }),
            change => UGraphic::from_layer(Self {
                ug: self.ug.apply(change),
                translate: self.translate,
            }),
        }
    }

    fn draw(&self, _this: &UGraphic, shape: AnyShape<'_>) {
        self.ug.apply(self.translate.multiply_by(2.0)).draw(shape);
    }
}

#[test]
fn surfaces_draw_at_the_sum_of_their_translations() {
    let (ug, recorder) = recording();
    ug.translated(1.0, 2.0)
        .translated(3.0, 4.0)
        .draw(&rectangle());
    assert_eq!(recorder.borrow().lines, ["rect 4,6 10x5"]);
}

#[test]
fn layers_decide_how_changes_reach_the_surface_below() {
    let (ug, recorder) = recording();
    let doubling = UGraphic::from_layer(Doubling {
        ug: ug.translated(100.0, 0.0),
        translate: UTranslate::default(),
    });
    doubling
        .translated(1.0, 2.0)
        .with_stroke(UStroke::with_thickness(3.0))
        .translated(3.0, 4.0)
        .draw(&rectangle());
    assert_eq!(
        recorder.borrow().lines,
        ["rect 108,12 10x5 stroke 0.0-0.0-3.0"]
    );
    // Queries go to the surface below.
    assert_eq!(doubling.param().stroke, UStroke::SIMPLE);
}

#[test]
fn only_the_outermost_layer_answers_instanceof() {
    let (ug, _) = recording();
    assert!(ug.layer::<Doubling>().is_none());
    let doubling = UGraphic::from_layer(Doubling {
        ug,
        translate: UTranslate::default(),
    });
    assert!(doubling.translated(1.0, 1.0).layer::<Doubling>().is_some());
    let outer = UGraphic::from_layer(HorizontalLineLayer {
        ug: doubling,
        translate: UTranslate::default(),
        drawer: Rc::new(UGraphicStencil {
            stencil: Rc::new(super::super::stencil::RectangleStencil { width: 1.0 }),
            default_stroke: None,
        }),
    });
    assert!(outer.layer::<Doubling>().is_none());
}

/// Separators span the stencil from where it was set, whatever layers lie below.
#[test]
fn separators_on_layers_are_drawn_where_the_stencil_was_set() {
    let (ug, recorder) = recording();
    let doubling = UGraphic::from_layer(Doubling {
        ug,
        translate: UTranslate::default(),
    });
    let stenciled = doubling.translated(5.0, 5.0).with_stencil(Rc::new(
        super::super::stencil::RectangleStencil { width: 30.0 },
    ));
    stenciled
        .translated(0.0, 7.0)
        .draw_horizontal_line(&UHorizontalLine {
            style: '-',
            title: None,
            default_thickness: 1.0,
            skip: 0.0,
        });
    stenciled.translated(0.0, 7.0).draw(&rectangle());
    assert_eq!(
        recorder.borrow().lines,
        ["line 10,24 30,0", "rect 10,24 10x5"]
    );
}

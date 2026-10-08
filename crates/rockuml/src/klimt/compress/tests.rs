//! Replays `tools/oracle/activity-unit/CompressDump.java`: scenes of shapes compressed across, down and both
//! must measure and draw bit for bit as in PlantUML.

use std::cell::RefCell;
use std::rc::Rc;

use super::{CompressionMode, CompressionXorYBuilder};
use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::shape::{CenteredText, UEllipse, UPolygon, URectangle, USegment, UShape, UText};
use crate::klimt::ugraphic::{UGraphic, UGraphicBackend, UParam, UStroke};

const FIXTURE: &str = include_str!("../../../tests/data/activity-compress.txt");

fn number(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn pair(pair: &str) -> (f64, f64) {
    let (x, y) = pair.split_once(',').unwrap();
    (number(x), number(y))
}

fn translate(words: &str) -> UTranslate {
    let (dx, dy) = pair(words);
    UTranslate::new(dx, dy)
}

/// A block of fixed size drawing a rectangle of that size, as the text of a centred title.
struct Block(XDimension2D);

impl TextBlock for Block {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.0
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&UShape::Rectangle(URectangle::new(
            self.0.width,
            self.0.height,
        )));
    }
}

fn shape(words: &[&str]) -> UShape {
    let size = || XDimension2D::new(number(words[1]), number(words[2]));
    match words[0] {
        "rect" => {
            let mut rectangle = URectangle::new(number(words[1]), number(words[2]));
            if words[3] == "true" {
                rectangle = rectangle.ignore_for_compression_on_x();
            }
            if words[4] == "true" {
                rectangle = rectangle.ignore_for_compression_on_y();
            }
            UShape::Rectangle(rectangle)
        }
        "line" => UShape::Line {
            dx: number(words[1]),
            dy: number(words[2]),
        },
        "polygon" => {
            let mut polygon = UPolygon::new(words[2..].iter().map(|point| pair(point)).collect());
            match words[1] {
                "ON_X" => polygon.set_compression_mode(CompressionMode::OnX),
                "ON_Y" => polygon.set_compression_mode(CompressionMode::OnY),
                _ => {}
            }
            UShape::Polygon(polygon)
        }
        "ellipse" => UShape::Ellipse(UEllipse::new(number(words[1]), number(words[2]))),
        "text" => UShape::Text(UText::new(
            words[2],
            FontConfiguration::black_blue_true(UFont::serif(words[1].parse().unwrap())),
        )),
        "path" => UShape::path(
            words[1..]
                .iter()
                .enumerate()
                .map(|(i, point)| {
                    let (x, y) = pair(point);
                    if i == 0 {
                        USegment::MoveTo(x, y)
                    } else {
                        USegment::LineTo(x, y)
                    }
                })
                .collect(),
        ),
        "empty" => UShape::Empty(size()),
        "centered" => UShape::CenteredText(CenteredText {
            text: Rc::new(Block(XDimension2D::new(number(words[2]), number(words[3])))),
            total_width: number(words[1]),
        }),
        other => panic!("unknown shape {other}"),
    }
}

/// One shape, moved twice and maybe drawn with a thicker stroke.
struct Op {
    first: UTranslate,
    second: UTranslate,
    stroke: Option<UStroke>,
    shape: UShape,
}

impl Op {
    fn parse(line: &str) -> Self {
        let words: Vec<&str> = line.split_whitespace().collect();
        Self {
            first: translate(words[0]),
            second: translate(words[1]),
            stroke: (words[2] != "-").then(|| UStroke::with_thickness(number(words[2]))),
            shape: shape(&words[3..]),
        }
    }

    fn draw(&self, ug: &UGraphic) {
        let mut ug = ug.apply(self.first).apply(self.second);
        if let Some(stroke) = self.stroke {
            ug = ug.apply(stroke);
        }
        ug.draw(&self.shape);
    }
}

struct Scene {
    dimension: XDimension2D,
    ops: Vec<Op>,
}

impl TextBlock for Scene {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.dimension
    }

    fn draw_u(&self, ug: &UGraphic) {
        for op in &self.ops {
            op.draw(ug);
        }
    }
}

/// Lists the primitives it receives with their absolute position, as `CompressDump.Recorder` does.
#[derive(Default)]
struct Recorder {
    lines: Vec<String>,
}

impl UGraphicBackend for Recorder {
    fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam) {
        let position = format!("{},{}", hex(at.dx), hex(at.dy));
        let line = match shape {
            UShape::Rectangle(rectangle) => format!(
                "rect {position} {} {}",
                hex(rectangle.width),
                hex(rectangle.height)
            ),
            UShape::Line { dx, dy } => format!("line {position} {} {}", hex(*dx), hex(*dy)),
            UShape::Polygon(_) => format!("polygon {position}"),
            UShape::Ellipse(_) => format!("ellipse {position}"),
            UShape::Text(text) => format!("text {position} {}", text.text),
            UShape::Path(_) => format!("path {position}"),
            UShape::Empty(_) => format!("empty {position}"),
            other => format!("{} {position}", other.java_class_name()),
        };
        self.lines.push(if param.stroke == UStroke::SIMPLE {
            line
        } else {
            format!("{line} stroke {}", hex(param.stroke.thickness))
        });
    }
}

fn compress<T: TextBlock>(
    passes: &str,
    compressed: &CompressionXorYBuilder<T>,
    out: &mut Vec<String>,
) {
    out.push(passes.to_owned());
    let dimension = compressed.calculate_dimension(&StringBounderDebug);
    out.push(format!(
        "  dimension {} {}",
        hex(dimension.width),
        hex(dimension.height)
    ));
    let min_max = compressed.get_min_max(&StringBounderDebug);
    out.push(format!(
        "  minmax {} {} {} {}",
        hex(min_max.min_x()),
        hex(min_max.min_y()),
        hex(min_max.max_x()),
        hex(min_max.max_y())
    ));
    let recorder = Rc::new(RefCell::new(Recorder::default()));
    compressed.draw_u(&UGraphic::new(
        recorder.clone(),
        Rc::new(StringBounderDebug),
        HColor::BLACK,
    ));
    out.extend(
        recorder
            .borrow()
            .lines
            .iter()
            .map(|line| format!("  {line}")),
    );
}

/// The fixture's lines for each scene: the scene, then what compressing it gives.
fn scenes() -> Vec<Vec<&'static str>> {
    let mut result: Vec<Vec<&str>> = Vec::new();
    for line in FIXTURE.lines().filter(|line| !line.starts_with('#')) {
        if line.starts_with("scene") {
            result.push(Vec::new());
        }
        result.last_mut().unwrap().push(line);
    }
    result
}

#[test]
fn compression_matches_plantuml_bit_for_bit() {
    let scenes = scenes();
    assert_eq!(scenes.len(), 150);
    for expected in scenes {
        let header: Vec<&str> = expected[0].split_whitespace().collect();
        let scene = Scene {
            dimension: XDimension2D::new(number(header[1]), number(header[2])),
            ops: expected[1..]
                .iter()
                .take_while(|line| line.starts_with(' '))
                .map(|line| Op::parse(line))
                .collect(),
        };
        let mut actual: Vec<String> = expected[..=scene.ops.len()]
            .iter()
            .map(|line| (*line).to_owned())
            .collect();
        compress(
            "ON_X",
            &CompressionXorYBuilder::build(CompressionMode::OnX, &scene),
            &mut actual,
        );
        compress(
            "ON_Y",
            &CompressionXorYBuilder::build(CompressionMode::OnY, &scene),
            &mut actual,
        );
        compress(
            "ON_X ON_Y",
            &CompressionXorYBuilder::build(
                CompressionMode::OnY,
                CompressionXorYBuilder::build(CompressionMode::OnX, &scene),
            ),
            &mut actual,
        );
        assert_eq!(actual.join("\n"), expected.join("\n"), "{}", expected[0]);
    }
}

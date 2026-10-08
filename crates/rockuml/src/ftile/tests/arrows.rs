//! Replays `tools/oracle/activity-unit/ArrowDump.java`: merged worms, the offsets of multi-colour arrows and
//! chains of tile geometry operations must come out bit for bit as in PlantUML.

use crate::ftile::{Arrows, FtileGeometry, MergeStrategy, Worm, WormMutation};
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UStroke;

const FIXTURE: &str = include_str!("../../../tests/data/activity-arrows.txt");

fn number(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn point(pair: &str) -> XPoint2D {
    let (x, y) = pair.split_once(',').unwrap();
    XPoint2D::new(number(x), number(y))
}

/// The points after a line's one-letter tag.
fn points(line: &str, tag: &str) -> Vec<XPoint2D> {
    let rest = line.strip_prefix(tag).unwrap();
    rest.split_whitespace().map(point).collect()
}

fn worm(points: &[XPoint2D]) -> Worm {
    let mut result = Worm::new(UStroke::SIMPLE, Arrows::Regular);
    for &point in points {
        result.add_point_at(point);
    }
    result
}

fn points_of(worm: &Worm) -> Vec<XPoint2D> {
    (0..worm.size()).map(|i| worm.get_point(i)).collect()
}

fn geometry(fields: &[&str]) -> FtileGeometry {
    let geometry = FtileGeometry::with_out(
        number(fields[0]),
        number(fields[1]),
        number(fields[2]),
        number(fields[3]),
        number(fields[4]),
    );
    assert_eq!(geometry.has_point_out().to_string(), fields[5]);
    geometry
}

fn describe(geometry: FtileGeometry) -> [u64; 5] {
    [
        geometry.get_width(),
        geometry.get_height(),
        geometry.get_left(),
        geometry.get_in_y(),
        geometry.get_out_y(),
    ]
    .map(f64::to_bits)
}

fn apply(geometry: FtileGeometry, operation: &str) -> FtileGeometry {
    let words: Vec<&str> = operation.split_whitespace().collect();
    let a = || number(words[1]);
    let b = || number(words[2]);
    match words[0] {
        "incHeight" => geometry.inc_height(a()),
        "addTop" => geometry.add_top(a()),
        "addBottom" => geometry.add_bottom(a()),
        "incRight" => geometry.inc_right(a()),
        "incLeft" => geometry.inc_left(a()),
        "incVertically" => geometry.inc_vertically(a(), b()),
        "incInY" => geometry.inc_in_y(a()),
        "withoutPointOut" => geometry.without_point_out(),
        "translate" => geometry.translate(UTranslate::new(a(), b())),
        "addDim" => geometry.add_dim(a(), b()),
        "fixedHeight" => geometry.fixed_height(a()),
        "appendBottom" => geometry.append_bottom(self::geometry(&words[1..])),
        other => panic!("unknown operation {other}"),
    }
}

#[test]
fn arrows_and_geometry_match_plantuml_bit_for_bit() {
    let mut lines = FIXTURE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .peekable();
    let mut cases = 0;
    while let Some(header) = lines.next() {
        cases += 1;
        let words: Vec<&str> = header.split_whitespace().collect();
        match words[0] {
            "merge" => {
                let strategy = match words[1] {
                    "FULL" => MergeStrategy::Full,
                    _ => MergeStrategy::Limited,
                };
                let (dx, dy) = (number(words[2]), number(words[3]));
                let first = worm(&points(lines.next().unwrap(), "a"));
                let second = worm(&points(lines.next().unwrap(), "b"));
                let expected = points(lines.next().unwrap(), "r");
                let merged = first
                    .move_by(dx, dy)
                    .merge(&second.move_by(dx, dy), strategy)
                    .move_by(-dx, -dy);
                assert_eq!(points_of(&merged), expected, "{header}");
            }
            "mutation" => {
                let delta = number(words[1]);
                let colors: usize = words[2].parse().unwrap();
                let worm = worm(&points(lines.next().unwrap(), "w"));
                let summary = lines.next().unwrap();
                let mutation = WormMutation::create(&worm, delta);
                if summary == "fails" {
                    // PlantUML fails on worms going straight on; here they are drawn somehow.
                    mutation.mute(&worm);
                    continue;
                }
                let muted = points(lines.next().unwrap(), "p");
                let first = mutation.get_first();
                let last = mutation.get_last();
                let hex = |value: f64| format!("{:016x}", value.to_bits());
                let actual = format!(
                    "m {} {},{} {},{} {} {}",
                    mutation.size(),
                    hex(first.dx),
                    hex(first.dy),
                    hex(last.dx),
                    hex(last.dy),
                    hex(mutation.get_text_translate(colors).dx),
                    mutation.is_dx_negative()
                );
                assert_eq!(actual, summary, "{header}");
                assert_eq!(points_of(&mutation.mute(&worm)), muted, "{header}");
            }
            "geometry" => {
                let mut current = geometry(&words[1..]);
                while lines.peek().is_some_and(|line| {
                    !line.starts_with("merge")
                        && !line.starts_with("mutation")
                        && !line.starts_with("geometry")
                }) {
                    let operation = lines.next().unwrap();
                    let result: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
                    current = apply(current, operation);
                    let expected = geometry(&result[1..]);
                    assert_eq!(
                        describe(current),
                        describe(expected),
                        "{header} {operation}"
                    );
                    assert_eq!(current.has_point_out(), expected.has_point_out());
                }
            }
            other => panic!("unknown case {other}"),
        }
    }
    assert_eq!(cases, 1000);
}

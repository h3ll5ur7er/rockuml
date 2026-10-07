//! Replays Smetana's path planner on the cases in data/pathplan.txt (made by tools/oracle/smetana-unit/pathplan.sh)
//! and requires bit-identical results.

use std::panic::{self, AssertUnwindSafe};

use smetana::internals::pathplan::{
    PathplanContext, PathplanError, Pedge_t, Ppoint_t, Ppoly_t, Ppolyline_t, Proutespline,
    Pshortestpath, solve3,
};

const FIXTURE: &str = include_str!("data/pathplan.txt");

fn double(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).expect("hex double"))
}

fn doubles(hexes: &[&str]) -> Vec<f64> {
    hexes.iter().map(|h| double(h)).collect()
}

fn points(hexes: &[&str]) -> Vec<Ppoint_t> {
    hexes
        .chunks(2)
        .map(|xy| Ppoint_t {
            x: double(xy[0]),
            y: double(xy[1]),
        })
        .collect()
}

/// `<count> <x y>...`; an edge counts as two points.
fn counted_points(fields: &[&str], points_per_item: usize) -> Vec<Ppoint_t> {
    let n: usize = fields[0].parse().expect("count");
    assert_eq!(fields.len(), 1 + 2 * points_per_item * n, "count matches");
    points(&fields[1..])
}

fn bits(points: &[Ppoint_t]) -> Vec<(u64, u64)> {
    points
        .iter()
        .map(|p| (p.x.to_bits(), p.y.to_bits()))
        .collect()
}

/// The polygon's sides, the barriers routespl gives `Proutespline`.
fn sides(poly: &[Ppoint_t]) -> Vec<Pedge_t> {
    let n = poly.len();
    (0..n)
        .map(|i| Pedge_t {
            a: poly[i],
            b: poly[(i + 1) % n],
        })
        .collect()
}

enum Shortest {
    Path(Vec<Ppoint_t>),
    Error(PathplanError),
    /// Smetana throws `UNSUPPORTED` with this key.
    Unsupported(String),
}

/// Either a polygon to route through (`poly`, `eps`, `shortest`), or a polyline and barriers for `Proutespline`
/// alone (`input`, `edges`).
#[derive(Default)]
struct Case {
    line: usize,
    label: String,
    poly: Vec<Ppoint_t>,
    eps: [Ppoint_t; 2],
    shortest: Option<Shortest>,
    input: Vec<Ppoint_t>,
    edges: Vec<Pedge_t>,
    evs: [Ppoint_t; 2],
    route: Option<Vec<Ppoint_t>>,
}

fn routing_cases() -> Vec<Case> {
    let mut cases: Vec<Case> = Vec::new();
    for (index, text) in FIXTURE.lines().enumerate() {
        let fields: Vec<&str> = text.split_whitespace().collect();
        let (kind, rest) = (fields[0], &fields[1..]);
        if kind == "case" {
            cases.push(Case {
                line: index + 1,
                label: rest[0].to_string(),
                ..Case::default()
            });
            continue;
        }
        let Some(case) = cases.last_mut() else {
            continue;
        };
        match kind {
            "poly" => case.poly = counted_points(rest, 1),
            "eps" => case.eps = points(rest).try_into().expect("two endpoints"),
            "input" => case.input = counted_points(rest, 1),
            "edges" => {
                case.edges = counted_points(rest, 2)
                    .chunks(2)
                    .map(|ab| Pedge_t { a: ab[0], b: ab[1] })
                    .collect();
            }
            "evs" => case.evs = points(rest).try_into().expect("two end vectors"),
            "shortest" => {
                case.shortest = Some(match rest[0] {
                    "0" => Shortest::Path(counted_points(&rest[1..], 1)),
                    "-1" => Shortest::Error(PathplanError::DestinationOutside),
                    "-2" => Shortest::Error(PathplanError::TriangulationFailed),
                    "throws" => {
                        assert_eq!(
                            rest[1],
                            "UnsupportedOperationException",
                            "line {}",
                            index + 1
                        );
                        Shortest::Unsupported(rest[2].to_string())
                    }
                    other => panic!("line {}: unknown result {other}", index + 1),
                });
            }
            "route" => {
                assert_eq!(rest[0], "0", "line {}: Proutespline succeeds", index + 1);
                case.route = Some(counted_points(&rest[1..], 1));
            }
            _ => {}
        }
    }
    cases
}

/// Runs `f`, returning its panic message if it panics, without printing the panic.
fn panic_message<R>(f: impl FnOnce() -> R) -> Option<String> {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    panic::set_hook(hook);
    let payload = result.err()?;
    Some(
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
            .unwrap_or_default(),
    )
}

/// Runs `Pshortestpath` on the case's polygon and returns the path, if Smetana found one and this port the same.
fn shortest_path(
    zz: &mut PathplanContext,
    case: &Case,
    expected: &Shortest,
    failures: &mut Vec<String>,
) -> Option<Ppolyline_t> {
    let id = format!("line {} ({})", case.line, case.label);
    let poly = Ppoly_t {
        ps: case.poly.clone(),
    };
    let mut path = Ppolyline_t::default();
    match expected {
        Shortest::Unsupported(key) => {
            match panic_message(|| Pshortestpath(zz, &poly, case.eps, &mut path)) {
                Some(message) if message.contains(key.as_str()) => {}
                other => failures.push(format!("{id}: expected UNSUPPORTED {key}, got {other:?}")),
            }
            // The Java harness also starts afresh after a throw.
            *zz = PathplanContext::default();
            None
        }
        Shortest::Error(error) => {
            let actual = Pshortestpath(zz, &poly, case.eps, &mut path);
            if actual != Err(*error) {
                failures.push(format!("{id}: expected {error:?}, got {actual:?}"));
            }
            None
        }
        Shortest::Path(points) => {
            let actual = Pshortestpath(zz, &poly, case.eps, &mut path);
            if actual.is_ok() && bits(&path.ps) == bits(points) {
                Some(path)
            } else {
                failures.push(format!(
                    "{id}: shortest path {actual:?} {:?} != {points:?}",
                    path.ps
                ));
                None
            }
        }
    }
}

/// The cases run in order on one context, as a layout's calls do, so the scratch arrays carry over between them.
#[test]
fn routes_like_smetana() {
    let cases = routing_cases();
    let mut zz = PathplanContext::default();
    let mut failures = Vec::new();
    let mut shortest_paths = 0;
    let mut splines = 0;
    for case in &cases {
        let (path, edges) = match &case.shortest {
            Some(expected) => {
                shortest_paths += 1;
                match shortest_path(&mut zz, case, expected, &mut failures) {
                    Some(path) => (path, sides(&case.poly)),
                    None => continue,
                }
            }
            None => (
                Ppolyline_t {
                    ps: case.input.clone(),
                },
                case.edges.clone(),
            ),
        };
        let Some(expected) = &case.route else {
            continue;
        };
        splines += 1;
        let mut spline = Ppolyline_t::default();
        Proutespline(&mut zz, &edges, &path, case.evs, &mut spline);
        if bits(&spline.ps) != bits(expected) {
            failures.push(format!(
                "line {} ({}): spline {:?} != {expected:?}",
                case.line, case.label, spline.ps
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
    assert!(
        shortest_paths > 150,
        "only {shortest_paths} polygons checked"
    );
    assert!(splines > 200, "only {splines} splines checked");
}

/// The roots, bit for bit: `jmath`'s correctly rounded `cos` and `pow` agree with Java's intrinsics on these.
#[test]
fn solves_cubics_like_smetana() {
    let mut failures = Vec::new();
    let mut count = 0;
    for (index, text) in FIXTURE.lines().enumerate() {
        let Some(rest) = text.strip_prefix("solve3 ") else {
            continue;
        };
        count += 1;
        let fields: Vec<&str> = rest.split_whitespace().collect();
        let coeff: [f64; 4] = doubles(&fields[..4]).try_into().expect("four coefficients");
        assert_eq!(fields[4], "=>");
        let expected_rootn: usize = fields[5].parse().expect("root count");
        let expected = doubles(&fields[6..]);
        let mut solved = [0.0; 3];
        let rootn = solve3(&coeff, &mut solved);
        let actual = if rootn < 4 { &solved[..rootn] } else { &[] };
        let exact = actual
            .iter()
            .map(|r| r.to_bits())
            .eq(expected.iter().map(|r| r.to_bits()));
        if rootn != expected_rootn || !exact {
            failures.push(format!(
                "line {}: {rootn} {actual:?} != {expected_rootn} {expected:?}",
                index + 1
            ));
        }
    }
    assert!(count >= 100, "fixture has {count} cubics");
    assert!(
        failures.is_empty(),
        "{} of {count} cubics differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

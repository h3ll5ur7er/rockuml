//! Acceptance tests against the golden model: rockuml must reproduce PlantUML's output for every corpus case.
//!
//! Most cases can only pass once their part of PlantUML is ported, so the suite fails on regressions of
//! cases recorded in `tests/parity-passing.txt`. Run with `ROCKUML_PARITY_RECORD=1` to record new passes.

mod check;
mod corpus;
mod ratchet;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use check::Outcome;
use corpus::{Case, GoldenKind};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Default)]
struct Tally {
    passed: usize,
    total: usize,
}

#[test]
fn rockuml_reproduces_the_golden_model() {
    let rockuml = Path::new(env!("CARGO_BIN_EXE_rockuml"));
    let ratchet_path = repository_root().join("tests/parity-passing.txt");
    let recorded_passes = ratchet::load(&ratchet_path);

    let mut passes = BTreeSet::new();
    let mut regressions = Vec::new();
    let mut tallies: BTreeMap<(String, GoldenKind), Tally> = BTreeMap::new();

    for (case, kind, outcome) in check_all_in_parallel(rockuml) {
        let area = case.id.split('/').next().unwrap().to_owned();
        let entry = (kind.extension().to_owned(), case.id.clone());
        let tally = tallies.entry((area, kind)).or_default();
        tally.total += 1;
        match outcome {
            Outcome::Pass => {
                tally.passed += 1;
                passes.insert(entry);
            }
            Outcome::Fail(reason) if recorded_passes.contains(&entry) => {
                regressions.push(format!("{} {}: {reason}", entry.0, entry.1));
            }
            Outcome::Fail(_) => {}
        }
    }

    print_report(&tallies);

    let new_passes: Vec<_> = passes.difference(&recorded_passes).collect();
    if std::env::var_os("ROCKUML_PARITY_RECORD").is_some() {
        ratchet::save(
            &ratchet_path,
            &recorded_passes.union(&passes).cloned().collect(),
        );
    } else if !new_passes.is_empty() {
        println!(
            "{} newly passing; record them with ROCKUML_PARITY_RECORD=1",
            new_passes.len()
        );
    }

    assert!(
        regressions.is_empty(),
        "regressions:\n{}",
        regressions.join("\n")
    );
}

/// Every (case, kind) the golden model has output for, checked on all cores.
fn check_all_in_parallel(rockuml: &Path) -> Vec<(Case, GoldenKind, Outcome)> {
    let work: Vec<(Case, GoldenKind)> = corpus::discover(&repository_root().join("tests/corpus"))
        .into_iter()
        .flat_map(|case| GoldenKind::ALL.map(|kind| (case.clone(), kind)))
        .collect();
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, usize::from);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some((case, kind)) = work.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Some(outcome) = check::check(rockuml, case, *kind) {
                        results.lock().unwrap().push((case.clone(), *kind, outcome));
                    }
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by(|(left, left_kind, _), (right, right_kind, _)| {
        (&left.id, left_kind).cmp(&(&right.id, right_kind))
    });
    results
}

fn print_report(tallies: &BTreeMap<(String, GoldenKind), Tally>) {
    println!("{:<24} {:<8} passing", "area", "kind");
    for ((area, kind), tally) in tallies {
        println!(
            "{area:<24} {:<8} {:>5} / {:<5}",
            kind.extension(),
            tally.passed,
            tally.total
        );
    }
}

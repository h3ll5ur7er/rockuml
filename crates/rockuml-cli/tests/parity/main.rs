//! Acceptance tests against the golden model: rockuml must reproduce PlantUML's output for every corpus case.
//!
//! Most cases can only pass once their part of PlantUML is ported, so the suite fails on regressions of
//! cases recorded in `tests/parity-passing.txt`. Run with `ROCKUML_PARITY_RECORD=1` to record new passes.

mod check;
mod corpus;
mod ratchet;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use check::Outcome;
use corpus::GoldenKind;

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

    for case in corpus::discover(&repository_root().join("tests/corpus")) {
        let area = case.id.split('/').next().unwrap().to_owned();
        for kind in GoldenKind::ALL {
            let Some(outcome) = check::check(rockuml, &case, kind) else {
                continue;
            };
            let entry = (kind.extension().to_owned(), case.id.clone());
            let tally = tallies.entry((area.clone(), kind)).or_default();
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

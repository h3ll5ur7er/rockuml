//! The `@startdot` cases (`tests/corpus/dot`) hand their source to an external Graphviz `dot`, so they can only
//! match the goldens when rockuml finds the `dot` the goldens were made with: the one
//! `tools/oracle/setup-graphviz.sh` unpacks, which measures text with fonts of its own. rockuml finds it like
//! PlantUML, through `GRAPHVIZ_DOT` or else on the `PATH`.

use std::path::Path;

use crate::check::{self, Outcome};
use crate::corpus::{Case, GoldenKind};

/// Set where the cases must run, as in CI: then not finding that Graphviz fails the suite instead of
/// skipping them.
pub(crate) const REQUIRED: &str = "ROCKUML_PARITY_REQUIRE_GRAPHVIZ";

/// The case whose SVG tells whether rockuml runs the goldens' Graphviz.
const PROBE: &str = "dot/digraph.puml";

pub(crate) fn runs_graphviz(case: &Case) -> bool {
    case.id.starts_with("dot/")
}

/// Why rockuml cannot reproduce the Graphviz cases, if it cannot.
pub(crate) fn matches_the_goldens(rockuml: &Path, cases: &[Case]) -> Result<(), String> {
    let probe = cases
        .iter()
        .find(|case| case.id == PROBE)
        .ok_or_else(|| format!("{PROBE} is missing"))?;
    match check::check(rockuml, probe, GoldenKind::Svg) {
        Some(Outcome::Pass) => Ok(()),
        Some(Outcome::Fail(difference)) => Err(format!(
            "the dot rockuml finds is not the goldens' (see tools/oracle/setup-graphviz.sh): {PROBE} {difference}"
        )),
        None => Err(format!("{PROBE} has no SVG golden")),
    }
}

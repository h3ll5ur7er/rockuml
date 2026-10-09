# Working agreements for rockuml

rockuml ports PlantUML 1.2026.8 (Java, `reference/plantuml-lgpl-1.2026.8-sources`) to Rust. Read [PLAN.md](PLAN.md)
first.

## Workflow
- One feature branch per plan phase (`phase-N-<topic>`), merged into `main` only after a critical self-review.
- TDD: write the failing test first. Acceptance tests are corpus cases with goldens from the Java golden model
  (`tools/oracle`). Unit tests cover the pieces in between.
- A phase is done when its parity numbers meet the exit criteria in PLAN.md, `cargo clippy --all-targets` is clean,
  `cargo fmt` has run, and the passing cases are recorded in `tests/parity-passing.txt`.
- The website (`site/`) is checked with `npm test` and `npm run format:check` in `site/`. New or changed features
  get documented there, with live examples.

## Code
- Port faithfully: output compatibility lives in PlantUML's arithmetic, constants and iteration order. Keep Java
  names for ported types so the two trees stay greppable. Idiomatic Rust is welcome where it can't change output.
- Clean code: SOLID, KISS, DRY, YAGNI. Port only what a test needs. Dead code is technical debt, so delete it.
- Names over comments. Comments explain *why*, never *what*.
- Refactor once the tests are green.
- The engine crate does no I/O of its own (it has to run in wasm). The CLI supplies files, environment and time.

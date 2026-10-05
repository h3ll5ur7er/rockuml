# rockuml

A PlantUML-compatible diagram renderer written in Rust: one self-contained binary for Windows, Linux and macOS
(and a WebAssembly build), with no Java and no Graphviz required.

Compatibility target: **PlantUML 1.2026.8**. See [PLAN.md](PLAN.md) for the porting plan and the definition of
"compatible".

## Build and test

```bash
cargo build --release
cargo test
```

`cargo test` includes the parity suite (`crates/rockuml-cli/tests/parity`), which runs rockuml over every case in
`tests/corpus` and compares the output with the golden files that PlantUML produced. Cases listed in
`tests/parity-passing.txt` must keep passing. To record cases that have started passing:

```bash
ROCKUML_PARITY_RECORD=1 cargo test -p rockuml-cli --test parity
```

## Adding corpus cases

Put a `.puml` file under `tests/corpus/<area>/` and generate its goldens with the reference implementation
(see [tools/oracle/README.md](tools/oracle/README.md)):

```bash
bash tools/oracle/generate-goldens.sh tests/corpus/<area>/<case>.puml
```

# rockuml

A PlantUML-compatible diagram renderer written in Rust: one self-contained binary for Windows, Linux and macOS
(and a WebAssembly build), with no Java and no Graphviz required.

Compatibility target: **PlantUML 1.2026.8**. See [PLAN.md](PLAN.md) for the porting plan and the definition of
"compatible".

## Usage

```bash
rockuml diagram.puml                    # writes diagram.svg next to it
rockuml -tpng diagram.puml              # diagram.png
rockuml -o out -f svg-deterministic a.puml b.puml
```

A diagram with several pages (`newpage`) writes one file per page: `diagram.svg`, `diagram_001.svg`, and so on.

### Supported diagrams

- Sequence diagrams (`@startuml`), laid out like PlantUML's teoz engine, the only sequence engine of 1.2026.8.
- Class, object, use case, component, deployment, archimate and state diagrams (`@startuml`) and Chen ER diagrams
  (`@startchen`), laid out by `crates/smetana`, a bit-exact port of PlantUML's Smetana (its Java translation of
  Graphviz `dot`), so no Graphviz installation is needed.
- Activity diagrams in the current syntax (`:action;`, `if`/`switch`/`while`/`repeat`, `fork`/`split`, partitions,
  notes, swimlanes, `goto`/`label`, `detach`/`kill`), laid out like PlantUML's `ftile` engine. The legacy syntax
  (`(*) --> "action"`) is not ported yet.
- Salt wireframes (`@startsalt`) and creole text (`@startcreole`).

Their text takes PlantUML's creole markup, including sprites (`sprite $name …`, `<$name>`, the stdlib and built-in
archimate sprites), images (`<img:file.png>`, data URIs and URLs) and emoji (`<:smile:>`). Images named by a path are
read relative to the diagram file; like PlantUML, rockuml refuses system paths such as `/etc/` unless
`PLANTUML_SECURITY_PROFILE` says otherwise.

For other diagram types rockuml reports that they are not ported yet and exits with status 1; the phases in
[PLAN.md](PLAN.md) say when they come.

### Fonts

Text is measured with embedded fonts: the Liberation fonts, which have the metrics of Arial, Times New Roman and
Courier New, the fonts PlantUML uses on Windows. Images therefore look the same on every machine and match PlantUML
run on Windows.

To use other fonts, register their files and name them in the diagram as usual (`skinparam defaultFontName`,
`<font:...>`, styles):

```bash
rockuml --font ~/fonts/Inter.ttf --font ~/fonts/more/ diagram.puml
```

`--font` takes a `.ttf`, `.otf` or `.ttc` file, or a directory of them, and can be repeated. The `ROCKUML_FONTS`
environment variable holds more such paths, separated like `PATH`. Fonts are registered under their family names; a
family nobody registered falls back to the default sans-serif font, as in PlantUML.

`-f svg-deterministic` measures text with PlantUML's built-in width table instead of fonts.

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

## Web / wasm

rockuml also runs in the browser, as `rockuml.wasm` with a small JavaScript module, `web/rockuml.js`, and no other
runtime. Build it (needs the `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`):

```bash
bash tools/build-wasm.sh          # writes web/rockuml.wasm
```

Try it in the demo page, an editor with a live preview:

```bash
cd web
python -m http.server             # then open http://localhost:8000
```

Use it from your own page or from Node:

```js
import { load, RockumlError } from './rockuml.js';

const rockuml = await load();     // fetches rockuml.wasm next to rockuml.js; or pass a URL, bytes or a WebAssembly.Module
const { data, pageCount, isError } = await rockuml.render(source, { format: 'svg', page: 0 });
```

`format` is `svg`, `png`, `svg-deterministic`, `debug` or `preproc`; `data` is a string, or a `Uint8Array` for PNG.
`page` counts the pages of all diagrams in the source, in the order the CLI writes their files. `isError` says the
image shows the diagram's errors. Sources without a diagram, pages past the last, and diagram types rockuml has not
ported yet throw a `RockumlError` (with `notPorted` set for the latter). In Node, pass the bytes:
`load(await readFile('web/rockuml.wasm'))`.

The wasm build has no files, URLs or environment variables, so `!include` of files and URLs fails; dates are local
time. To keep the download small it leaves out the standard library (`!include <C4/...>` fails like a library that
does not exist) and the emoji: these are the engine's `stdlib` and `emoji` cargo features, on by default.

Its tests render corpus cases and compare them with the goldens:

```bash
node --test web/rockuml.test.mjs
```

## Adding corpus cases

Put a `.puml` file under `tests/corpus/<area>/` and generate its goldens with the reference implementation
(see [tools/oracle/README.md](tools/oracle/README.md)):

```bash
bash tools/oracle/generate-goldens.sh tests/corpus/<area>/<case>.puml
```

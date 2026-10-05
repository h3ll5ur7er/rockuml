# rockuml — porting plan

Goal: a single self-contained executable (Windows, Linux, macOS) plus a WebAssembly build that reads the
same `.puml` input as PlantUML 1.2026.8 and produces the same output. No JRE and no Graphviz install.

Reference: `reference/plantuml-lgpl-1.2026.8-sources` (abbreviated `REF`; `P` = `REF/net/sourceforge/plantuml`).

---

## 1. What we are porting (analysis summary)

### Size
| | Files | Raw lines | Code lines (no blank, no comment) |
|---|---|---|---|
| All Java | 3,128 | 618k | **~232k** |
| Each file has a ~100-line licence header, so raw counts overstate the size by about 2×. |

Largest packages (raw lines): `klimt` 69k (graphics and text), `activitydiagram3` 48k, `sequencediagram` 35k,
`svek` 33k (Graphviz glue), `gantt` 31k, `tim` 23k (preprocessor), `command` 15k, `skin` 13k, `timingdiagram` 12k.
Smetana (Graphviz 2.38 `dot` ported to Java, in `REF/gen`, `REF/h`, `REF/smetana`) is 60k raw lines. About 60% of its
functions are `UNSUPPORTED` stubs, which leaves roughly 25–35k lines of real logic.

### The pipeline
```
source text
 └─ BlockUmlBuilder ......... cuts @startX/@endX blocks, joins lines ending in \, injects -config
     └─ tim.TimLoader ....... preprocessor: !define, !function, !include, !theme, 76 %builtins, JSON values
         └─ PSystemBuilder .. tries 52 factories in order; the first result that is not an error wins
             └─ PSystemCommandFactory: each line is matched against Command regexes (SingleLine / Multilines)
                 └─ Diagram model (SequenceDiagram, ClassDiagram, ActivityDiagram3, ...)
                     └─ layout
                     │   ├─ CucaDiagram family (class/object/usecase/component/deployment/state/chen/act-v1)
                     │   │    → svek (writes DOT, runs `dot -Tsvg`, reads coordinates back by colour tag)
                     │   │    → or sdot/Smetana (used automatically when dot is missing)
                     │   ├─ JSON/YAML/HCL/git → always Smetana
                     │   └─ everything else → its own engine (teoz, ftile, gantt solver, mindmap, ...)
                     └─ klimt UGraphic → drivers: SVG | PNG (Java2D) | EPS | TikZ | PDF | txt | DEBUG ...
```

### Findings that drive the plan
1. **Text metrics decide output compatibility.** PNG and SVG measure text with `java.awt` and the logical font
   "SansSerif". That font is Arial on Windows and DejaVu Sans on most Linux systems, so **Java PlantUML itself
   is not byte-identical across operating systems.** PlantUML has two font-independent bounders:
   - `StringBounderFromWidthTable` (`klimt/drawing/font`, `SVG_DETERMINISTIC`): a portable Unicode width table.
   - `StringBounderDebug` (`DEBUG` format): `size × len × random(0.8..1.3)`, where the random factor comes from
     `java.util.Random` seeded by the text.

   The DEBUG output (`UGraphicDebug`) is a plain-text dump of every drawn primitive with its coordinates.
   **It is the best regression oracle available: it does not depend on fonts and it is exact.**
2. **Graphviz is replaceable.** PlantUML already falls back to Smetana silently when `dot` is absent. Smetana
   is a line-by-line C→Java translation: functions keep their C names (`xxx__c.java`), C structs become `ST_*`
   classes, and global state sits in a `Globals` object. That style ports to Rust mechanically. It covers the
   `dot` layout only. Missing: `splines=ortho`, HTML labels, concentrate, newrank.
3. **The regex dialect is manageable.** There are about 1,600 `RegexLeaf`s. Lookarounds appear in only about 12 places,
   including one negative lookbehind. There are no backreferences, possessive quantifiers or atomic groups. Everything
   is compiled `CASE_INSENSITIVE`, with macros `%s %q %g %pLN`. A second home-grown engine, `com/plantuml/ubrex`
   (6k lines, used by 72 files), has to be ported too. Java `\w \d \b` are **ASCII-only**, while Rust's are Unicode.
4. **The resources are plain data.** `stdlib/*.spm` is Brotli-compressed records (8.8 MB, the C4/AWS/Azure/... libraries).
   `themes/*.puml` holds 44 themes, `skin/*.skin` holds CSS-like defaults, and there are also sprites, OpenIconic SVGs,
   1,174 Twemoji SVGs and SVG js/css. All of it can be embedded with `include_bytes!`.
5. **External jars that are not in the tree:** ELK, OpenPDF, JLaTeXMath, Batik, viz.js/J2V8, zxing, TeaVM, Ant.
   Each needs a replacement or a deliberate drop (§5).
6. **There is very little reflection** and none on the parse path: factories and commands are created with explicit `new`.
7. **Prior art inside the tree:** a GraalVM `native-image` config (`REF/META-INF/native-image`) and a TeaVM browser build
   (`P/teavm`, which uses viz.js for layout). See §9.

---

## 2. Language choice: **Rust**

| | Rust | C++ | Swift | Python |
|---|---|---|---|---|
| Single static binary on all 3 OSes | ✅ `cargo build` + cross | ✅ but toolchain/CMake pain | ⚠️ Windows second-class | ❌ needs a runtime (the thing we want to avoid) |
| Wasm | ✅ first-class (`wasm32-unknown-unknown`, wasm-bindgen) | ⚠️ Emscripten | ⚠️ experimental | ❌ (Pyodide = 10 MB+ runtime) |
| Ready-made crates for our gaps | ✅ (see below) | partial | few | n/a |
| Memory/thread safety while porting 230k lines | ✅ | ❌ | ✅ | ✅ |

Crates that close the gaps:

| Need | Crate |
|---|---|
| Java-style regex with lookarounds | `fancy-regex` (wrapping `regex`) |
| Brotli (stdlib .spm) | `brotli-decompressor` |
| Raw deflate (URL encoding, `z` sprites) | `miniz_oxide` |
| Font parsing and metrics | `ttf-parser` (+ `rustybuzz` if shaping is ever needed) |
| SVG → PNG rasterisation | `resvg` / `usvg` / `tiny-skia` |
| PNG chunks (embedded source, DPI) | `png` |
| SVG → PDF | `svg2pdf` (typst) |
| Raster image decoding (`<img>`) | `image` (png/jpeg/gif/webp) |
| JSON / YAML for preprocessor and diagrams | `serde_json`, hand port of PlantUML's YAML subset (keeps its quirks) |
| QR codes (`<qrcode:>`, missing in the LGPL build) | `qrcodegen` |
| Hashing / misc | `sha1_smol`, `indexmap` (for Java `LinkedHashMap` order) |
| CLI | `clap` (with a custom layer for PlantUML's `-tsvg`-style flags) |
| Wasm bindings | `wasm-bindgen` |

---

## 3. What "compatible" means (be explicit, because byte-identical is impossible)

| Level | Definition | Target |
|---|---|---|
| **Input** | Every valid core PlantUML file renders and every invalid one fails, including preprocessor, stdlib, themes, skinparam, `<style>` | **100% for Tier 1+2 diagrams** |
| **Errors** | Same error message and line number; same red error image layout | Tier 1 |
| **Geometry** | `-f debug` output **identical** to Java's | Tier 1, the main CI gate |
| **Deterministic SVG** | `SVG_DETERMINISTIC` output identical after normalising version/date comments | Tier 1 |
| **Default SVG** | Same structure and ids; coordinates within ε of Java-on-Windows | Tier 1 |
| **PNG** | Same pixel dimensions, visually equivalent (antialiasing differs from Java2D) | Tier 1 |
| **CLI** | Same flags, file naming, `-pipe`, `-o`, globbing, exit codes, `-checkonly`, `-encodeurl` | Tier 1 |
| **Layout engine** | Matches Java **without** Graphviz installed (Smetana path). Optionally matches Java *with* dot when the user has `dot` on PATH | default / optional |

### Fonts: our deliberate improvement
rockuml **embeds** its fonts:
- Liberation Sans (metric-compatible with Arial, which is what Java uses on Windows)
- Liberation Mono and Serif
- DejaVu Sans as a fallback for wider Unicode coverage

Measurement uses `ttf-parser` advances with fractional metrics, which is how Java measures (`FRACTIONALMETRICS ON`).
PNG rendering uses the **same** embedded fonts through resvg.

The result: output is identical on every OS and in the browser, and it matches Java-on-Windows. A user who sets
`skinparam defaultFontName` to a font we don't embed gets it resolved through `fontdb` (system fonts, native builds only),
with a fallback to Liberation.

Three bounders are selectable, mirroring Java:
- `font` (default)
- `table` (`SVG_DETERMINISTIC`)
- `debug`

---

## 4. Architecture of rockuml

Cargo workspace:

```
rockuml/
  crates/
    jcompat/        Java-semantics helpers: UTF-16 string ops (length/substring/charAt as Java),
                    java.util.Random LCG, Double.toString / String.format("%.4f") clones,
                    StrictMath (fdlibm via `libm`), case-insensitive ASCII rules
    ru-assets/      include_bytes! of stdlib/*.spm, themes, skins, sprites, openiconic, emoji, svg js/css, fonts
                    (cargo features to drop stdlib/emoji for a small wasm build)
    ru-regex/       Pattern2 macro expansion → fancy-regex; RegexLeaf/Concat/Or/Optional with named-group mapping;
                    FoxSignature prefilter; port of com/plantuml/ubrex
    ru-preproc/     block extraction + tim preprocessor + stdlib (.spm) reader + theme loader
    ru-core/        Diagram trait, PSystemBuilder, Command framework, BlocLines, errors, UmlSource,
                    security profile, URL text codec (P/code)
    ru-style/       skin parser, StyleBuilder/StyleSignature resolution, SkinParam, FromSkinparamToStyle
    ru-klimt/       geometry, colours (HColor, 154 named colours, HSLuv), UGraphic + UShape model, TextBlock family,
                    font stack + StringBounders, creole engine, sprites, decoration/extremities
    ru-render-svg/  SvgGraphics clone (hand-written XML, ids, js/css injection, source embedding)
    ru-render-debug/
    ru-render-png/  rasterise our own SVG with resvg; add iTXt `plantuml` source chunk and pHYs DPI
    ru-render-*/    later: pdf (svg2pdf), eps, tikz, txt
    ru-dot/         Rust port of Smetana (dotgen, common, cgraph-lite, pathplan, label, pack)
    ru-svek/        CucaDiagram layout glue: node images, clusters, edge/label placement; LayoutEngine trait with
                    impls Smetana (built-in) and ExternalDot (`dot -Tsvg` + the svek SVG back-parser)
    ru-diagrams/    one module per family: sequence (teoz + `real` solver), class, description, state, activity3,
                    mindmap, wbs, gantt, timing, json/yaml, nwdiag, salt, ...
    rockuml-cli/    the `rockuml` binary (PlantUML-compatible CLI, -pipe, later --http-server)
    rockuml-wasm/   wasm-bindgen API: render(source, format, options) -> bytes/string; tiny demo page
  tests/
    corpus/         .puml inputs
    golden/         Java outputs: .debug, .det.svg, .svg, .png (dims), .preproc, .err
  tools/
    oracle/         scripts that run the reference jar over the corpus
```

### How the Java maps to Rust
- **Transliterate first, idiomatise later.** Output compatibility lives in the arithmetic: integer font-size casts,
  margin constants, order of float operations, iteration order. Keep Java class names as Rust type names and keep
  method structure 1:1, with a `// REF: path/Class.java` comment per type. Grep-ability between the two trees is worth
  more than elegance.
- **Interfaces → traits; small class hierarchies → enums.** `TextBlock` and `UGraphic` become traits (`Box<dyn ...>`).
  `UShape` becomes an enum. `HColor` becomes an enum.
- **UGraphic decorators** (`ug.apply(UTranslate)` returns a new UGraphic) become a cheap `Clone` value:
  `{ param, translate, clip, driver: Rc<RefCell<dyn Driver>> }`.
- **Object graphs** (entities, links, groups, Smetana nodes and edges) become arenas with typed `Id`s, not `Rc<RefCell>`.
  Smetana's `Globals zz` becomes a `&mut LayoutCtx`, so the static lock goes away and layout becomes re-entrant.
- **Collections:** `LinkedHashMap`/`LinkedHashSet` → `indexmap`. Audit every `HashMap` iteration that affects output
  and replicate Java's order where it matters (rare; flag each case).
- **Strings:** most text is ASCII and Rust `String` works. Anywhere Java semantics leak into output (`%strlen`,
  `%substr`, `text.length()` in bounders, regex offsets), go through `jcompat::Utf16`.
- **No I/O in the core.** `trait Host { read_file, fetch_url, env, now, ... }`. The CLI implements it with std. Wasm
  implements it with an in-memory filesystem plus optional fetch. This also carries the PlantUML security-profile semantics.

---

## 5. Scope tiers

| Tier | Contents | Notes |
|---|---|---|
| **0 – foundation** | CLI basics, block extraction, preprocessor, stdlib, themes, regex/command framework, style/skin, klimt, creole, SVG + DEBUG backends, error diagrams, URL encode/decode | everything else depends on these |
| **1 – core UML** | sequence (teoz), class/object, usecase/component/deployment, state, activity v3, Smetana layout, PNG output, wasm build | ~90% of real-world use |
| **2 – popular extras** | mindmap, wbs, gantt, timing, json, yaml, nwdiag, salt, chen ER, PDF (svg2pdf), `-pipe` / `-o` / dir globbing parity, http server mode | the http server makes the VS Code/JetBrains PlantUML plugins work against rockuml (server URL `http://localhost:port`) |
| **3 – long tail** | ebnf, regex, chart, packetdiag, hcl, git, files, board, wire, bpm, flow, legacy activity v1, txt/utxt ASCII art, EPS, LaTeX/TikZ, ditaa (port the vendored `org/stathissideris`, 10k lines), xmi/scxml/graphml, external `dot` engine, `splines=ortho` (port `lib/ortho` from Graphviz C) | on demand |
| **Math** | `<math>`/`<latex>`/`@startmath` | Java needs JLaTeXMath (not in tree). Options: port the ASCIIMath→TeX converter (pure Java, 1.1k lines) plus a Rust TeX-math layout crate (evaluate ReX-style crates), or render to MathML inside SVG `foreignObject` (browser only). Decide when we get there. |
| **Drop** | easter eggs (eggs, dedication, donors, fun, charlie...), swing GUI, ftp, telnet, stats, licensing, argon2, ELK, viz.js, TeaVM, Ant task, HTML5 canvas, VDX, braille | `@startuml` + `version`/`license` should still print something sensible |

---

## 6. The test oracle (build this first)

Java is needed **only on the development machine and only to produce golden files**. Use a portable JDK zip
(unzipped into `tools/`, no installer) or the `plantuml/plantuml` Docker image, plus the official
`plantuml-lgpl-1.2026.8.jar` from the GitHub release or Maven Central (same version as `REF`). **Don't install
Graphviz.** Java then uses Smetana, which is the engine we port, so layouts are comparable 1:1.

**Corpus**, collected into `tests/corpus/`:
1. Examples from the PlantUML Language Reference Guide and plantuml.com pages for every Tier 1 and 2 diagram (several hundred).
2. PlantUML's own non-regression tests from the GitHub repo (`test/nonreg/...`). They are DEBUG-format tests, exactly our oracle.
3. Every theme × a few diagrams. Stdlib usage samples (C4, AWS, Azure, k8s, material...).
4. Preprocessor torture tests: each builtin, include variants, loops, JSON, `!function` default args.
5. Deliberately broken inputs, for error-message parity.
6. Later: a grammar-aware mutator that generates variants. Diff Java vs rockuml and keep anything that diverges.

**Comparison levels**, each a separate CI job (`cargo test -p oracle`):

| Level | Java command | Comparison |
|---|---|---|
| L0 preprocessor | `-preproc` | exact text |
| L1 geometry | `-f debug` | exact text (render timestamps masked) |
| L2 det. SVG | `SVG_DETERMINISTIC` (via `-t svg` + deterministic option / API helper in the oracle tool) | exact after normalisation |
| L3 SVG | `-tsvg` (generated on Windows) | XML-structural diff, numeric ε = 0.5 px |
| L4 PNG | `-tpng` | same width × height; perceptual diff report (not gating) |
| L5 errors | any broken input | same message and line |

A small `tools/oracle` program keeps one JVM running (`-pipe`/API) so golden generation is fast. Goldens are checked in,
so CI never needs Java. A dashboard (`cargo xtask parity`) prints the pass rate per diagram type. That number *is* the
project's progress bar.

---

## 7. Phases

Effort is shown as Java code lines to port (non-blank, non-comment), which is the honest measure of work.
Phases 3–6 can run in parallel once Phase 2 has fixed the core traits.

### Phase 0 — Bootstrap
- Install Rust (rustup) and add the `wasm32-unknown-unknown` target. Create the workspace skeleton and `jcompat`.
- CI matrix (once a remote exists): windows-msvc, linux-musl (static), macOS universal, wasm. Release artifacts are single files.
- Oracle tooling, initial corpus, golden generation.
- **Exit:** `rockuml --version` works; the library builds for wasm; parity harness and seed corpus are in place. Each later phase grows the corpus with the cases its features need.

### Phase 1 — Text front-end (~25k Java lines)
- `ReadLineReader`, `UncommentReadLine`, line merging, `-config`, `StartUtils`, `BlockUml`, YAML header removal, `jaws`.
- The full `tim` preprocessor: `TContext`, `TLineType`, the Eater classes, ShuntingYard expressions, `TValue` with JSON,
  all 76 builtins, functions and procedures, includes (file, `<stdlib>`, URL behind the security profile, `!includesub`,
  `!import` zip), `!theme`.
- Stdlib `.spm` reader (Brotli + Java `DataInputStream` / modified-UTF-8 records), sprites and images from stdlib.
- URL codec (`P/code`): `-encodeurl` / `-decodeurl` parity.
- **Exit:** L0 at 100% on the corpus. `-preproc`, `-encodeurl` and `-decodeurl` match Java.
- **Status: done.** 42/42 `-preproc` and 42/42 `-encodeurl` corpus cases match. Learned along the way:
  - The engine is one crate with modules (`tim`, `preproc`, `json`, `color`, `deflate`, `url_code`, `java`...) rather than
    the `ru-*` crates sketched in §4; split crates later only if compile times or reuse call for it.
  - Java's `Deflater` output (zlib 1.3.1, level 9) is reproduced by a port of zlib's deflate: other deflaters choose
    different matches, and the encoded source also appears in SVG/PNG metadata.
  - Deliberate deviations: where PlantUML crashes the whole file with an unchecked exception (division by zero in an
    assignment, `!include` of an unknown stdlib library), rockuml reports "Fatal parsing error" on the line instead.
    `%getenv` cannot read JVM system properties other than `path.separator`/`line.separator`.
  - The oracle runs with a fixed `en_US` locale; `%date` uses English names like it.
  - jiff needs its bundled tz database on wasm (`tzdb-bundle-always`) for named time zones in `%date`.

### Phase 2 — Rendering foundation (~45k Java lines)
- `ru-regex`: Pattern2 + RegexLeaf/Concat/... + ubrex.
- `ru-core`: Command framework, PSystemBuilder trial-and-error ordering, BlocLines, CommonCommands
  (skinparam, style, sprite, scale, title/header/footer/legend/caption, pragma, hide/show), newpage.
- `ru-style`: skin files, `<style>` parser (nesting, `:depth`, stereotypes, CSS vars, `@media` dark),
  StyleBuilder merge rules, the 159 skinparam→style conversions.
- `ru-klimt`: geometry, colours, UGraphic, shapes, TextBlock family, the three StringBounders, creole (stripes, atoms,
  tables, trees, lists, inline markup, sprites, OpenIconic, emoji, `<img>`, links), decorations and arrow extremities.
- Backends: DEBUG, SVG (both bounders), PNG via resvg.
- Error diagram (`PSystemError`) rendering, plus `@startcreole`/`@startsalt` as first end-to-end smoke diagrams.
- **Exit:** salt/creole and error images pass L1/L2.

### Phase 3 — Sequence diagrams (~20k)
- teoz (`PlayingSpace`, `LivingSpaces`, tiles), the `real` constraint solver, sequence `graphic` components,
  all ~41 commands, notes, groups, refs, dividers, delays, autonumber, boxes, return, activation, create/destroy, newpage.
- **Exit:** L1 ≥ 98% on the sequence corpus. This is the first genuinely usable release.

### Phase 4 — Graph layout: Smetana → `ru-dot` (~30k real lines)
- Port only the functions that are actually implemented. The `UNSUPPORTED` stubs become `unimplemented!()`.
  Keep C/Java function names (`dot_mincross`, `rank1`, `make_flat_edge`...).
- Port order: cdt (or replace with `BTreeMap`/`indexmap`, keeping ordering semantics) → cgraph-lite →
  common (shapes, splines, ns = network simplex, utils) → dotgen (rank, mincross, position, dotsplines, flat, cluster,
  conc, sameport) → pathplan → label (xlabels).
- Include the PlantUML `[FIX-cluster-layout]` / `[FIX-flat-label]` patches; that's why we port the Java and not upstream C.
- Unit oracle: a debug hook in both versions that dumps node coordinates and edge bezier points for a given graph,
  built from the JSON diagram (which always uses Smetana) and from class diagrams.
- **Exit:** identical coordinates on all corpus graphs (bit-exact f64, or within 1e-9).

### Phase 5 — CucaDiagram family (~40k: svek + cucadiagram + decoration + diagrams)
- The svek glue (`EntityImage*`, clusters, `SvekEdge` label placement, extremities), the `sdot` driver
  (`CucaDiagramFileMakerSmetana`, composite-state recursion), `net/atmp/CucaDiagram`.
- Class/object, description (usecase/component/deployment/archimate), state (multi-pass parser), chen ER.
- `ExternalDot` engine (optional): port svek's DOT writer and colour-tag SVG back-parser. Selected with
  `!pragma layout dot`, or automatically when `dot` is on PATH if the user opts in via config. Smetana stays the default.
- **Exit:** L1 ≥ 95% per type.

### Phase 6 — Activity v3 (~25k)
- ftile + vcompact + vertical + gtile, swimlanes, goto, notes, partitions, parallel/split, switch, repeat/while, detach.
- **Exit:** L1 ≥ 95%.

### Phase 7 — Distribution polish (Tier 1 complete)
- Full CLI parity (`-t*`, `-o`, `-pipe`, `-pipemap`, `-charset`, `-D`, `-config`, `-theme`, `-checkonly`, `-failfast2`,
  `-nbthread`, dir and glob inputs, output naming `name_001.svg` for newpages, embedded source in PNG/SVG, `-metadata`).
- Wasm package: `rockuml.wasm` + JS shim + a static demo page (editor + live SVG), all running locally in the browser.
- Release builds: `rockuml.exe`, `rockuml-linux-x86_64`, `rockuml-macos-universal`; `cargo dist` or a plain GitHub-Actions-style script.
- Binary size budget: about 15 MB with the full stdlib, about 3 MB wasm without stdlib/emoji (fetch them lazily on the web).

### Phase 8+ — Tier 2, then Tier 3, ordered by what you and your friends actually use.

---

## 8. Known risks and mitigations

| Risk | Mitigation |
|---|---|
| Java `Math.sin/cos/atan2/pow` vs Rust may differ by 1 ulp and break exact L1 | use the `libm` crate (fdlibm = Java `StrictMath`); where Java uses `Math` intrinsics, accept ε at L1 for those values only |
| Java `Double.toString` / `String.format` number formatting in SVG/DEBUG output | clone `SvgGraphics.format` and Java's shortest-repr algorithm in `jcompat` (Rust's `{}` for f64 is also shortest-repr but formats differently, e.g. `1.0E10`) |
| `HashMap`/`HashSet` iteration order leaking into output | audit during the port; reproduce Java's String hashCode + bucket order only where a test proves it matters |
| Regex semantics drift (ASCII `\w`, case folding, `find` vs `matches`) | central `Pattern2` wrapper that rewrites to `(?-u:\w)` etc.; property tests comparing against Java on all 1,600 leaf patterns, extracted mechanically |
| Sheer volume (~110k Java code lines for Tier 0+1) | strict transliteration and per-package agents working in parallel against the oracle; no redesign until parity |
| Upstream PlantUML keeps moving | pin to 1.2026.8; re-diff later only if needed (it's for personal use) |
| Font metrics differ from Java on Linux/macOS | by design: we match Java-on-Windows everywhere; `--bounder table` gives a fully deterministic mode |

---

## 9. Alternatives considered (and why we're not stopping there)
- **GraalVM native-image** of the existing jar (`REF/META-INF/native-image` already has configs): you get a native binary
  in a day, but it still uses AWT fonts (native-image AWT on Windows/macOS is fragile), produces no wasm, and the
  binary is ~100 MB. Useful as a **stopgap** to hand to friends now, not as the end state.
- **TeaVM build** (`P/teavm`): already a JS target, but it needs viz.js for layout, measures text in the browser, and
  doesn't run natively.
- **Link real Graphviz C** instead of porting Smetana: it gives the "with dot" layouts, but it brings back a C
  toolchain on Windows and a painful wasm story. The optional `ExternalDot` engine covers users who have dot anyway.

---

## 10. Immediate next steps
1. Phase 2: regex/command framework, `PSystemBuilder`, style/skin, klimt core and the DEBUG backend, starting with
   error diagrams (every unported diagram type currently fails) and `@startcreole`/`@startsalt`.
2. Grow the corpus per diagram type before porting it (examples from the PlantUML language reference).

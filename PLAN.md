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
rockuml **embeds** its fonts: Liberation Sans, Serif and Mono, metric-compatible with Arial, Times New Roman and
Courier New, which is what Java's logical fonts map to on Windows.

Measurement uses `ttf-parser` advances with fractional metrics, which is how Java measures (`FRACTIONALMETRICS ON`).
PNG rendering uses the **same** embedded fonts through resvg.

The result: output is identical on every OS and in the browser, and it matches Java-on-Windows. Users can register any
other font (`--font`, `ROCKUML_FONTS`; `FontRegistry::register` for embedders). System fonts are deliberately not
picked up automatically, so that output does not depend on the machine.

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
    rockuml-cli/    the `rockuml` binary (PlantUML-compatible CLI, -pipe, --http-server)
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
- **Status: done.** Every ported corpus case matches the golden model in all formats: debug (L1), deterministic SVG (L2),
  font-measured SVG (L3, byte for byte, except one known font-coverage case) and PNG size (L4). Done:
  - regex tree, UBrex engine, command framework and the command-factory parse loop;
  - style system: skin files, `<style>` sheets, merge priorities, skinparam→style conversion, `SkinParam`;
  - klimt core and the DEBUG backend; creole sheets with lists, headings, separators and all inline markup
    (styles, colours, sizes, fonts, sup/sub);
  - creole tables, trees, links (`<a>` in SVG), `<code>` blocks, OpenIconic icons and separators, which span their
    title, legend or note through PlantUML's stencil;
  - `@startcreole`; `@startsalt` grids, widgets, trees, tabs, menus, scroll panes and separators; error images,
    including the welcome text;
  - common commands: `skinparam`, `<style>`, `scale` (all forms), title, caption, legend, header and footer (one-line
    and block forms); `skinparam dpi`; gradient colours;
  - SVG with both bounders: `-f svg-deterministic` (width table) and `-tsvg` (embedded Liberation fonts, plus fonts
    registered with `--font` / `ROCKUML_FONTS`). Both match the goldens byte for byte, so L3 needs no ε comparator yet;
  - PNG: the SVG rasterised by resvg with the same fonts, at PlantUML's image size (cropped at `PLANTUML_LIMIT_SIZE`),
    with the source in an `iTXt` chunk;
  - CLI: exit status 200 for error images, `--font` / `ROCKUML_FONTS`, rendering on a large-stack thread.

  Learned along the way / deliberate deviations:
  - Java's `%.4f` rounds the shortest decimal representation half-up, not the exact binary value; `java::format_fixed`
    reproduces that. Java collections' iteration orders leak into output (regex results, salt grid lines), hence
    `JavaHashMap` / `JavaHashSet`.
  - Font names other than Java's logical fonts depend on the fonts installed where Java runs; rockuml names them like a
    machine without them (`Dialog`).
  - The oracle starts PlantUML through `tools/oracle/launcher`, which switches off the donation banners error images
    get in some minutes of the hour; rockuml never shows them.
  - Where PlantUML crashes while drawing (a creole `----` in SVG, an unclosed salt group) it prints a crash report with
    a random quote; rockuml does not reproduce crash reports, and the golden generator drops them.
  - rockuml matches PlantUML on the happy path. Java bugs and quirks that only show on odd input, easter eggs and
    toy diagrams are not ported.
  - Java's logical fonts on Windows are composites: their line height includes fallback fonts for other scripts. The
    font measurement reproduces that extent; characters the embedded fonts lack are measured with PlantUML's width
    table, where Java would measure them with a Windows font.
  - PNG antialiasing differs from Java2D, and resvg draws wavy underlines straight.
  - Top-level `@startcreole` separators are drawn across the sheet; PlantUML cannot draw them there (its debug output
    marks them unsupported, its SVG and PNG crash).
  - Java's logical fonts take some characters from Windows fallback fonts even where Arial or Times New Roman have
    them (some dashes and bullets, `…`, `™`, arrows, maths and box drawing, Vietnamese letters). rockuml measures
    those with the Liberation glyphs, which can differ by a pixel or two (corpus: `creole/escapes` SVG).
  - Several salt menu popups are drawn in creation order; Java's order follows identity hash codes, so it is
    effectively random.
  - Gradients compare by identity in Java, so a gradient background never counts as "the same as the image's"; rockuml
    keeps that, as it decides whether titles paint their background.
  - Moved on: sprites, `<img>` and emoji (all embedded as PNG or rendered through PlantUML's SVG parser) become
    Phase 2b, with a parity check on decoded pixels instead of PNG bytes. The `@startuml` best-error selection needs
    the UML diagram factories and moves to Phase 3. Salt border layouts, vertical tab bars and salt images are
    undocumented or rare and wait until someone needs them.

### Phase 2b — Images (sprites, `<img>`, emoji)
- `sprite $name [WxH/n] {…}` definitions (monochrome, compressed, SVG) and `<$name>` in creole; stdlib sprites.
- `<img:…>` (files through `Host`, data URIs) and `<:emoji:>` (PlantUML's SVG parser and the Twemoji set).
- The parity harness compares embedded images by decoded pixels, as Java's PNG encoding is not worth reproducing.
- **Exit:** sprite, image and emoji corpus cases pass L1 and L2 with pixel-compared images.
- **Status: done.** All 25 image corpus cases pass L1 (debug) and PNG size; 24 pass L2 and L3 with pixel-compared
  images. `img-jpeg` misses them: JPEG pixels come from `zune-jpeg`, a level or two off Java's libjpeg, and porting
  libjpeg is not worth it for how rarely diagrams embed JPEGs. Emoji (`<:smile:>`, `<#red:heart:>`) draw as vectors
  through a port of PlantUML's `SvgNanoParser`; the 1174 Twemoji SVGs ship as one Brotli bundle (0.5 MB,
  `tools/bundle-emoji.sh`), decompressed on first use. Details:
- Raster sprites pass all formats with exact pixels (`sprite-gray-levels`, `-compressed`, `-scale-color`, `-base64`,
  `-color`):
  - `sprite` definitions with 4, 8 and 16 gray levels, compressed (`z`) and 4096-colour data; `<$name>` in creole, with
    scale and colour; raster images as a `UShape` of ARGB pixels, which SVG re-encodes as PNG.
  - Scaled images go through a port of medialib's bilinear affine transform, which Java 2D's `AffineTransformOp` runs
    natively; it matched Java on every pixel of 593 random images and scales, so the harness needs no tolerance.
  - PlantUML replaces every `data:image/png;base64,` payload in the source by its MD5 before parsing
    (`UmlSource.patchBase64`), and the seed hashes the shortened lines. rockuml does the same, so base64 sprites
    reach `CommandSpriteMd5`; `CommandSpriteBase64` can never match and is not ported.
  - Muting a raster sprite to the text colour adds the alpha to an opaque colour, which carries into the alpha byte
    (opaque pixels become alpha 254); rockuml keeps that, as text mutes every raster sprite.
  - Salt diagrams look sprites up in their own dictionary, which is not ported: `<$name>` draws nothing there yet.
  - SVG sprites (`sprite $name <svg ...>`, on one line or several) draw through the nano parser, sized by their
    `viewBox` rounded up or else their `width`/`height`; its path reader now takes quadratic curves and exponents.
    `!pragma svgParser sax` is not ported. Built-in sprites (`<$archimate/actor>`, `sprite $n jar:archimate/actor`)
    are bundled Brotli-compressed (`tools/bundle-sprites.sh`, 211 KB of SVG and PNG in 22 KB); `CommandSpriteFile`'s
    file and zip sources need host I/O and answer "Cannot read" for now. Standard library sprites come from the
    `sprite` (16 gray levels) and `svg` channels (`stdlib-office`, `stdlib-archimate-svg`).
  - On a malformed SVG path PlantUML fails; rockuml draws the movements before the fault.
- `<img:...>` passes all formats (`img-data-uri`, `-file`, `-svg`, `-svg-styled`, `-formats`); `img-jpeg` passes debug
  and PNG only:
  - PNG data URIs (by their MD5), SVG data URIs, files beside the diagram and URLs; PNG, GIF and JPEG pixels. Files and
    URLs go through AWT in PlantUML, which drops the colour of see-through pixels and rounds translucent ones through
    premultiplied alpha; rockuml does the same.
  - SVG images are embedded as `data:image/svg+xml` under a new root element, scaled by a transform on their first
    group. PlantUML's PNG driver draws none, so rockuml leaves them out of the SVG it rasterises.
  - Creole builds atoms while drawing, where the engine has no `Host`. Instead `diagram::create` takes the host and
    reads every file and URL an `<img>` in the source names, relative to the diagram file's directory; creole finds
    the content through the skin (`SpriteContainer::image_file`). An embedder without files gets `(Cannot decode)`,
    as PlantUML draws for a missing file outside its INSECURE profile.
  - Deviations: JPEGs are decoded by `zune-jpeg`, whose pixels differ from Java's libjpeg by a level or two, so
    `img-jpeg` fails the SVG pixel comparison. Undecodable base64 draws `(Cannot decode...)` instead of PlantUML's
    exception text, and an SVG declaring no size takes no room where PlantUML fails. `plantuml.include.path` is not
    searched. `@startcreole`, salt and error images do not read images yet.
  - Where PlantUML fails on an image it cannot make, rockuml draws nothing: images without pixels (a `0x0` sprite, a
    tiny scale), scaling a source 32768 pixels wide or high (medialib refuses), and an SVG image whose root is not
    `<svg>`. So that no input exhausts memory, declared sprite sizes beyond 32767 pixels a side or 2^24 pixels (a
    4096 x 4096 image, PlantUML's default `PLANTUML_LIMIT_SIZE`) are a command error, scaled images beyond that draw
    nothing, and compressed sprites inflate no more bytes than they have pixels.
  - Files (`<img>`, `!include`) obey `SFile.isFileOk`: the default LEGACY profile refuses paths under `/etc/`,
    `/dev/`, `/boot/`, `/proc/` and `/sys/` and those starting with `//`; SANDBOX and the allowlist profiles refuse
    every file, as their allowlists are not read; INSECURE allows all. A refused file is missing. Like PlantUML, `..`
    is not resolved first.

### Phase 3 — Sequence diagrams (~20k)
- Also: the `@startuml` factory order and best-error selection, so that unknown syntax gives PlantUML's error image.
- teoz (`PlayingSpace`, `LivingSpaces`, tiles), the `real` constraint solver, sequence `graphic` components,
  all ~41 commands, notes, groups, refs, dividers, delays, autonumber, boxes, return, activation, create/destroy, newpage.
- **Exit:** L1 ≥ 98% on the sequence corpus. This is the first genuinely usable release.
- **Status: done.** All 89 sequence corpus cases pass L1 (debug). L2 (deterministic SVG) and L4 (PNG size) pass on all
  88 cases PlantUML can render (`notes-aligned` crashes Java's SVG and PNG output). L3 (font-measured SVG) passes on
  87: `stereotypes` draws its spot letters as glyph outlines from Courier New Bold, which the embedded Liberation Mono
  cannot reproduce. Done:
  - the `real` constraint solver, `YGauge`s, living spaces with their activation stairs, and every teoz tile:
    messages (to self, from and to the border, multicast, creation), notes (beside, over, across, merged, on
    messages), life events, groups with `else` and `partition`, references, dividers, delays, spacing and page breaks;
  - participants of every shape, boxes around them (nested, coloured, stereotyped), autonumber, `return`,
    autoactivate, link anchors (`{a} <-> {b}`), `mainframe`;
  - the Rose components, their styles (with stereotype styles and the legacy skinparams) and line wrapping at
    `maxMessageSize` / `MaximumWidth` (PlantUML's `Fission`);
  - pages: each `newpage` starts a page with its own title, clipped as PlantUML's `UClip` does per output format;
  - skinparam deprecation warnings drawn above the diagram.

  Learned along the way / deliberate deviations:
  - Teoz is the only sequence engine in 1.2026.8 (`!pragma teoz` changes nothing); the older Puma engine is not ported.
  - Teoz draws links only on participants; links on messages, notes and references are parsed and dropped, as in
    PlantUML.
  - Shadows (`skinparam shadowing`) are not drawn yet.
  - Several notes `note across` merged with `/` crash PlantUML; rockuml lays them out like other merged notes.
  - A stereotype's spot letter is drawn from the font's glyph outline as in PlantUML, but centred on an unhinted
    rendering where Java uses a hinted one; with Courier New registered the outlines match and the centre can differ
    by a pixel.
  - `skinparam padding` around titles, headers, legends and the mainframe is not ported yet.
  - Moved on: the `@startuml` best-error selection needs the other UML diagram factories and moves to Phase 5. Until
    then a `@startuml` diagram that is not a sequence diagram is reported as not ported.

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
- **Status: done.** The crate `crates/smetana` (about 12k lines) reproduces Smetana bit-exactly, with no tolerance, on
  329 traces: the 21 layouts the corpus makes plus 308 seeded random graphs built through the same cgraph calls as
  PlantUML's three drivers (class-like diagrams, JSON/YAML records with ports, git). Every phase is compared on its
  own (ranks, mincross orders, positions, splines, final layout), and the routing, pathplan and xlabels building blocks
  against thousands of recorded calls.
  - Oracle: `tools/oracle/smetana-trace/` patches copies of eight Smetana sources at build time so that, with
    `ROCKUML_SMETANA_TRACE` set, each layout writes its cgraph call sequence and the state after every dot phase.
    `smetana-traces.sh` traces the corpus, `smetana-random.sh` the random graphs (`RandomGraphs.java`);
    `tools/oracle/smetana-unit/` dumps unit fixtures.
  - Java's `Math.cos`/`sin`/`pow` are HotSpot intrinsics, not fdlibm. `jmath` computes them correctly rounded, which
    equals Java on every value the traces and fixtures use (1 ulp apart in about 0.1% of random arguments); `atan2` is
    fdlibm's (`libm`). Bit-exact everywhere would need a port of HotSpot's GPL-only stubs.
  - Recursions as deep as the graph (network simplex, acyclic, decompose, flat search) use explicit stacks with the
    same visiting order, so 2000-node graphs lay out on a 1 MB stack (the wasm default).
  - `smetana::Graph` builds a graph with PlantUML's calls and `layout()` returns a `Drawing` or a `LayoutError` for
    what Smetana throws on; on wasm, where panics abort, such input still aborts. PlantUML never produces it except
    as noted below.
  - Deviation: Smetana loops forever in `fastgr`'s `basic_merge` when two opposite edges between clusters merge into
    each other (`merge_oneway`), which real PlantUML hits on a small class diagram with two packages linked both ways
    (`tools/oracle/README.md`). rockuml skips such a merge, as later Graphviz does, and lays the graph out.
  - Java fails, and so does rockuml, on record ports under `rankdir=LR`, some vertical record labels and a record
    port node with several in-edges; PlantUML's JSON diagrams could produce these.

### Phase 5 — CucaDiagram family (~40k: svek + cucadiagram + decoration + diagrams)
- The svek glue (`EntityImage*`, clusters, `SvekEdge` label placement, extremities), the `sdot` driver
  (`CucaDiagramFileMakerSmetana`, composite-state recursion), `net/atmp/CucaDiagram`.
- Class/object, description (usecase/component/deployment/archimate), state (multi-pass parser), chen ER.
- The `@startuml` factory order and best-error selection (moved here from Phase 3), so that unknown syntax gives
  PlantUML's error image.
- `ExternalDot` engine (optional): port svek's DOT writer and colour-tag SVG back-parser. Selected with
  `!pragma layout dot`, or automatically when `dot` is on PATH if the user opts in via config. Smetana stays the default.
- **Exit:** L1 ≥ 95% per type.
- **Status: done.** Every corpus case of the family passes L1 (debug), L2 (deterministic SVG) and PNG size: class 63,
  object 16, usecase 22, component 29, deployment 20, archimate 5, state 35 (47 layouts with the nested ones), chen 6.
  L3 (font-measured SVG) passes everywhere except class diagrams (1 of 63): a class's spot letter is drawn as the
  outline of a Courier New Bold glyph, which the embedded Liberation Mono cannot reproduce (as `sequence/stereotypes`).
  Every layout builds exactly the Smetana graph PlantUML builds, call for call (checked against the traces).
  - `@startuml` tries PlantUML's factories in order and picks its best error when none parses; diagram types not
    ported yet still parse (their commands exist as parse-only stubs generated from Java's command lists) so that the
    selection and error positions match, then report that they are not ported.
  - Ported: the CucaDiagram model (`abel`, `plasma`, `diagram/cuca`), the class, description, state and chen
    commands, notes and tips, hide/show/remove, the Smetana driver (`sdot`), svek's clusters and entity images,
    USymbols, link decorations (extremities).
  - Deviations: links to a package drawn with a symbol that has no big form crash Java; rockuml draws them. State
    transitions drawn as nodes (`-[node]->`) are reported as not ported: PlantUML names that node with the current
    time. A stray `}` at the top level is an error (PlantUML accepts it and fails later). Corpus cases use Java's
    logical fonts only: fonts the reference machine happens to have are named `Dialog` by rockuml.
  - Not ported (no corpus case): `newpage` in these diagrams, `(element)` creation in class diagrams, domain and
    requirement elements, stereotype skinparam colours on notes, the optional ExternalDot engine.

### Phase 6 — Activity v3 (~25k)
- ftile + vcompact + vertical + gtile, swimlanes, goto, notes, partitions, parallel/split, switch, repeat/while, detach.
- **Exit:** L1 ≥ 95%.
- **Status: done.** Every activity corpus case (106) passes L1 (debug), L2 (deterministic SVG), PNG size and the URL
  and preprocessor outputs. L3 (font-measured SVG) fails only `connectors` and `connectors-several`: circled
  connector letters are drawn as Courier New glyph outlines (as class spots).
  - Ported: the activity v3 model and commands, `ftile` with the `vcompact` delegator chain (if/elseif/switch with
    every condition style, while/repeat with breaks and backward, fork/split/merge, partitions and groups, notes and
    Opale), `vertical` tiles, Snake/Worm arrows, compression across then down, swimlanes (lane sizing, titles,
    colours, dividers, crossing arrows), the layered `UGraphic` they draw through, skinparam `padding`, `monochrome`
    and `reversecolor`. Bit-exact harnesses in `tools/oracle/activity-unit` check the model, arrows, compression
    and tile trees against Java.
  - Still differing elsewhere: skinparam `padding` in sequence, class and description diagrams.
  - Not ported: `gtile` (dead in 1.2026.8: `USE_GTILE` is false), shadows, the legacy activity syntax
    (`(*) -->`, Tier 2), skinparam `mode dark` (it needs PlantUML's dark colour variants).
  - Deviations: where PlantUML throws on odd input (a switch without `case`, a lane missing for a horizontal line,
    a slanted arrow segment) rockuml reports an error or skips the shape instead of crashing.

### Phase 7 — Distribution polish (Tier 1 complete)
- Full CLI parity (`-t*`, `-o`, `-pipe`, `-pipemap`, `-charset`, `-D`, `-config`, `-theme`, `-checkonly`, `-failfast2`,
  `-nbthread`, dir and glob inputs, output naming `name_001.svg` for newpages, embedded source in PNG/SVG, `-metadata`).
- Wasm package: `rockuml.wasm` + JS shim + a static demo page (editor + live SVG), all running locally in the browser.
- Release builds: `rockuml.exe`, `rockuml-linux-x86_64`, `rockuml-macos-universal`; `cargo dist` or a plain GitHub-Actions-style script.
- Binary size budget: about 15 MB with the full stdlib, about 3 MB wasm without stdlib/emoji (fetch them lazily on the web).
- **Status: done**, except the size budgets, which turned out unrealistic (below).
  - Command line: PlantUML's `CliFlag` table with every spelling, `--help`/`--help-more`, `-pipe` (with
    `-pipedelimitor`, `-pipenostderr`, `--pipe-image-index`), `-D`/`-I`/`-P`/`-S`, `--theme`, `--config`, `--charset`,
    `--check-syntax`, `--stop-on-error`, `--check-before-run`, `--no-error-image`, `--threads`, `--exclude`, files,
    directories and `*`/`?`/`**` patterns, `-o` (with `dir$`), `--extract-source`, `--disable-metadata`, `--skip-fresh`,
    `--overwrite`, `--sprite` (uncompressed), `--null`, and PlantUML's exit statuses. 65 scenarios in `tests/cli`,
    recorded from the golden model (`tools/oracle/cli-goldens.sh`), check stdout, stderr, exit status and the files
    written.
  - Deviations: unknown `-options` are refused unless such a file exists (PlantUML takes them for file names); a
    diagram the engine panics on (where PlantUML throws and draws a crash report) is reported as
    `<output>: crashed: <message>` with status 200 and no image; `**` does not follow links to directories; status 1
    for flags, formats or diagram types not ported, which outranks PlantUML's 200/50/100. Not ported: other output
    formats (PDF, EPS, LaTeX, txt/utxt, HTML, SCXML, XMI, VDX, obfuscate, base64, braille), `-pipemap`, `--verbose`,
    `-stdlib`, `--list-keywords`, compressed sprites. Dropped: GUI, FTP server, statistics, clipboard, splash
    screen, progress bar, Graphviz checks, dark mode.
  - Wasm: `crates/rockuml-wasm` (a C ABI, no wasm-bindgen), `web/rockuml.js` (dependency-free ES module for
    browsers and Node), `web/index.html` (editor with live preview), `tools/build-wasm.sh`, Node tests against the
    goldens. The test suite also passes on Linux (checked under WSL).
  - Releases: `.github/workflows/release.yml` builds `rockuml.exe`, a static musl `rockuml-linux-x86_64`, a universal
    macOS binary and `rockuml-web.zip` on a `v*` tag; `ci.yml` tests on Windows, Linux and macOS. Not run yet: the
    repository has no GitHub remote.
  - Sizes: embedded assets are deflated and the stdlib and emoji are cargo features. Native `rockuml.exe` is about
    26 MB: the stdlib (8.7 MB, already Brotli) and fonts (2.4 MB deflated) are 11.6 MB of data before any code, and the
    code is 10.9 MB at opt-level 3. opt-level `s` would save about 3 MB at roughly 45% slower rendering, so release
    builds keep opt-level 3. Wasm (opt-level `s`, no stdlib or emoji) is 9.5 MB, 4.95 MB gzipped, of which the fonts
    are 2.4 MB; reaching 3 MB would mean fetching the fonts separately.

### Phase 8+ — Tier 2, then Tier 3, ordered by what you and your friends actually use.
Cheapest first unless use says otherwise: mind maps and work breakdowns, JSON and YAML, the HTTP server, then
nwdiag, timing and gantt.

### Phase 8 — Mind maps and work breakdowns
- **Status: done.** Every corpus case (19 mind maps, 17 work breakdowns) passes L1, L2, L3, PNG size, URL and
  preprocessor outputs.
  - Ported: `@startmindmap` (org-mode, markdown tabs, `+`/`-`, `0` root, multiline nodes, sides, `left side` and
    direction commands, `top to bottom direction`, several roots) with PlantUML's Tetris packing of subtrees;
    `@startwbs` (new and old orders, quoted labels with aliases, multiline nodes, `<`/`>`, boxless and pseudo nodes,
    `Width auto`, links between aliases); the depth-weighted node styles (`getMergedStyleSpecial`, starred rules),
    with Java's wrapping `int` priorities.
  - Deviations: a mind map or breakdown without a root draws nothing (PlantUML crashes).
  - Not ported: the ASCII-art output of work breakdowns (txt/utxt are Tier 3).
- Every push to `main` that passes CI is published as a pre-release with the binaries and the web package; a `v*`
  tag as a release.

### Phase 9 — JSON and YAML
- **Status: done.** Every corpus case (10 JSON, 8 YAML) passes L1, L2, PNG size, URL and preprocessor outputs, and
  builds exactly the Smetana graph PlantUML builds (checked against the traces). L3 fails four JSON cases only: PlantUML
  measures `☑` and `☃` with Segoe UI Symbol, which rockuml does not embed.
  - Ported: `@startjson` and `@startyaml` with PlantUML's style extraction (`<style>`, `title`, `scale` before the
    data), `#highlight` paths with stereotypes, the YAML parser (block maps and lists, flow lists, `|` blocks), the
    tables (in the new `NO_CREOLE` mode) and their Smetana layout, drawn turned.
  - Deviations: a malformed `#highlight` line is ignored (PlantUML fails the whole diagram).
  - Not ported: `skin` and handwritten drawing before the data (reported as not ported).

### Phase 10 — HTTP server
- **Status: done.** `--http-server[:port[:address]][:stop]` serves `GET /png/`, `/svg/` (also under `/plantuml/`),
  `POST /render` (JSON source and command-line options) and `/serverinfo` like PlantUML's `PicoWebServer`, so editor
  plugins can use rockuml as their server. Images, status codes and the size and error headers match PlantUML's
  (checked side by side); `tests/http_server.rs` drives a real server.
  - Deviations: `/stopserver` really stops the server; ASCII-art formats and `/language` answer 501; a source without
    a diagram posted to `/render` answers 400; the description, title, donation and quote headers are left out.

### Phase 11 — Network diagrams
- **Status: done.** Every corpus case (12 nwdiag) passes L1, L2, L3, PNG size, URL and preprocessor outputs.
  - Ported: `@startnwdiag` networks (`address`, `color`, `width = full`, `description`), servers with their
    addresses, shapes, colors and descriptions, servers on several networks, peer links (`a -- b`), groups inside
    and outside networks, laid out in PlantUML's `NTetris` columns and drawn on its grid with the `nwdiag` styles.
  - Deviations: an unnamed network (`network {`) with an address shows only the address (PlantUML prints `null`).

### Phase 12 — Timing diagrams
- **Status: done.** Every corpus case (20 timing) passes L1, L2, L3, PNG size, URL and preprocessor outputs.
  - Ported: robust, concise, rectangle, clock (period, pulse, offset), binary and analog players (bounds, ticks,
    height); states by time or by player, defined and long state labels, `{-}`, `{hidden}`, `{...}` and two states at
    once, colours and comments; messages, constraints, notes, highlights; `@` times, relative times, anchors
    (`as :name`), hours, dates, clock ticks; `scale … as … pixels`, `mode compact`, `hide`/`manual time-axis`,
    `use date format`.
  - Deviations: dates print in UTC (PlantUML uses the JVM's time zone); `use date format` knows the
    `SimpleDateFormat` letters `G y Y M L d D E u H k K h m s S a` and refuses others; a manual time axis labels a
    time with the name of any anchor at that time (PlantUML only the anchor's own); states given to a clock, or
    defined for a binary or analog player, are ignored (PlantUML fails).

### Phase 13 — Gantt charts
- **Status: done.** Every corpus case (22 gantt) passes L1, L2, L3, PNG size, URL and preprocessor outputs.
  - Ported: the sentence language (`[Task] starts at [Other]'s end and lasts 2 weeks`, `then`, `it`, aliases,
    stereotypes, resources with percentages, `they`), PlantUML's workload model (piecewise constant calendars
    integrated over time, with its caching), milestones, groups, separators, constraints and arrow styles, notes,
    colours and completion, pauses, closed, opened, coloured and named days, resources' days off, `today`, the task
    table and its columns, daily, weekly, monthly, quarterly and yearly scales with zoom and week numbering,
    `print between`, `hide closed`, `hide footbox`, `hide resources names|footbox`, `language`.
  - Deviations: `today` without a date is the host's current day in UTC (PlantUML uses the JVM's time zone); a task
    whose calendar never opens gives up after 100 000 days (PlantUML loops forever).
  - Not ported: working hours (`from 9:00 to 17:00 are working hours`, reported as not ported); the ASCII-art
    table of txt output (Tier 3).

### Phase 14 — Website
- **Status: done.** An Angular 22 site in `site/`, deployed to GitHub Pages by CI once the repository is public.
  - The site: a home page with a live editor and a gallery of every diagram type, a playground whose address is a
    link to its diagram (fit or actual size, PNG and SVG downloads), and 30 documentation pages: every ported
    diagram type, text formatting, styles, themes, sprites, the preprocessor, the command line, the server, the
    JavaScript API, the Angular components and compatibility. Each page opens with a TL;DR template and its
    examples are edited in place with a live preview, all rendered by the wasm build in the browser.
  - `site/projects/rockuml-angular`: the components as a library shaped for npm, with two entry points:
    `rockuml-angular` (provider, renderer, `<rockuml-diagram>`, link codes) and `rockuml-angular/editor`
    (`<rockuml-editor>`, `<rockuml-playground>`, the CodeMirror 6 language), so applications that only show
    diagrams never load an editor. The site consumes the built package, as an npm user would.
  - `web/` became the `rockuml` package, with `package.json` and TypeScript types (`rockuml.d.ts`).
  - Both are on npm as `@rockuml/core` and `@rockuml/angular`, published by CI on `v*` tags through npm's trusted
    publishing (no token stored, provenance recorded); `site/tools/stage-npm-packages.mjs` assembles them with
    their license texts (`tools/collect-licenses.sh`).
  - Tests: the library's and the site's unit tests (Vitest), and `site/tools/examples.test.mjs`, which renders
    every documentation example with `rockuml.wasm` and fails on error images and deprecated syntax.

### Phase 15 — Feature status
- **Status: done.** Suggested by Arnaud Roques, PlantUML's author: a table of what is ported, planned soon,
  planned later or not planned (with the reason), usable to see what each new PlantUML release asks of rockuml.
  - `tools/features/inventory.py` lists a PlantUML release from its sources (`features/plantuml-<version>.json`):
    52 diagram factories with the commands each registers (1159), 74 preprocessor functions, 92 command line
    flags, 29 output formats, 157 style names and 30 style properties, each with a hash of its Java source.
  - `tools/features/status.py` decides what rockuml ports: commands by their Java class names in the Rust
    modules of their diagram (outside the stubs of unported commands), functions by name, flags and formats by
    the command line's tables; `features/decisions.json` holds the rest. It writes FEATURES.md and the website's
    feature page; CI checks both are current.
  - `tools/features/compare.py` compares two inventories: new, removed and changed diagram types, commands,
    functions, flags, formats and style names, flagging changed items rockuml has ported.
  - The ported commands that the sources did not name yet now carry their Java names.

### Phase 16 — Shared commands and pages
- **Status: done.** Every command of the ported diagram types is ported (752 of 752).
  - `skin name` starts from an embedded or local `.skin`; `<style file=…>` reads a style sheet from a file or a
    URL; `<style>…</style>` fits on one line; a `skinparam` block may come on one line (`%newline()`). Files a
    source names are read before the diagram is built, as images are, so that the engine does no I/O.
  - `page 2x3` cuts the PNG files the command line writes, framed by `pageMargin`, `pageBorderColor` and
    `pageExternalColor` (Gantt charts frame them their own way), like PlantUML's `PngSplitter`.
  - `newpage` in class, object and description diagrams: every page is a diagram of its own. PlantUML 1.2026.8
    writes only the first page (`NewpagedDiagram` lost its `getNbImages`); the oracle is built with that fixed
    (`tools/oracle/fixes.patch`) and rockuml writes every page.
  - `() Name` declares an interface in class diagrams; `!assume transparent`, `minwidth` and `rotate` are
    accepted and ignored, as in PlantUML.
  - Found on the way: rockuml draws no shadows (`Shadowing`, `skinparam shadowing`, the `rose` skin). Next phase.

### Phases 17+ — The remaining planned diagram types
Measured in the reference sources, about 45k Java lines in four groups. Each phase grows the corpus first, from the
examples of the PlantUML language reference. Goldens can be generated on Linux (`tools/oracle/setup-linux-jdk.sh`).

### Phase 17 — Shadows
- **Status: done.** `Shadowing`, `skinparam shadowing` and the `rose` skin draw PlantUML's blurred shadows, in SVG
  (`filter`) and PNG, in every ported diagram that has them. 21 shadowing cases and the `rose` skin case pass every output, except the class
  case's font-measured SVG, whose glyph outlines differ as in every class diagram.
  - Rectangles, ellipses, polygons and paths carry `deltaShadow`. Like PlantUML, `rounded()`, `halfRounded()` and the
    compression flags give a new shape without it, so the order of the calls matters.
  - Legends never shadow (`TextBlockBordered` hard-codes it off); timing diagrams shadow only notes.

### Phase 18 — Legacy activity diagrams (~2.6k)
- **Status: done.** Every corpus case (15 legacy-activity) passes L1, L2, L3, PNG size, URL and preprocessor
  outputs, and makes the Smetana graph PlantUML makes (`diagram/activity/graph_tests.rs`).
  - Ported: `ActivityDiagramFactory` and its commands over the ported cucadiagram and Smetana: `(*)` starts and
    ends (`(*top)`, `(*n)`), quoted and named actions, `as`, actions on several lines, synchronization bars, arrows
    with directions, lengths, labels, styles, `*` and URLs, `if`/`else`/`endif` branches, partitions with colours
    and stereotypes, notes on actions and on links, `hide`, `left to right direction`; `EntityImageActivity`
    with its shadow. A generic `UBrexSingleLine` command replaces the Gantt chart's own.
  - Deviations: `skinparam activityShape octagon` draws rounded boxes, as PlantUML does with Smetana (only
    Graphviz gives the node an octagon).
  - Not ported: inner activities (`-->{` ... `}`), reported as not ported.

### Phases 19+ — The self-contained Tier 3 diagrams (~33k)
Smallest and most reusing first: file trees (0.9k), HCL (0.9k, the JSON renderer), flow (1k), boards (1.3k),
git (2k), regex and EBNF together (2k + 6.2k, both railroad diagrams), packet (2.7k), wire (3.5k), BPM (5.7k),
charts (6.3k).

### Phase 19 — File trees
- **Status: done.** Every corpus case (9 files) passes L1, L2, L3, PNG, URL and preprocessor outputs.
  - Ported: `@startfiles` (`FilesDiagramFactory`): `/paths` that share their folders, files that repeat, notes
    between `<note>` and `</note>` placed beside the last file, `filesDiagram` and `note` styles. Like PlantUML,
    lines that do not start with `/` are ignored, `title` and `scale` among them.
  - The wasm build, which leaves out the emoji, keeps the folder and the page that file trees draw.

### Phase 20 — HCL data
- **Status: done.** `@starthcl`: `HclDiagramFactory` and `HclParser` read the file into JSON, drawn by the JSON
  diagram's tables and Smetana layout. 11 cases match every output.
  - Faithful to PlantUML's small subset: lines are joined without a separator, nested blocks need `=`, lists keep
    only quoted strings and maps, a lone block is drawn as its attributes, and `title`, `skin` and `<style>` are
    taken out of the data and ignored (only `scale` applies).

### Phase 21 — EBNF and regex diagrams
- **Status: done.** `@startebnf` (`PSystemEbnfFactory`) and `@startregex` (`PSystemRegexFactory`) draw railroad
  diagrams with the same `ETile` tiles. 14 EBNF and 15 regex cases match every output.
  - EBNF: rules on one line or several, `,` `|` `( )` `[ ]` `{ }` `{ }-` `n *` and `-`, the three kinds of
    terminal, `(* *)` comments as notes on boxes, rule names and between rules, `Syntax error!` for rules the
    engine cannot build, "Unparsable expression" for unknown characters.
  - Regex: letters merged into boxes, classes, sets, POSIX classes, anchors, escapes, quantifiers with counts,
    named groups and lookarounds; "Error parsing: …" and PlantUML's `ProtectedCommand` message where its reader
    throws. Like PlantUML, only the last expression is drawn, and `<style>` applies before any line is read.
  - Not ported: `!option use_descriptive_names` (the preprocessor has no `!option`), and the crash report of a
    regex diagram without expression.

### Phase 22 — Packet diagrams
- **Status: done.** `@startpacketdiag`: `PacketDiagramFactory` and its commands (fields by bit range, single bit or
  `*` with `len`, `colwidth`, `node_height`, `scale_interval`, `scale_direction`, `same_height`, `height`), the
  bit scale (`PacketIndicator`) and the fields (`PacketBlock`, labels cut short with `...`, `packetdiagDiagram`
  and `rectangle` styles with corners and shadows). 10 cases match every output.
  - Faithful to PlantUML's quirks: a field continued after full rows keeps a full row's width, so rows can grow
    wider than `colwidth`; a row is as high as its last field; `rotate` has no effect, and the open edges of split
    fields are drawn closed.
  - Deviation: a first row without width, which makes PlantUML fail on a remainder by zero, draws a short
    unnumbered tick.

### Phase 23 — Git diagrams
- **Status: done.** `@startgit`: `GitDiagramFactory` reads `git log --graph` text (`GitTextArea`, `CursorPosition`,
  `Commit`), merges straight runs of commits into one box (`GNodeBuilder`) and lays the graph out with Smetana
  (`SmetanaForGit`, `MagicBox`, `GitCurve`). 10 cases match every output and make PlantUML's Smetana graph.
  - Faithful to PlantUML: lines without a `*` are ignored, `title`, `skinparam` and `<style>` among them, and a
    `|\` at the end of a line joins the next line, as everywhere. A graph PlantUML cannot follow crashes it; rockuml
    reports it as not ported. Deviation: a loop of commits named twice, on which PlantUML never stops, is drawn.

### Phase 24 — Boards
- **Status: done.** `@startboard` (`BoardDiagramFactory`, `CommandBoardPlus`): activities with their releases
  one `+` per row, columns from `BNode.computeX`, cards drawn in the order of `BArray`'s `HashMap`, dashed lines
  between the rows, and the common commands (title, legend, styles, scale, sprites). 9 cases match every output.
  - A release before any activity crashes PlantUML; rockuml reports it as not ported.

### Phase 25 — Charts
- **Status: done.** `@startchart` (`ChartDiagramFactory` and its ten commands): bar, line, area and scatter series
  by category or as coordinate pairs, grouped and stacked bars, horizontal bars, a second value axis, custom
  ticks, spacing, grids, legends on four sides, annotations with arrows, the `chartDiagram` styles with
  stereotypes, and the common commands. Rotated axis titles (`UText` orientation) are drawn in every output.
  16 cases match every output.
  - Faithful to PlantUML's quirks: series without a colour take their style's grey, scatter markers are black
    unless `MarkerColor` says otherwise, and grid lines fall at tenths of the axes' ranges, not at their ticks.
    Numbers print as `String.format` does (`%.2f`, `%.2e`).

### Phase 26 — Reference cards
The welcome screen, `colors`, `license`, `version`, `skinparameters`, `listfonts`, `emoji`, `openiconic`,
`listopeniconic`, the sprite lists, `help …` and `@startdef`, on the ported creole, tables and sprites. `version`,
`license` and `listfonts` tell rockuml's facts, not PlantUML's.
- **Status: done.** Icon, sprite, help and definition cards: 19 cases (`tests/corpus/cards`) match every
  output, and so does the preprocessor's `includedef-and-subs`, which draws a `@startdef` block.
  - Ported, on `GraphicStrings`, the donors card's columns (`PSystemDonors.getCols`) and the ported creole and
    sprites: `emoji` and `emoji <start of a name>` (`PSystemListEmojiFactory`), `openiconic <name>`
    (`PSystemOpenIconicFactory`), `listopeniconic` (`PSystemListOpenIconicFactory`), `listsprites` alone, which
    lists the built-in ArchiMate sprites (`PSystemListArchimateSpritesFactory`), and after sprite definitions,
    which draws them (`ListSpriteDiagramFactory`), `help keywords|types|colors|themes|fonts` (`HelpFactory`,
    `LanguageDescriptor`), and `@startdef` (`PSystemDefinitionFactory`), which draws its start line only.
  - Faithful to PlantUML: `emoji` lists the shortcuts that start with its text as well as the codes; the whole
    emoji list and the Open Iconic list are corpus cases (2 MB and 0.7 MB of goldens).
  - Deviations: `help fonts` lists the fonts rockuml always has (Java's logical fonts, the Liberation fonts and the
    Windows families they stand in for), not those of the machine, so it has a unit test and no corpus case. An
    unknown `openiconic` name crashes PlantUML; rockuml reports it as not ported. The browser build has no emoji
    but the two that file trees draw, so its emoji list shows no more.
- Welcome screen, colours, skin parameters, fonts, version and license: 10 cases (`cards`)
  match every output: the welcome screen, a blank diagram's error, the colour chart and the hexagons around a
  colour (`PSystemColors`), `skinparameters` (`SkinParam.getPossibleValues`) and `stdlib`. `GraphicStrings`
  draws the cards and the welcome text of short error images.
  - rockuml's own facts, in PlantUML's layout, tested by unit and command line tests: `version` and `testdot`
    tell rockuml's version, the PlantUML release it reproduces, its license and its Smetana layout, not Java,
    memory or Graphviz; `authors` that it ports PlantUML; `license` its LGPL, the EPL of its layout code and
    the licenses of what it embeds; `listfonts` the embedded Liberation fonts, Java's logical fonts and the
    fonts registered with `--font` or `ROCKUML_FONTS`. These cards draw no PlantUML logo.
  - Not planned: the license keys of PlantUML's professional edition (`keygen`, `keycheck`…), reported as not
    ported.

### Phases 27+ — The rest
- The undocumented flow, wire and BPM diagrams, ported faithfully like the others.
- On hold, not dropped: ditaa (the vendored `org/stathissideris`, 10k lines, a bitmap drawn by Java2D, comparable
  by pixels only) and `@startmath`/`@startlatex` (JLaTeXMath is not in the sources; port it or use a Rust math
  layout crate).

### Phase 27 — Graphviz sources
- **Status: done.** `@startdot` (`PSystemDotFactory`, `PSystemDot`): the lines from a graph's header on go to an
  external Graphviz `dot`, run as PlantUML runs it (`-Tsvg`, `-Tpng`, its error stream joined to its output),
  found through `--dot-path`, `-DGRAPHVIZ_DOT`, `GRAPHVIZ_DOT`, the `PATH` and the usual install places. The
  engine asks its host (`Host::create_for_system_dot`); the command line runs `dot`, the browser build has none
  and reports the diagram as not available. 11 cases (`tests/corpus/dot`) match every output, and 3 command line
  scenarios cover a missing `dot` and the security profiles that suppress `dot`'s SVG.
  - Goldens and parity need the oracle's Graphviz: `tools/oracle/setup-graphviz.sh` unpacks Ubuntu's Graphviz
    2.42 into `tools/graphviz` (never installed, so the other goldens stay Smetana's) with a font
    configuration of rockuml's Liberation fonts only. The parity suite skips the `dot` cases, with a note, when
    the `dot` rockuml finds draws `dot/digraph` differently; CI sets `ROCKUML_PARITY_REQUIRE_GRAPHVIZ` on Linux,
    where it runs the same script, so the cases must run there.
  - Faithful to PlantUML: a dot syntax error or warning ends up in the image file, a missing `dot` or a failed
    run draws PlantUML's green-on-black message and exits with status 200 (503 from the server), and
    `INSECURE`/`INTERNET_WITH_DOTSVG` write the "feature has been suppressed" SVG. As in `ProcessRunner`, a
    `dot` still running 30 seconds (`--graphviz-timeout`) after its output ends is killed: a crash.
  - Deviations: `-f debug` and `svg-deterministic` ask `dot` for formats it does not have; PlantUML then
    reports a crash or writes `dot`'s complaint, depending on a race that the first diagram of a run usually
    loses. rockuml always reports the crash, and the generator drops goldens of the complaint.

### Phase 28 — BPM diagrams
- **Status: done.** `@startbpm` (`BpmDiagramFactory` and its seven commands): events, merges, `resume`, `goto`
  and branches with `else` and `end branch`, placed on PlantUML's `Grid`, its connectors routed and its lines
  merged by `CleanerEmptyLine` and `CleanerInterleavingLines`, drawn by `GridArray` with the activity diagram's
  tiles. 14 cases match every output; of 1900 random diagrams, the 685 PlantUML draws match its debug output
  and the others fail in both.
  - Faithful to PlantUML's sketch: the grid's lines, red stubs and blue connectors are drawn, the factory has no
    common commands, and the elements ignore skins, as PlantUML draws them with a skin of their own (no
    shadows). An `else` or `end branch` without a branch is PlantUML's `NoSuchElementException` syntax error.
  - Where PlantUML's layout throws (a connector through an element, a `goto` or `resume` to an unknown merge,
    a connection back to its own line or column), rockuml reports the diagram as not ported. Deviation: of
    two merges with the same name, `goto` and `resume` take the first, where PlantUML's `HashMap` takes either.

### Phase 29 — Wire diagrams
- **Status: done.** `@startwire` (`WireDiagramFactory` and its seven commands): blocks with sizes and colours
  (`CommandComponent`), nested by indentation, columns (`CommandNewColumn`), the cursor moved by `goto` and `move`,
  texts printed at the cursor (`CommandPrint`), spots at pixel and percent offsets (`CommandSpot`), and wires and
  buses across or down with arrowheads, colours and labels (`CommandWLink`), over the common commands. 11 cases
  (24 images) match every output.
  - Faithful to PlantUML's quirks: any space that is not part of four makes a level, so two spaces are two;
    a name must be new at every level down to its parent; links down draw no label; link starts are fixed when
    the link is read, spots when drawn. Prints are measured as for PNG whatever the output, as PlantUML does
    (`FileFormat.PNG.getDefaultStringBounder()`), with the embedded fonts. A line indented deeper than the blocks
    go reports PlantUML's `NullPointerException`, word for word.

### Phase 30 — Flow diagrams
- **Status: done.** `@startflow` (`FlowDiagramFactory`, `CommandLineSimple`, `CommandLink`): boxes on the grid of
  PlantUML's `golem` package (`TilesField`, `Position`, `TileGeometry`), each two cells from the last one, the boxes
  in its way moved east (`moveAllToEast`), links between facing sides or westwards along a row, and the boxes
  (`ActivityBox`). 11 cases: 8 match every output; `directions`, `links` and `crowded` match their PNG, preprocessor
  and URL outputs, and draw the same shapes as the debug and SVG goldens in another order.
  - Faithful to PlantUML: no common commands, so `title`, styles and empty lines are syntax errors; a link to an
    unknown id or between sides that do not face each other reports the `IllegalArgumentException` PlantUML throws.
  - Deviation: PlantUML keeps tiles in `HashMap`s keyed by identity hash codes, so the order it draws boxes in
    changes with the Java machine, between `-pipe` and file runs and even between two output formats of one case
    (`crowded`'s SVG golden swaps two boxes its debug golden does not). rockuml takes the tiles in the order they
    were declared.
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
1. Remaining Tier 2: PDF output (svg2pdf), then `-pipe`, `-o` and directory globbing parity checks.
   New diagram features get a section in the website's documentation, with live examples.
2. Grow the corpus per diagram type before porting it (examples from the PlantUML language reference).
3. Close the gaps FEATURES.md lists in ported diagrams, starting with those marked "planned soon".

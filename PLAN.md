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
    `-stdlib`, `--list-keywords`, compressed sprites. Dropped: GUI, HTTP/FTP servers, statistics, clipboard, splash
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
1. Phase 9: JSON and YAML diagrams (`@startjson`, `@startyaml`), then the HTTP server mode that lets editor
   plugins use rockuml.
2. Grow the corpus per diagram type before porting it (examples from the PlantUML language reference).

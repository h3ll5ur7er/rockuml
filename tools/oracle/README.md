# Golden-model oracle

The goldens in `tests/corpus` come from PlantUML 1.2026.8, built from the reference sources. Java is only needed
to regenerate goldens. Building and testing rockuml never needs it.

## One-time setup

1. Put the reference sources in `reference/plantuml-lgpl-1.2026.8-sources` (git-ignored).
2. Unpack a portable JDK 21 into `tools/jdk/` (git-ignored), for example the Temurin zip from
   `https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jdk/hotspot/normal/eclipse`.
3. `bash tools/oracle/build-reference.sh` builds `tools/oracle/build/plantuml-ref.jar`.

**Do not install Graphviz** and do not set `GRAPHVIZ_DOT`. Without `dot`, PlantUML lays out graphs with Smetana,
which is the engine rockuml ports, so the goldens stay comparable.

## Usage

```bash
bash tools/oracle/generate-goldens.sh                 # every case in tests/corpus
bash tools/oracle/generate-goldens.sh path/to/x.puml  # selected cases
bash tools/oracle/reference-plantuml.sh -f debug x.puml
```

Each case's goldens are written to `<case>.golden/` beside it. A case can produce several files per kind:
one per block (`x.preproc`, `x_001.preproc`, …), one per page, or a name chosen by `@startuml name`.

| Extension | Produced by | Compared by the parity suite |
|---|---|---|
| `.preproc` | `-preproc` | exactly (line endings normalised) |
| `.debug` | `-f debug` (font-independent dump of drawn shapes) | exactly (render timestamps masked) |
| `.svg` | `-f svg` (text measured with the fonts Java finds on Windows) | exactly |
| `.png` | `-tpng` | by size (antialiasing differs) |
| `.dsvg` | `-f svg-deterministic` (oracle-only name: SVG measured with PlantUML's font-independent width table) | exactly |

The `.svg` and `.png` goldens must be generated on Windows: elsewhere Java measures with other fonts. rockuml's embedded
Liberation fonts have the Windows fonts' metrics.

Where PlantUML crashes it draws a crash report with a random quote; the generator drops such goldens (and the PNG of a crashed
SVG), because rockuml renders the diagram instead.

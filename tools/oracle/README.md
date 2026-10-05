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

For each case, the following goldens are written next to it:

| File | Produced by | Compared by the parity suite |
|---|---|---|
| `x.preproc` | `-preproc` | exactly |
| `x.debug`, `x_001.debug`, … | `-f debug` (font-independent dump of drawn shapes) | exactly |
| `x.svg`, `x_001.svg`, … | `-f svg` | not yet (needs a structural comparator) |

```rockuml tldr
@startuml
title What rockuml does with a file
start
:read diagram.puml;
:run the preprocessor
(!include, !define, -D …);
:draw every diagram in the file;
if (--svg?) then (yes)
  :write diagram.svg;
else (no)
  :write diagram.png;
endif
stop
@enduml
```

The `rockuml` binary turns diagram sources into images. It takes PlantUML's command line, so build scripts, Makefiles and CI jobs written for `plantuml` work with `rockuml` as they are. Save the template above as `diagram.puml` to try the commands on this page.

## Drawing files

```bash
rockuml diagram.puml                  # writes diagram.png next to it
rockuml --svg diagram.puml            # writes diagram.svg
rockuml --svg docs/                   # every diagram file in docs/
rockuml --svg "docs/**/*.puml"        # every .puml under docs/, at any depth
rockuml --svg --output-dir out docs/  # writes the images into out/
```

A directory stands for the files in it that can hold diagrams, the same ones PlantUML looks at: `.puml`, `.pu` and `.txt`, and source files that may have diagrams in their comments, such as `.java`, `.c`, `.cpp`, `.h`, `.html` and `.tex`. Files without a diagram are skipped. Patterns take `*` and `?` within a directory and `**` across directories; quote them so your shell leaves them alone. `--exclude "**/drafts/**"` skips files that match a pattern.

A file with several diagrams, or a diagram with `newpage`, writes several images: `diagram.png`, `diagram_001.png`, `diagram_002.png`, and so on. A diagram named on its start line, `@startuml checkout`, is written as `checkout.png`; `--ignore-startuml-filename` ignores such names.

## Output formats

| Flag | Output |
|---|---|
| `--png` (default) | PNG images |
| `--svg` | SVG images |
| `-f svg-deterministic` | SVG measured with PlantUML's built-in width table instead of fonts, identical on every machine |
| `--preproc` | the source after preprocessing, without drawing anything |
| `-f debug` | a text description of every shape, as PlantUML's own tests use |
| `-f null` | nothing, for timing or syntax checks |

The old PlantUML spellings work too: `-tsvg`, `-tpng`, `-o out`, `-pipe`, `-charset UTF-8`.

## Pipes

`-p` (or `-pipe`) reads a source from standard input and writes the image to standard output, for use in scripts and other programs:

```bash
cat diagram.puml | rockuml --svg -pipe > diagram.svg
echo "@startuml
Alice -> Bob : hi
@enduml" | rockuml -pipe > hello.png
```

## Defines, includes and themes

These flags add to every diagram, as if the lines were written at its top:

```bash
rockuml -DAUTHOR="Arthur Dent" -DVERSION=42 diagram.puml   # !define AUTHOR …
rockuml -I common/styles.puml diagram.puml                 # !include common/styles.puml
rockuml -Pteoz=true diagram.puml                           # !pragma teoz true
rockuml --skinparam backgroundColor=#FFEEDD diagram.puml   # skinparam backgroundColor #FFEEDD
rockuml --theme cyborg diagram.puml                        # !theme cyborg
rockuml --config settings.puml diagram.puml                # the lines of settings.puml
```

## Checking diagrams

`--check-syntax` checks every diagram without drawing anything, and the exit status says how it went, which makes it a good CI step:

```bash
rockuml --check-syntax docs/
```

| Exit status | Meaning |
|---|---|
| `0` | everything was drawn |
| `1` | a diagram needs a part of PlantUML rockuml doesn't have yet |
| `50` | no file was found |
| `100` | no diagram was found in the files |
| `200` | a diagram has a syntax error |

`--stop-on-error` stops at the first error, `--check-before-run` checks everything before drawing anything, and `--no-error-image` writes no image for a diagram with an error (normally rockuml draws the error, with the line it happened on).

## Fonts

rockuml measures and draws text with fonts it carries, metric clones of Arial, Times New Roman and Courier New, so an image is the same on every machine. `--font` adds your own fonts, which diagrams then use by name:

```bash
rockuml --font ~/fonts/Inter.ttf --font ~/fonts/more/ diagram.puml
```

`--font` takes a `.ttf`, `.otf` or `.ttc` file or a directory of them, and can be repeated. The `ROCKUML_FONTS` environment variable holds more such paths, separated like `PATH`.

## Metadata

PNG and SVG files carry their diagram's source, so the image alone is enough to edit it again:

```bash
rockuml --extract-source diagram.png     # writes the source back out
rockuml --skip-fresh --svg docs/         # only redraws images whose source changed
rockuml --disable-metadata diagram.puml  # leaves the source out
```

## URL codes

PlantUML servers take a diagram in their URL as a compressed code. rockuml makes and reads these codes:

```bash
rockuml --encode-url diagram.puml
rockuml --decode-url SyfFKj2rKt3CoKnELR1Io4ZDoSa70000
```

## Sprites

`--sprite 16 icon.png` turns an image into a [sprite](docs/sprites) definition to paste into a diagram. 4 and 8 give fewer grey levels and a shorter text.

## Everything else

| Flag | Effect |
|---|---|
| `--threads 4` or `--threads auto` | draws several files in parallel |
| `--overwrite` | overwrites read-only output files |
| `--duration` | prints how long it took |
| `--version` | prints rockuml's version and the PlantUML release it is compatible with |
| `--help`, `--help-more` | lists the flags |
| `--http-server` | starts the [server for editor plugins](docs/server) |

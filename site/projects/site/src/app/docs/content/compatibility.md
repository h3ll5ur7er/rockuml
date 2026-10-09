```rockuml tldr
@startuml
title How rockuml relates to PlantUML
component "PlantUML 1.2026.8\n(Java)" as PlantUML
component "rockuml\n(Rust)" as Rockuml
artifact "your .puml files" as Sources
artifact "images" as Images
Sources --> PlantUML
Sources --> Rockuml
PlantUML --> Images
Rockuml --> Images : the same,\nwithout Java
Rockuml ..> PlantUML : tested against
@enduml
```

rockuml is a port of [PlantUML](https://plantuml.com) 1.2026.8 from Java to Rust. The goal is simple: the same source gives the same image. This page explains what "the same" means, and lists what rockuml does differently or doesn't do yet.

## How rockuml is tested

rockuml is checked against PlantUML itself. A corpus of over 600 diagrams covers every diagram type and feature rockuml supports, and for each one PlantUML's output was recorded. Every change to rockuml is tested against all of them, at three levels:

- the **shapes**: every line, box and text, with its coordinates, as PlantUML's own debug output describes them;
- the **SVG**: the image file itself;
- the **PNG size**, the URL encoding and the preprocessed source.

The tests run on Windows, Linux and macOS, and every diagram that passes must keep passing.

## Fonts

PlantUML measures text with the fonts of the machine it runs on, so the same diagram comes out slightly differently on Windows, Linux and macOS. rockuml carries its own fonts, the Liberation fonts, whose letters have exactly the widths of Arial, Times New Roman and Courier New. Its images match PlantUML's on Windows, and they are the same on every machine. You can still use [your own fonts](docs/cli#fonts).

## Layout

PlantUML lays out class, component, state and similar diagrams with Graphviz when it is installed, and otherwise with Smetana, its built-in Java translation of Graphviz's `dot`. rockuml contains an exact port of Smetana, so its layouts match PlantUML's Smetana layouts (`!pragma layout smetana`). PlantUML with Graphviz may place things differently. `skinparam linetype ortho` and `polyline` are ignored; lines are drawn as curves, as Smetana draws them.

## Not ported yet

The [feature status](docs/features) lists every diagram type, command, function, flag and output format of PlantUML, and whether rockuml ports it; it is generated from both projects' sources. In short:

| Area | What's missing |
|---|---|
| Diagram types | ditaa, EBNF, regex, charts, packet diagrams, HCL, git, files, board, wire, BPM and flow diagrams; the legacy activity syntax `(*) --> "Action"`; math with `@startmath`, `<math>` and `<latex>` |
| Output formats | PDF, EPS, LaTeX, ASCII art (`-ttxt`, `-tutxt`), HTML, SCXML, XMI, VDX and braille |
| Gantt charts | working hours (`from 9:00 to 17:00 are working hours`) |
| Salt | the `salt` keyword inside `@startuml`, and `{{ … }}` diagrams embedded in notes or labels |
| Drawing | handwritten drawing (`!option handwritten true`) |
| Command line | `-pipemap`, `--verbose`, `-stdlib`, `--list-keywords`, compressed sprites |

A diagram type that isn't ported yet is reported with a message and exit status 1, never drawn wrong. The rest is ordered by what people use; the [plan](https://github.com/h3ll5ur7er/rockuml/blob/main/PLAN.md) in the repository says what comes next.

## Deliberate differences

- **Dates.** Timing diagrams print dates in UTC, and a Gantt chart's `today` without a date is today in UTC. PlantUML uses the time zone of its Java machine.
- **Hopeless sources.** Where PlantUML crashes or loops forever, rockuml reports an error: a mind map without a root draws nothing, and a Gantt task whose calendar never opens gives up after 100 000 days.
- **Command line.** An unknown `-option` is refused unless a file of that name exists; PlantUML takes any unknown option for a file name. A diagram the engine fails on is reported as crashed, without an image.
- **The server.** `/stopserver` really stops the server; ASCII-art formats answer 501.
- **Extras.** PlantUML's easter eggs, GUI, FTP server, statistics, splash screen and Graphviz checks are left out.

## The browser build

The WebAssembly build that runs this site has no files, so `!include` of files and URLs, images from files, and themes from folders don't work in it. To keep it small, it also leaves out the standard library and emoji. The [JavaScript page](docs/javascript#what-the-browser-build-leaves-out) has the details.

## Reporting a difference

If a diagram looks different in rockuml than in PlantUML, that is a bug worth reporting. Open an issue on [GitHub](https://github.com/h3ll5ur7er/rockuml/issues) with the source, and if you can, PlantUML's image next to rockuml's.

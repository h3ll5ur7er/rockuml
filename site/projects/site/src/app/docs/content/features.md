```rockuml tldr
@startmindmap
<style>
mindmapDiagram {
  .ported { BackgroundColor #BBF7D0 }
  .planned { BackgroundColor #FEF3C7 }
  .never { BackgroundColor #E5E7EB }
}
</style>
* rockuml and PlantUML 1.2026.8
** Ported: 26 diagram types <<ported>>
*** Sequence diagrams <<ported>>
*** Class and object diagrams <<ported>>
*** Legacy activity diagrams <<ported>>
*** Use case, component, deployment and ArchiMate diagrams <<ported>>
*** State diagrams <<ported>>
*** Activity diagrams <<ported>>
*** Wireframes <<ported>>
*** Network diagrams <<ported>>
*** Mind maps <<ported>>
*** Work breakdown structures <<ported>>
*** Charts <<ported>>
*** Packet diagrams <<ported>>
*** Definitions drawn on their own <<ported>>
*** Creole text <<ported>>
*** Gantt charts <<ported>>
*** Timing diagrams <<ported>>
*** JSON data <<ported>>
*** Git diagrams <<ported>>
*** File trees <<ported>>
*** Boards <<ported>>
*** YAML data <<ported>>
*** HCL data <<ported>>
*** EBNF diagrams <<ported>>
*** Regular expression diagrams <<ported>>
*** Entity relationship diagrams <<ported>>
*** Reference cards <<ported>>
left side
** Planned <<planned>>
*** BPM diagrams <<planned>>
*** Graphviz sources <<planned>>
*** Ditaa diagrams <<planned>>
*** Math <<planned>>
*** LaTeX math <<planned>>
*** Flow diagrams <<planned>>
*** Wire diagrams <<planned>>
*** Reference cards <<planned>>
** Not planned <<never>>
*** PlantUML's donors <<never>>
*** Crash reports <<never>>
*** Easter eggs <<never>>
@endmindmap
```

This page compares rockuml with PlantUML 1.2026.8, feature by feature: what is ported and ready to use, what is planned and what is not, with the reason. It is generated from PlantUML's own sources and from rockuml's, so it stays true as both evolve, and when PlantUML publishes a new release, comparing the two releases shows exactly what rockuml has to catch up on.

| Status | Meaning |
|---|---|
| ✅ Ported | Works like in PlantUML. |
| 🟡 Partly ported | Works, except for the parts listed. |
| 🔜 Planned soon | Next on the plan, or a gap in something already ported. |
| 🗓️ Planned later | Planned, after the more widely used features. |
| ⛔ Not planned | Not planned, for the reason given. |

## At a glance

| Area | rockuml |
|---|---|
| Diagram types | 31 of 52 ported, 13 planned, 8 not planned |
| Commands of the ported diagrams | 1098 of 1098 ported |
| Preprocessor functions | 74 of 74 ported |
| Command line flags | 48 of 92 ported |
| Output formats | 6 of 29 ported |
| Style names and properties | 187 of 187 known |

## Diagram types

| Diagram | Status | Reference diagrams | Notes | In PlantUML |
|---|---|---|---|---|
| [Sequence diagrams](docs/sequence) | ✅ Ported | 92 of 92 match PlantUML |  | `SequenceDiagramFactory` |
| [Class and object diagrams](docs/class) | ✅ Ported | 84 of 84 match PlantUML |  | `ClassDiagramFactory` |
| [Legacy activity diagrams (`(*) --> "Action"`)](docs/activity) | ✅ Ported | 15 of 15 match PlantUML |  | `ActivityDiagramFactory` |
| [Use case, component, deployment and ArchiMate diagrams](docs/component) | ✅ Ported | 80 of 80 match PlantUML |  | `DescriptionDiagramFactory` |
| [State diagrams](docs/state) | ✅ Ported | 36 of 36 match PlantUML |  | `StateDiagramFactory` |
| [Activity diagrams](docs/activity) | ✅ Ported | 110 of 110 match PlantUML |  | `ActivityDiagramFactory3` |
| [The emoji list (`emoji`)](docs/reference-cards) | ✅ Ported | 19 of 19 match PlantUML | The browser build has no emoji but the two that file trees draw, so it lists no more. | `PSystemListEmojiFactory` |
| [An Open Iconic icon on its own (`openiconic`)](docs/reference-cards) | ✅ Ported | 19 of 19 match PlantUML | An unknown icon, on which PlantUML crashes, is reported as not ported. | `PSystemOpenIconicFactory` |
| [The Open Iconic list (`listopeniconic`)](docs/reference-cards) | ✅ Ported | 19 of 19 match PlantUML |  | `PSystemListOpenIconicFactory` |
| [The ArchiMate sprite list](docs/reference-cards) | ✅ Ported | 19 of 19 match PlantUML |  | `PSystemListArchimateSpritesFactory` |
| [Wireframes (Salt)](docs/salt) | ✅ Ported | 14 of 14 match PlantUML | `salt` inside `@startuml` and `{{ … }}` embedded diagrams are not ported yet. | `PSystemSaltFactory` |
| [Network diagrams (nwdiag)](docs/network) | ✅ Ported | 13 of 13 match PlantUML |  | `NwDiagramFactory` |
| [Mind maps](docs/mindmap) | ✅ Ported | 21 of 21 match PlantUML |  | `MindMapDiagramFactory` |
| [Work breakdown structures](docs/wbs) | ✅ Ported | 19 of 19 match PlantUML | The ASCII-art output is not ported (txt formats are planned later). | `WBSDiagramFactory` |
| [Charts](docs/chart) | ✅ Ported | 16 of 16 match PlantUML |  | `ChartDiagramFactory` |
| [Packet diagrams](docs/packet) | ✅ Ported | 10 of 10 match PlantUML |  | `PacketDiagramFactory` |
| [Definitions drawn on their own (`@startdef`)](docs/reference-cards) | ✅ Ported | 19 of 19 match PlantUML |  | `PSystemDefinitionFactory` |
| [The sprite list (`listsprites`)](docs/reference-cards) | ✅ Ported | 19 of 19 match PlantUML |  | `ListSpriteDiagramFactory` |
| [Creole text (`@startcreole`)](docs/creole) | ✅ Ported | 17 of 17 match PlantUML |  | `PSystemCreoleFactory` |
| [Gantt charts](docs/gantt) | ✅ Ported | 24 of 24 match PlantUML | Working hours are not ported yet; `today` without a date is today in UTC. | `GanttDiagramFactory` |
| [Timing diagrams](docs/timing) | ✅ Ported | 22 of 22 match PlantUML | Dates print in UTC; `use date format` knows the common SimpleDateFormat letters. | `TimingDiagramFactory` |
| [Help diagrams (`help keywords`, …)](docs/reference-cards) | ✅ Ported | 19 of 19 match PlantUML | `help fonts` lists the fonts rockuml always has, not those of the machine. | `HelpFactory` |
| [JSON data](docs/json) | ✅ Ported | 10 of 10 match PlantUML |  | `JsonDiagramFactory` |
| [Git diagrams](docs/git) | ✅ Ported | 10 of 10 match PlantUML |  | `GitDiagramFactory` |
| [File trees](docs/files) | ✅ Ported | 9 of 9 match PlantUML |  | `FilesDiagramFactory` |
| [Boards](docs/board) | ✅ Ported | 9 of 9 match PlantUML |  | `BoardDiagramFactory` |
| [YAML data](docs/yaml) | ✅ Ported | 8 of 8 match PlantUML |  | `YamlDiagramFactory` |
| [HCL data](docs/hcl) | ✅ Ported | 11 of 11 match PlantUML |  | `HclDiagramFactory` |
| [EBNF diagrams](docs/ebnf) | ✅ Ported | 14 of 14 match PlantUML |  | `PSystemEbnfFactory` |
| [Regular expression diagrams](docs/regex) | ✅ Ported | 15 of 15 match PlantUML |  | `PSystemRegexFactory` |
| [Entity relationship diagrams (Chen)](docs/chen) | ✅ Ported | 6 of 6 match PlantUML |  | `ChenEerDiagramFactory` |
| The welcome screen of an empty diagram | 🗓️ Planned later |  | A reference card; low priority. | `PSystemWelcomeFactory` |
| The colour chart (`colors`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemColorsFactory` |
| BPM diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `BpmDiagramFactory` |
| The license diagram (`license`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemLicenseFactory` |
| The version diagram (`version`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemVersionFactory` |
| The skin parameter list (`skinparameters`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemSkinparameterListFactory` |
| The font list (`listfonts`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemListFontsFactory` |
| Graphviz sources (`@startdot`) | 🗓️ Planned later |  | Tier 3 of the plan: needs an external `dot` or a port of more of Graphviz. | `PSystemDotFactory` |
| Ditaa diagrams | 🗓️ Planned later |  | Tier 3 of the plan: a port of the vendored ditaa code, about 10 000 lines. | `PSystemDitaaFactory` |
| Math (`@startmath`) | 🗓️ Planned later |  | Needs a TeX math layout engine, which PlantUML borrows from JLaTeXMath; the approach is still to be chosen. | `PSystemMathFactory` |
| LaTeX math (`@startlatex`) | 🗓️ Planned later |  | Needs a TeX math layout engine, which PlantUML borrows from JLaTeXMath; the approach is still to be chosen. | `PSystemLatexFactory` |
| Flow diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `FlowDiagramFactory` |
| Wire diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `WireDiagramFactory` |
| PlantUML's donors | ⛔ Not planned |  | About PlantUML itself, not about diagrams. | `PSystemDonorsFactory` |
| Easter egg | ⛔ Not planned |  | Easter eggs are PlantUML's own. | `PSystemEggFactory` |
| Easter egg | ⛔ Not planned |  | Easter eggs are PlantUML's own. | `PSystemAppleTwoFactory` |
| Easter egg | ⛔ Not planned |  | Easter eggs are PlantUML's own. | `PSystemRIPFactory` |
| Easter egg | ⛔ Not planned |  | Easter eggs are PlantUML's own. | `PSystemPathFactory` |
| Easter egg | ⛔ Not planned |  | Easter eggs are PlantUML's own. | `PSystemCharlieFactory` |
| Easter egg | ⛔ Not planned |  | Easter eggs are PlantUML's own. | `PSystemDedicationFactory` |
| Crash reports (`@startcrash`) | ⛔ Not planned |  | PlantUML's way of testing its crash report; rockuml reports crashes on the command line. | `CrashDiagramFactory` |

Reference diagrams are the cases of rockuml's test corpus; a matching one gives the same shapes, at the same coordinates, as PlantUML.

## Commands not yet ported

Every command of a ported diagram type that is not listed here is ported. Commands are named after PlantUML's Java classes.

None.

## Preprocessor functions

All 74 are ported.

## Command line flags

48 of 92 ported. Not ported:

| Name | Status | Notes |
|---|---|---|
| `--gui` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--dark-mode` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--verbose` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--progress-bar` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--splash-screen` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--check-graphviz` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `-pipemap` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--list-keywords` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--dot-path` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--ftp-server` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--clipboard` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--clipboardloop` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `-debugsvek` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `-printfonts` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `-stdlib` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `-stdrpt` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `-syntax` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `-word` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `-useseparatorminus` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--eps` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--teps:text` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--html` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--latex` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--latex-nopreamble` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--obfuscate` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--pdf` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--scxml` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--txt` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--utxt` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--vdx` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--xmi` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--xmi:argo` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--xmi:custom` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--xmi:script` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--xmi:star` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--base64` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--braille` | 🗓️ Planned later | Recognised and refused with a message; ported on demand. |
| `--enable-stats` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--export-stats-html` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--export-stats` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--html-stats` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--xml-stats` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--realtime-stats` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |
| `--loop-stats` | ⛔ Not planned | Belongs to PlantUML's Java runtime or GUI, which rockuml does not have. |

## Output formats

| Format | Status | Notes |
|---|---|---|
| `DEBUG` | ✅ Ported |  |
| `NULL` | ✅ Ported |  |
| `PREPROC` | ✅ Ported |  |
| `PNG` | ✅ Ported |  |
| `SVG_DETERMINISTIC` | ✅ Ported |  |
| `SVG` | ✅ Ported |  |
| `PDF` | 🔜 Planned soon | Next on the plan (Tier 2), through svg2pdf. |
| `EPS` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `EPS_TEXT` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `ATXT` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `UTXT` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `XMI_STANDARD` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `XMI_STAR` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `XMI_ARGO` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `XMI_CUSTOM` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `XMI_SCRIPT` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `SCXML` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `GRAPHML` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `HTML` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `LATEX` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `LATEX_NO_PREAMBLE` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `LATEX_DETERMINISTIC` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `BASE64` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `OBFUSCATE` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `PNG_EMPTY` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `RAW` | 🗓️ Planned later | Tier 3 of the plan, ported on demand. |
| `HTML5` | ⛔ Not planned | PlantUML's HTML5 canvas output; SVG covers browsers. |
| `VDX` | ⛔ Not planned | Visio's old XML format. |
| `BRAILLE_PNG` | ⛔ Not planned | Dropped in the plan; may be reconsidered on request. |

## Style names and properties

rockuml knows all 157 style names (the selectors of `<style>` sheets) and all 30 style properties of PlantUML.

## When PlantUML releases a new version

1. Make an inventory of the new release from its sources: `python tools/features/inventory.py <sources> <version> features/plantuml-<version>.json`.
2. Compare it with this one: `python tools/features/compare.py features/plantuml-1.2026.8.json features/plantuml-<version>.json`. The report lists the new, removed and changed diagram types, commands, functions, flags, formats and style names, and says which of the changed ones rockuml has ported, since those need a second look.
3. Port what is new, record decisions for the rest in `features/decisions.json`, set its `inventory` to the new file, and run `python tools/features/status.py` to update this page.

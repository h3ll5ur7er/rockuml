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
** Ported: 15 diagram types <<ported>>
*** Sequence diagrams <<ported>>
*** Class and object diagrams <<ported>>
*** Use case, component, deployment and ArchiMate diagrams <<ported>>
*** State diagrams <<ported>>
*** Activity diagrams <<ported>>
*** Wireframes <<ported>>
*** Network diagrams <<ported>>
*** Mind maps <<ported>>
*** Work breakdown structures <<ported>>
*** Creole text <<ported>>
*** Gantt charts <<ported>>
*** Timing diagrams <<ported>>
*** JSON data <<ported>>
*** YAML data <<ported>>
*** Entity relationship diagrams <<ported>>
left side
** Planned <<planned>>
*** Legacy activity diagrams <<planned>>
*** BPM diagrams <<planned>>
*** Graphviz sources <<planned>>
*** Charts <<planned>>
*** Packet diagrams <<planned>>
*** Ditaa diagrams <<planned>>
*** Definitions drawn on their own <<planned>>
*** Math <<planned>>
*** and 10 more <<planned>>
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
| Diagram types | 15 of 52 ported, 29 planned, 8 not planned |
| Commands of the ported diagrams | 653 of 752 ported |
| Preprocessor functions | 74 of 74 ported |
| Command line flags | 48 of 92 ported |
| Output formats | 6 of 29 ported |
| Style names and properties | 187 of 187 known |

## Diagram types

| Diagram | Status | Reference diagrams | Notes | In PlantUML |
|---|---|---|---|---|
| [Creole text (`@startcreole`)](docs/creole) | ✅ Ported | 17 of 17 match PlantUML |  | `PSystemCreoleFactory` |
| [JSON data](docs/json) | ✅ Ported | 10 of 10 match PlantUML |  | `JsonDiagramFactory` |
| [YAML data](docs/yaml) | ✅ Ported | 8 of 8 match PlantUML |  | `YamlDiagramFactory` |
| [Sequence diagrams](docs/sequence) | 🟡 Partly ported | 91 of 91 match PlantUML | 8 of 84 commands not yet ported, listed below (8 of them shared by every diagram). | `SequenceDiagramFactory` |
| [Class and object diagrams](docs/class) | 🟡 Partly ported | 80 of 80 match PlantUML | 10 of 82 commands not yet ported, listed below (8 of them shared by every diagram). | `ClassDiagramFactory` |
| [Use case, component, deployment and ArchiMate diagrams](docs/component) | 🟡 Partly ported | 76 of 76 match PlantUML | 9 of 68 commands not yet ported, listed below (8 of them shared by every diagram). | `DescriptionDiagramFactory` |
| [State diagrams](docs/state) | 🟡 Partly ported | 35 of 35 match PlantUML | 8 of 65 commands not yet ported, listed below (8 of them shared by every diagram). | `StateDiagramFactory` |
| [Activity diagrams](docs/activity) | 🟡 Partly ported | 106 of 106 match PlantUML | 8 of 89 commands not yet ported, listed below (8 of them shared by every diagram). | `ActivityDiagramFactory3` |
| [Wireframes (Salt)](docs/salt) | 🟡 Partly ported | 14 of 14 match PlantUML | `salt` inside `@startuml` and `{{ … }}` embedded diagrams are not ported yet. 8 of 41 commands not yet ported, listed below (8 of them shared by every diagram). | `PSystemSaltFactory` |
| [Network diagrams (nwdiag)](docs/network) | 🟡 Partly ported | 12 of 12 match PlantUML | 8 of 52 commands not yet ported, listed below (8 of them shared by every diagram). | `NwDiagramFactory` |
| [Mind maps](docs/mindmap) | 🟡 Partly ported | 19 of 19 match PlantUML | 8 of 49 commands not yet ported, listed below (8 of them shared by every diagram). | `MindMapDiagramFactory` |
| [Work breakdown structures](docs/wbs) | 🟡 Partly ported | 17 of 17 match PlantUML | The ASCII-art output is not ported (txt formats are planned later). 8 of 48 commands not yet ported, listed below (8 of them shared by every diagram). | `WBSDiagramFactory` |
| [Gantt charts](docs/gantt) | 🟡 Partly ported | 22 of 22 match PlantUML | Working hours are not ported yet; `today` without a date is today in UTC. 8 of 59 commands not yet ported, listed below (8 of them shared by every diagram). | `GanttDiagramFactory` |
| [Timing diagrams](docs/timing) | 🟡 Partly ported | 20 of 20 match PlantUML | Dates print in UTC; `use date format` knows the common SimpleDateFormat letters. 8 of 65 commands not yet ported, listed below (8 of them shared by every diagram). | `TimingDiagramFactory` |
| [Entity relationship diagrams (Chen)](docs/chen) | 🟡 Partly ported | 6 of 6 match PlantUML | 8 of 50 commands not yet ported, listed below (8 of them shared by every diagram). | `ChenEerDiagramFactory` |
| The welcome screen of an empty diagram | 🗓️ Planned later |  | A reference card; low priority. | `PSystemWelcomeFactory` |
| The colour chart (`colors`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemColorsFactory` |
| Legacy activity diagrams (`(*) --> "Action"`) | 🗓️ Planned later |  | Tier 3 of the plan: the current activity syntax covers the same diagrams. | `ActivityDiagramFactory` |
| BPM diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `BpmDiagramFactory` |
| The license diagram (`license`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemLicenseFactory` |
| The version diagram (`version`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemVersionFactory` |
| The skin parameter list (`skinparameters`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemSkinparameterListFactory` |
| The font list (`listfonts`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemListFontsFactory` |
| The emoji list (`emoji`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemListEmojiFactory` |
| An Open Iconic icon on its own (`openiconic`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemOpenIconicFactory` |
| The Open Iconic list (`listopeniconic`) | 🗓️ Planned later |  | A reference card; low priority. | `PSystemListOpenIconicFactory` |
| The ArchiMate sprite list | 🗓️ Planned later |  | A reference card; low priority. | `PSystemListArchimateSpritesFactory` |
| Graphviz sources (`@startdot`) | 🗓️ Planned later |  | Tier 3 of the plan: needs an external `dot` or a port of more of Graphviz. | `PSystemDotFactory` |
| Charts | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `ChartDiagramFactory` |
| Packet diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `PacketDiagramFactory` |
| Ditaa diagrams | 🗓️ Planned later |  | Tier 3 of the plan: a port of the vendored ditaa code, about 10 000 lines. | `PSystemDitaaFactory` |
| Definitions drawn on their own (`@startdef`) | 🗓️ Planned later |  | Definitions work with `!includedef`; drawing them on their own is rarely needed. | `PSystemDefinitionFactory` |
| The sprite list (`listsprites`) | 🗓️ Planned later |  | A reference card; low priority. | `ListSpriteDiagramFactory` |
| Math (`@startmath`) | 🗓️ Planned later |  | Needs a TeX math layout engine, which PlantUML borrows from JLaTeXMath; the approach is still to be chosen. | `PSystemMathFactory` |
| LaTeX math (`@startlatex`) | 🗓️ Planned later |  | Needs a TeX math layout engine, which PlantUML borrows from JLaTeXMath; the approach is still to be chosen. | `PSystemLatexFactory` |
| Flow diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `FlowDiagramFactory` |
| Help diagrams (`help keywords`, …) | 🗓️ Planned later |  | PlantUML's built-in reference cards; low priority. | `HelpFactory` |
| Wire diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `WireDiagramFactory` |
| Git diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `GitDiagramFactory` |
| File trees | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `FilesDiagramFactory` |
| Boards | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `BoardDiagramFactory` |
| HCL data | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `HclDiagramFactory` |
| EBNF diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `PSystemEbnfFactory` |
| Regular expression diagrams | 🗓️ Planned later |  | Tier 3 of the plan, ported on demand. | `PSystemRegexFactory` |
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

| Command | Status | In | Notes |
|---|---|---|---|
| `CommandAssumeTransparent` | 🗓️ Planned later | every ported diagram | Recognised and reported as not ported. |
| `CommandCreateElementParenthesis` | 🔜 Planned soon | Class and object diagrams | Recognised and reported as not ported. |
| `CommandMinwidth` | 🗓️ Planned later | every ported diagram | Recognised and reported as not ported. |
| `CommandNewpage` | 🔜 Planned soon | Class and object diagrams, Use case, component, deployment and ArchiMate diagrams | Recognised and reported as not ported. |
| `CommandPage` | 🗓️ Planned later | every ported diagram | Recognised and reported as not ported. |
| `CommandRotate` | 🗓️ Planned later | every ported diagram | Recognised and reported as not ported. |
| `CommandSkin` | 🗓️ Planned later | every ported diagram | Old skins are superseded by themes and styles. Recognised and reported as not ported. |
| `CommandSkinParamJaws` | 🗓️ Planned later | every ported diagram | A `skinparam` line with line breaks inside, which only macros produce. Not recognised yet: a syntax error. |
| `CommandStyleImport` | 🗓️ Planned later | every ported diagram | Reads style files. Recognised and reported as not ported. |
| `CommandStyleSingleLineCSS` | 🗓️ Planned later | every ported diagram | Recognised and reported as not ported. |

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

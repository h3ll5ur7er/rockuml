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

## Smetana layout traces

Phase 4 ports Smetana, PlantUML's Java port of Graphviz dot. Its oracle is a trace of every Smetana layout:
the graph PlantUML builds, the state after each dot phase, and the final values PlantUML reads back.

```bash
bash tools/oracle/smetana-traces.sh                  # every case in tests/corpus
bash tools/oracle/smetana-traces.sh path/to/x.puml   # selected cases
```

Each case is rendered with `-f debug` (font-independent text measurement, so traces match on every machine), and
each Smetana layout it runs writes `tests/smetana/<area>/<case>/NN.trace`, numbered from `01` in the order the
graphs are opened. Cases that do not use Smetana get no directory. A full run first deletes `tests/smetana`, except
the random graphs' traces in `tests/smetana/random` (see below).

### How it is hooked in

`build-reference.sh` copies the sources named in `smetana-trace/hooks.patch` to `build/patched-sources`, applies the
patch there and compiles those copies instead of the originals; `reference/` is never modified. The patch only adds
calls to `smetana-trace/rockuml/oracle/SmetanaTrace.java`, which does nothing unless the environment variable
`ROCKUML_SMETANA_TRACE` names a directory to write traces to. The hooks are:

| Java method | Hook | Records |
|---|---|---|
| `graph__c.agopen` (entry) | `agopen` | starts a trace (the nameless ProtoGraph that `gvContext` opens is ignored) |
| `subg__c.agsubg`, `node__c.agnode`, `edge__c.agedge`, `attr__c.agsafeset`, `gvc__c.gvContext` (entry) | same name | the call and its arguments |
| `gvlayout__c.gvLayoutJobs` (entry) | `gvLayoutJobs` | the call; collects nodes and edges in creation order |
| `dotinit__c.dotLayout`, after `dot_rank` / `dot_mincross` / `dot_position` / `dot_splines` | `afterRank` ... `afterSplines` | the phase state |
| `gvlayout__c.gvLayoutJobs`, after `gvle.layout.exe` (that is `dot_layout`: `doDot`, then `dotneato_postprocess`) | `afterLayout` | the final state; writes the file |

`dot_sameports` (between position and splines) only touches ports, and `dot_compoundEdges` only runs with
`compound=true`, which PlantUML never sets. Traces are recorded per `Globals`, so a layout nested in another
(a composite state laid out as a leaf) gets its own file.

### Format (version 1)

UTF-8 text, one record per line, tokens separated by single spaces. Strings are double-quoted with `\"`, `\\`,
`\n`, `\r` and `\t` escapes. Numbers are Java `Double.toString`, the shortest decimal that parses back to the same
double, so Rust's `str::parse::<f64>` recovers the exact bits; integers are plain.

Object references:

- `graph "<name>"` and `node "<name>"`: graphs, clusters and nodes by name.
- `edge eN`: the edge created by the N-th `agedge` call (cgraph's edge sequence number, which `gvLayoutJobs` checks).
- In layout dumps, nodes are written without the `node` keyword: `"<name>"` for real nodes and `vN` for virtual
  nodes, numbered in the order the trace first meets them. A virtual name stays the node's name for the rest of
  the trace.

```
smetana-trace 1
agopen "g"
agsubg graph "g" "cluster6"
agnode graph "cluster6" "sh0010"
agedge graph "g" node "sh0010" node "sh0011"
agsafeset <object> "<attribute>" <value> <default>
gvContext
gvLayoutJobs graph "g"
phase rank
graph "<name>" minrank R maxrank R        # the root, then clusters depth-first (GD_clust order)
node <node> rank R                        # every real node, in creation order
phase mincross
rank R <node> <node> ...                  # GD_rank(root)[R] left to right, virtual nodes included
phase position
node <node> rank R coord X Y lw L rw R ht H   # every node in GD_rank, rank by rank
graph "<name>" bb LLX LLY URX URY
graph "<name>" label pos X Y dimen W H set S  # only for graphs with a label
phase splines
edge eN spl none                          # no ED_spl
edge eN bezier I sflag S eflag E sp X Y ep X Y points N X Y X Y ...
edge eN label|head_label|tail_label|xlabel pos X Y dimen W H set S
phase final                               # after post-processing: what PlantUML reads
graph ... bb / label lines as above
node <node> coord X Y width W height H lw L rw R ht H   # real nodes; width and height in inches
edge ... lines as above
```

Attribute values are quoted strings, except PlantUML's fixed-size labels (`Macro.createHackInitDimensionFromLabel`,
the string `_dim_W_H_`), which are written as `dim(W,H)`. Record labels (JSON and YAML) keep their `_dim_W_H_`
fields verbatim inside the quoted string, because Smetana's record parser splits them.

### Determinism

Two full runs produce byte-identical traces, and the debug output with tracing on equals the goldens. Smetana's
object ids come from a JVM-wide `CString` counter (`CString.UID`), so absolute ids depend on what ran before in the
JVM; only their relative order (creation order) affects cgraph's dictionaries. One JVM per case keeps that stable.

### Random graphs

The corpus gives few layouts, so `smetana-trace/rockuml/oracle/RandomGraphs.java` (compiled into the jar with the
tracer) adds reproducible random ones. It builds each graph through the cgraph calls PlantUML makes, in the order
and with the attribute values PlantUML uses, in one of three styles:

| Style | Copied from | Graph |
|---|---|---|
| `cuca` | `sdot/CucaDiagramFileMakerSmetana` | boxes, nested clusters with fixed-size labels and margin 16 or 20, group core nodes (`zent…`), edges with `minlen` 0-3 and fixed-size `label`/`taillabel`/`headlabel`, self loops, multi-edges, back edges, sometimes `rankdir=LR` |
| `git` | `gitlog/SmetanaForGit` | boxes, `ranksep=0.35`, edges with `arrowhead=normal` |
| `json` | `jsondiagram/SmetanaForJson` | a tree of `shape=record` nodes with ports, edges leaving through `tailport=P<n>` |

```bash
bash tools/oracle/smetana-random.sh        # seeds 1 to 300
bash tools/oracle/smetana-random.sh 500    # seeds 1 to 500
```

Seed `s` always gives the same graph, and its trace is `tests/smetana/random/<s>.trace` (three digits). Low seeds
are small, plain graphs; graphs grow to 40 nodes and features get denser up to seed 200. `summary.txt` has one line
per seed, `<seed> ok <style> nodes N edges E <features>` (`virtual` means the layout made virtual nodes), or
`<seed> skipped <reason>` for a graph Smetana threw on (no trace). The generator avoids what PlantUML cannot build,
such as a record graph without any label (Smetana cannot parse the default `\N` as a record).

One JVM lays out a batch of seeds (`BATCH`, default 50). The traces do not depend on the batch size: runs with one
JVM per seed and with one JVM for all seeds are byte-identical.

## cgraph dumps

```bash
bash tools/oracle/cgraph-dumps.sh
```

`cgraph-dump/CgraphDump.java` replays a trace's input section through Java's cgraph and dumps what the port's
cgraph must reproduce: counts, the iteration orders of nodes, edges and subgraphs, the order of object ids, wildcard
edge lookups (`agfindedge`) and attribute values. Each `tests/smetana/**/NN.trace` gives
`tests/smetana-cgraph/**/NN.dump`; the hand-written inputs in `tests/smetana-cgraph/synthetic/*.trace` (same format)
get their dump next to them. `crates/smetana/tests/cgraph_replay.rs` makes the same calls and compares. Rerun the
script after regenerating the traces.

## Smetana unit oracles

`smetana-unit/` holds harnesses that run single Smetana functions against the golden-model jar; their fixtures are
in `crates/smetana/tests/data` and the matching Rust tests in `crates/smetana/tests`.

```bash
bash tools/oracle/smetana-unit/pathplan.sh   # pathplan.txt: Pshortestpath, Proutespline, solve3
bash tools/oracle/smetana-unit/routing.sh    # routing-*.txt: spline clipping, end boxes, routesplines, ports
```

`routing.sh` patches copies of `splines__c`, `routespl__c` and `shapes__c` (`smetana-unit/routing-hooks.patch`)
to call `RoutingDump` around `clip_and_install`, `beginpath`/`endpath`, `_routesplines`, `simpleSplineRoute`,
`makeSelfEdge` and the port functions, and puts them before the jar on the class path. It then lays out random
graphs (seeds 1-100, 200 and 241 by default; others as arguments), synthetic graphs and corridors (arrows PlantUML
never draws, boxes `checkpath` must repair) and the corpus cases that use Smetana. Each recorded call comes with the
state of the nodes and edges it reads (only where it changed since the last call) and is followed by its results
and the state it changed; `crates/smetana/tests/routing.rs` rebuilds that state, makes the same call and compares
bit for bit.

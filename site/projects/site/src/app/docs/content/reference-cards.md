```rockuml tldr
@startuml
colors Orange
@enduml
```

A diagram whose only line is a keyword draws a reference card instead of a diagram of yours: the colours you can name, the skin parameters, the fonts at hand, the version and license of rockuml. They are handy to look something up without leaving your editor. Each card is the whole diagram: with any other line beside the keyword, rockuml reads the lines as a diagram of their own.

## The welcome screen

An empty diagram, `@startuml` followed directly by `@enduml`, shows how to start, as in PlantUML.

```rockuml
@startuml
@enduml
```

A blank line between the two is no longer empty: it is an error, "Empty description".

## Colours

`colors` draws every colour name rockuml knows, on a swatch of that colour, black or white whichever reads better. Any of these names can stand wherever a colour goes, in any case: `#LightBlue`, `#lightblue`.

```rockuml
@startuml
colors
@enduml
```

Followed by a colour, a name or a hex value, `colors` draws the named colours closest to it on hexagons around it: a quick way to pick a neighbouring shade.

```rockuml
@startuml
colors #8AF
@enduml
```

The keyword may also be written `color`; it is case-sensitive, so `Colors` is an error.

## Skin parameters

`skinparameters` lists the names `skinparam` knows, in five columns, as PlantUML lists them. Many of them are covered by [style sheets](docs/styling) today.

```rockuml
@startuml
skinparameters
@enduml
```

## Fonts

`listfonts` lists the font families diagrams can name with `<font:…>` or `skinparam defaultFontName`, each with a sample in itself. Give your own sample text after the keyword.

```rockuml
@startuml
listfonts The quick brown fox
@enduml
```

PlantUML lists the fonts installed on the machine. rockuml never reads the machine's fonts, so that a diagram looks the same everywhere: it lists the Liberation fonts it embeds, under their own names and under the names of the Windows fonts they stand in for (Arial, Times New Roman and Courier New), Java's logical fonts (`SansSerif`, `Serif`, `Monospaced`, `Dialog` and `DialogInput`), and every font you added with `--font` or `ROCKUML_FONTS` (see the [command line](docs/cli)). The fonts you add are used, and listed, in the formats that draw text with fonts: PNG and SVG.

## Version and authors

`version` tells which rockuml draws the diagram, which PlantUML release it reproduces, its license and how it lays out graphs.

```rockuml
@startuml
version
@enduml
```

PlantUML's card also tells the Java runtime, the operating system, the memory and the Graphviz installation it uses. rockuml runs without Java and without Graphviz, so it tells none of those; `testdot`, which in PlantUML checks the Graphviz installation, says the same.

`authors` (or `about`) tells where rockuml comes from: a port of PlantUML, by Arnaud Roques and its contributors.

`stdlib` lists the libraries of the [standard library](docs/preprocessor) rockuml embeds, with their versions and where they come from, as PlantUML does. The browser build leaves the standard library out, so the list is empty there.

The `keygen`, `keyimport`, `keycheck` and `keydistributor` lines of PlantUML's professional edition are recognised and reported as not ported: rockuml has no license keys.

## License

`license` tells rockuml's license, in the layout of PlantUML's card: the GNU Lesser General Public License, version 3 or later, like the PlantUML edition it ports; the Eclipse Public License for its layout code, ported from Graphviz; the licenses of the fonts, icons, sprites, themes and emoji it carries; and that the images you make belong to you.

```rockuml
@startuml
license
@enduml
```

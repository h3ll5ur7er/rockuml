```rockuml tldr
@startflow
idea "Idea"
e draft "Write a draft"
s review "Review"
w ship "Ship it"
n idea
@endflow
```

Between `@startflow` and `@endflow`, rockuml draws a flow diagram: yellow boxes with rounded corners on a grid, each placed a step away from the one before, and red links between them. PlantUML has these diagrams but does not document them; rockuml reads and draws them as PlantUML 1.2026.8 does.

## Boxes

A box is a line with an id, a space and a label in double quotes: `review "Review"`. The first box sits at the top left. Every further box goes one step from the box written just before it: south unless the line starts with `n`, `s`, `e` or `w` (north, south, east or west, in either case) and a space. The two are linked, from the side of the old box to the facing side of the new one, with a dot where the link arrives.

```rockuml
@startflow
start "Start"
e east "East"
s down "Then south"
s further "South again"
w back "Then west"
@endflow
```

Each box is centred in a cell 200 pixels wide and 70 high, so a long label runs over its neighbours. Labels are written with [text formatting](docs/creole): bold, italics and colours work, `\n` does not break the line, and curly quotes can enclose the label as well as straight ones.

```rockuml
@startflow
note “**Bold**, //italic// and <color:blue>blue</color>”
@endflow
```

## Links

A line with an id and nothing else links the last box written to the box with that id, from the side its letter names (south when there is none) to the facing side of the other box. The two boxes have to touch on those sides; the one exception is a link westwards along a row, which crosses the boxes between. A box can link to itself.

```rockuml
@startflow
draft "Draft"
e review "Review"
e publish "Publish"
w draft
s archive "Archive"
n publish
@endflow
```

## Crowded cells

When the cell a new box should take is held by another box, that box moves two steps east, and so does every box to its right and below. A box moved away from the one it follows cannot be linked to it, and PlantUML reports the error: in practice a crowded cell works when going east or south.

```rockuml
@startflow
one "One"
e two "Two"
e three "Three"
s four "Four"
w five "Five"
w six "Six"
e seven "Seven"
@endflow
```

## Errors

Flow diagrams know only boxes and links: `title`, `skinparam`, styles, empty lines and every other command are syntax errors, though comments and the [preprocessor](docs/preprocessor) work. A link to an unknown id, or between sides that do not face each other, is reported with the Java exception PlantUML throws.

## Differences from PlantUML

PlantUML keeps the boxes in a table ordered by Java's identity hash codes, which change from one Java machine to another, even between two runs of different output formats. So the order in which it writes the boxes into an SVG file, the box a link finds when two boxes share an id, and, in a crowded row, which box moves where, vary. rockuml always takes the boxes in the order they were written.

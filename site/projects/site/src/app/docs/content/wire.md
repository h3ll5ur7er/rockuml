```rockuml tldr
@startwire
* cpu [120x80] #lightblue
* memory [120x60]
--
goto(220, 10)
* io_bridge [100x170]
cpu -> io_bridge : interrupt
cpu => io_bridge : data
memory <=> io_bridge : dma
cpu --> memory
@endwire
```

Between `@startwire` and `@endwire`, rockuml draws a wire diagram: boxes stacked in columns, with wires and buses from one box to the side of another. PlantUML has these diagrams but does not document them; rockuml reads them as PlantUML 1.2026.8 does, quirks included.

## Blocks

`* name` adds a block below the previous one. A block is 100 pixels square unless `[width x height]` (or `[width*height]`) sizes it, and a colour after it fills it. Underscores in names are drawn as spaces.

Indentation nests a block in the last block of the level above: four spaces or a tab make one level, and any other space makes one too, so two spaces are two levels. A block that holds others is as large as they need, unless it has a size of its own. A line of `-` starts a new column to the right of what its level holds.

```rockuml
@startwire
* board
    * cpu [60x40]
    * memory [60x40] #pink
    --
    * io
* rack [300x110] #lightgray
    * slot_a [80x60]
    * slot_b [80x60]
--
* single
@endwire
```

Names must be new: a block named like a block of a level above it is refused, as is a line indented deeper than the blocks go.

## Placing and printing

Each level has a cursor, where its next block goes. `goto(x, y)` sends it to a point of the block it is indented in, and `move(dx, dy)` moves it. `print("text")` writes a small text at the cursor and moves the cursor below it; `\n` breaks the line and [text formatting](docs/creole) works.

```rockuml
@startwire
print("A wire diagram")
* first [120x40]
print("**bold**, //italic//\nand <color:red>red</color>")
goto(200, 30)
* placed [80x50]
move(40, 20)
* shifted [80x50]
@endwire
```

## Spots

`spot name` draws a dot at the top left corner of a block, red unless coloured. `spot name(x, y)` places it: each coordinate is in pixels, a percentage of the block's size (`50%`), or both (`100%-10`). Blocks inside others are named by their path, `outer.inner`.

```rockuml
@startwire
* chip [200x120]
    * core [60x40]
spot chip
spot chip(100%, 0) #blue
spot chip(50%, 50%) #green
spot chip(100%-10, 100%+5)
spot chip.core(10, 10) #purple
@endwire
```

## Wires and buses

`from -> to` draws a wire from the right side of a block to the left side of another; `from => to` draws a bus, a wide arrow. `<` and `>` choose the arrowheads: `->`, `<-`, `<->` and `-` (or `=>`, `<=`, `<=>` and `=`). The links leaving a block start one below the other, 15 pixels apart after a wire and 25 after a bus, unless `from(x, y)` gives their start. A colour and a label after `:` follow the target.

Doubling the line, `-->` or `==>`, runs the link down from the bottom of a block to the top of another, with no label.

```rockuml
@startwire
* source [100x150]
* below [100x40]
--
goto(250, 10)
* target [100x150]
source -> target : forward
source <- target : backward
source <-> target #red : both ways
source => target : bus
source <=> target #blue
source(50%, 100%) --> below
@endwire
```

## Titles and decorations

`title`, `header`, `footer`, `caption`, `legend`, `mainframe` and `scale` work as in other diagrams, and so does the `document` style for the background. Blocks, links and prints keep their black and white look: skin parameters and styles do not change them.

```rockuml
@startwire
title Wiring of the board
caption A caption
* left_part [80x60]
--
* right_part [80x60]
left_part -> right_part : signal
@endwire
```

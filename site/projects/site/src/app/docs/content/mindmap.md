```rockuml tldr
@startmindmap
* The Hitchhiker's Guide
** Essentials
*** Towel
*** Babel fish
*** Don't panic
** Places
*** Magrathea
*** Milliways
left side
** People
*** Arthur Dent
*** Ford Prefect
*** Zaphod Beeblebrox
@endmindmap
```

A mind map puts a topic in the middle and lets ideas branch out from it, level by level. Mind maps have their own start line, `@startmindmap`, and a syntax that is mostly asterisks.

## Branches

Each line is a node; the number of asterisks is its depth. One asterisk is the root, two its children, and so on. `left side` sends the nodes after it to the left; nodes before it go right.

```rockuml
@startmindmap
* Minecraft
** Survival
*** Find food
*** Build a shelter
** Creative
*** Build a castle
left side
** Redstone
*** Doors
*** Calculators
** The End
*** Ender dragon
@endmindmap
```

### Plus and minus

Instead of asterisks, `+` adds nodes on the right and `-` on the left, in the same diagram:

```rockuml
@startmindmap
+ Infinity Stones
++ Space
++ Mind
++ Reality
-- Power
-- Time
-- Soul
--- Vormir
@endmindmap
```

### Markdown lists

Nodes can also be written as an indented Markdown list, with tabs for the levels:

```rockuml
@startmindmap
* Straw Hats
	* Luffy
		* Gomu Gomu no Mi
	* Zoro
		* Three swords
	* Nami
@endmindmap
```

## Node text

A node's text takes [formatting](docs/creole), and `\n` breaks a line. For text over several lines, write the node as `**:` and end it with `;`.

```rockuml
@startmindmap
* Naruto Uzumaki
** <b>Jutsu</b>
***:Shadow clone
<i>Kage Bunshin no Jutsu</i>;
*** Rasengan
** Dream\nbecome Hokage
@endmindmap
```

### Boxless nodes

An underscore after the asterisks draws a node without a box:

```rockuml
@startmindmap
* Simpsons
** Family
***_ Homer
***_ Marge
***_ Bart
***_ Lisa
***_ Maggie
**_ Springfield
@endmindmap
```

## Colours

A colour in brackets right after the asterisks colours one node:

```rockuml
@startmindmap
*[#Orange] Avengers
**[#Red] Iron Man
**[#Blue] Captain America
**[#Green] Hulk
--[#Gold] Thor
--[#Purple] Hawkeye
@endmindmap
```

To style many nodes at once, give them a stereotype and style it in a [style sheet](docs/styling#style-sheets):

```rockuml
@startmindmap
<style>
mindmapDiagram {
  .hero {
    BackgroundColor LightGreen
  }
  .villain {
    BackgroundColor Salmon
  }
}
</style>
* Marvel
** Spider-Man <<hero>>
** Doctor Strange <<hero>>
** Thanos <<villain>>
** Loki <<villain>>
@endmindmap
```

Style sheets also select nodes by depth with `:depth(1)`, and the special `rootNode`, `leafNode` and `arrow` elements:

```rockuml
@startmindmap
<style>
mindmapDiagram {
  node {
    BackgroundColor LightYellow
  }
  :depth(1) {
    BackgroundColor Khaki
  }
  rootNode {
    BackgroundColor Gold
    RoundCorner 0
  }
  leafNode {
    LineColor Gray
  }
  arrow {
    LineColor DarkOrange
    LineThickness 2
  }
}
</style>
* One Piece
** East Blue
*** Foosha Village
*** Baratie
** Grand Line
*** Alabasta
*** Skypiea
@endmindmap
```

`MaximumWidth` in a node style wraps long texts at that width.

## Direction

`top to bottom direction` grows the map downwards, and `right to left direction` puts everything on the left:

```rockuml
@startmindmap
top to bottom direction
* Pied Piper
** Engineering
*** Richard
*** Gilfoyle
*** Dinesh
** Business
*** Jared
*** Erlich
@endmindmap
```

## Several roots

A map can have more than one root: every line with a single asterisk starts a new tree, drawn beside the others.

```rockuml
@startmindmap
* Sith
** Vader
** Palpatine
* Jedi
** Yoda
** Obi-Wan
@endmindmap
```

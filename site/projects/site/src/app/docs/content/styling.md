```rockuml tldr
@startuml
<style>
sequenceDiagram {
  participant {
    BackGroundColor #FFF3D6
    LineColor #C2410C
  }
  arrow {
    LineColor #C2410C
    LineThickness 2
  }
  note {
    BackGroundColor #E0F2FE
  }
}
.villain {
  BackGroundColor #FCA5A5
}
</style>
participant Batman
participant Joker <<villain>>
Batman -> Joker : why so serious?
Joker -[#green]> Batman : HAHAHA
note right : style sheets for everything,\ninline colours for exceptions
@enduml
```

There are three ways to change how a diagram looks, from the most local to the most global: a colour right on an element, `skinparam` settings, and `<style>` style sheets. They combine: a style sheet sets the house style, and an inline colour makes one element stand out. To restyle everything at once, use a [theme](docs/themes).

## Inline colours

A colour after an element's declaration colours its background: a name (`#Pink`, `#LightSkyBlue`) or a hexadecimal code (`#FF8800`). Two colours with `/` make a gradient. `##` sets the border, optionally with a style: `##[dashed]blue`, `##[bold]red`, `##[dotted]`. The long form sets several parts at once: `#back:palegreen;line:green;text:darkgreen`.

```rockuml
@startuml
class Gryffindor #Crimson
class Hufflepuff #Gold ##[dashed]black
class Ravenclaw ##[bold]RoyalBlue
class Slytherin #back:DarkGreen;line:Silver;text:Silver
class Hogwarts #white/lightgray
Hogwarts *-- Gryffindor
Hogwarts *-- Hufflepuff
Hogwarts *-- Ravenclaw
Hogwarts *-- Slytherin
@enduml
```

Arrows take their colour and style in brackets, `-[#red,dashed]->`, and text has its own colour tags in [creole](docs/creole#colours-sizes-and-fonts).

## skinparam

`skinparam name value` changes one setting for the whole diagram. Settings have names like `BackgroundColor`, `ClassBorderColor` or `ArrowColor`, and the ones for one kind of element can be grouped in braces:

```rockuml
@startuml
skinparam backgroundColor #FFFDF5
skinparam roundCorner 12
skinparam shadowing false
skinparam class {
  BackgroundColor #FEF3C7
  BorderColor #B45309
  ArrowColor #B45309
  FontName Courier
}
class Hobbit
class Wizard
Wizard --> Hobbit : late, never
@enduml
```

A stereotype after a setting's name applies it only to elements with that stereotype:

```rockuml
@startuml
skinparam class {
  BackgroundColor<<Sith>> #FCA5A5
  BorderColor<<Sith>> DarkRed
  BackgroundColor<<Jedi>> #BFDBFE
}
class Vader <<Sith>>
class Luke <<Jedi>>
class Han
Vader --> Luke : I am your father
Han --> Luke : kid
@enduml
```

A few settings change more than colours:

| Setting | Effect |
|---|---|
| `skinparam monochrome true` | draws in black and white (`reverse` for white on black) |
| `skinparam dpi 150` | renders at a higher resolution |
| `skinparam defaultFontName Courier` | the font of all texts |
| `skinparam defaultFontSize 14` | the size of all texts |
| `skinparam roundCorner 12` | rounds the corners of boxes |
| `skinparam shadowing false` | removes shadows |

```rockuml
@startuml
skinparam monochrome reverse
skinparam defaultFontName Courier
skinparam defaultFontSize 14
actor Neo
participant "The Matrix" as M
Neo -> M : there is no spoon
M --> Neo : wake up
@enduml
```

## Style sheets

`<style>` … `</style>` holds a style sheet in a CSS-like syntax. Selectors are diagram types (`sequenceDiagram`, `classDiagram`, `activityDiagram`, `mindmapDiagram`, `ganttDiagram`, `jsonDiagram`, …), elements (`participant`, `arrow`, `note`, `class`, `node`, `title`, `legend`), and stereotypes starting with a dot.

```rockuml
@startuml
<style>
classDiagram {
  class {
    BackgroundColor #ECFDF5
    LineColor #047857
    FontColor #064E3B
    RoundCorner 8
  }
  arrow {
    LineColor #047857
    LineThickness 2
  }
}
title {
  FontColor #047857
  FontSize 20
}
</style>
title The Shire's supply chain
class Farmer
class "Green Dragon Inn" as Inn
class Hobbit
Farmer --> Inn : ale
Inn --> Hobbit : second breakfast
@enduml
```

### Properties

The common properties are:

| Property | Example |
|---|---|
| `BackgroundColor` | `#FFEEDD`, `pink`, `white/lightblue` |
| `LineColor` | `#333`, `red` |
| `LineThickness` | `2` |
| `LineStyle` | `4` (dashes), `8;3` (dash and gap) |
| `FontColor`, `FontSize`, `FontName`, `FontStyle` | `red`, `14`, `Courier`, `bold` or `italic` |
| `RoundCorner` | `10` |
| `Padding`, `Margin` | `8` |
| `MaximumWidth` | `150` (wraps longer texts) |
| `HorizontalAlignment` | `left`, `center`, `right` |
| `Shadowing` | `0` for none |

### Stereotypes

A selector with a dot styles the elements with that stereotype, in any diagram:

```rockuml
@startuml
<style>
.critical {
  BackgroundColor #FECACA
  LineColor #B91C1C
  FontStyle bold
}
.optional {
  BackgroundColor #F3F4F6
  LineStyle 4
}
</style>
start
:Save the cat; <<critical>>
:Take a selfie; <<optional>>
:Save the world; <<critical>>
stop
@enduml
```

### Depth

In mind maps and work breakdowns, `:depth(1)` selects the nodes of one level, and `rootNode` and `leafNode` the top and the ends. See [mind maps](docs/mindmap#colours).

## Colours of the page

`skinparam backgroundColor` colours the whole image, gradients included (`#FFFFFF-#AAAAFF`). The title, legend, header, footer and caption have their own style selectors:

```rockuml
@startuml
skinparam backgroundColor #FFFFFF-#DBEAFE
<style>
title {
  FontColor #1E3A8A
  BackgroundColor #BFDBFE
  LineColor #1E3A8A
  RoundCorner 10
}
legend {
  BackgroundColor #FEF9C3
}
</style>
title The Blue Lagoon
legend Water: very blue
Fish -> Coral : hides
@enduml
```

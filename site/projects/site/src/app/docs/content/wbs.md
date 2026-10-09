```rockuml tldr
@startwbs
* Build the Death Star
** Design
*** Superlaser
*** Thermal exhaust port
**** Make it small
** Construction
*** Hire contractors
*** Fire contractors
** Testing
*** Destroy Alderaan
@endwbs
```

A work breakdown structure divides a project into its deliverables, and those into smaller ones, until the pieces are small enough to plan. rockuml draws it as a tree with the project at the top. Breakdowns start with `@startwbs` and use the same asterisks as [mind maps](docs/mindmap).

## Levels

One asterisk is the project, two are its main parts, and so on. Each level is drawn under the one above it, and the deepest levels are listed downwards.

```rockuml
@startwbs
* Become Hokage
** Graduate from the academy
*** Learn shadow clones
*** Pass the exam
** Become a chunin
*** Survive the forest of death
*** Fight in the finals
** Save the village
@endwbs
```

### Left and right

`<` or `>` after the asterisks puts a node on the left or right side of its parent:

```rockuml
@startwbs
* Planet Express delivery
** Preparation
***< Fuel the ship
***< Find Fry
***> Wake Bender
***> Ignore the Professor
** Delivery
@endwbs
```

### Boxless nodes

An underscore draws a node without a box; a lone underscore makes an invisible node that only groups its children:

```rockuml
@startwbs
* Hogwarts curriculum
** Year one
***_ Charms
***_ Potions
***_ Herbology
** Year two
**_
*** Duelling club
*** Quidditch
@endwbs
```

## Text and colours

Nodes over several lines start with `:` and end with `;`, and take [formatting](docs/creole). A colour in brackets colours a node, and a stereotype at the end can name a [style](docs/styling#style-sheets).

```rockuml
@startwbs
*[#SkyBlue] Avengers Initiative
**[#Pink] Recruitment
***:Find the
<b>strongest</b> heroes;
*** Bribe Tony with snacks
**[#LightGreen] Headquarters
*** Stark Tower
@endwbs
```

## Links between nodes

Give nodes an alias, either in parentheses after the asterisks or with `"text" as alias`, and draw arrows between them with `alias -> alias`:

```rockuml
@startwbs
* Silicon Valley season 1
**(app) Build the app
***(algo) Middle-out algorithm
***(ui) The interface
**(pitch) TechCrunch Disrupt
***(demo) Live demo
algo -> demo
ui -> pitch
@endwbs
```

## Styling

Style sheets style all nodes, or nodes at certain depths, as in mind maps. `Width auto` sizes each box to its text:

```rockuml
@startwbs
<style>
wbsDiagram {
  node {
    BackgroundColor #FFF3D6
    LineColor #C2410C
  }
  :depth(0) {
    BackgroundColor #FB923C
    FontColor white
  }
  arrow {
    LineColor #C2410C
  }
}
</style>
* Mine diamonds
** Find a cave
*** Bring torches
*** Bring food
** Dig to level -59
*** Avoid lava
@endwbs
```

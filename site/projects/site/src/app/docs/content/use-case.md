```rockuml tldr
@startuml
left to right direction
actor Customer
actor "Kitchen staff" as Kitchen

rectangle "Krusty Burger" {
  usecase "Order a Krusty Burger" as Order
  usecase "Pay" as Pay
  usecase "Complain" as Complain
  Customer --> Order
  Order .> Pay : include
  Complain .> Order : extends
  Order <-- Kitchen
}
@enduml
```

A use case diagram shows who uses a system and what for: the *actors* outside, the *use cases* inside, and lines between them. It is the diagram for the first meeting of a project, when nobody agrees yet on what the system is supposed to do.

## Actors

An actor is a name between colons, or a declaration with `actor`. Quote names with spaces and give them a short alias with `as`:

```rockuml
@startuml
:Homer:
:Bart Simpson: as Bart
actor Lisa
actor "Marge Simpson" as Marge
actor :Maggie: as Baby
@enduml
```

`skinparam actorStyle` draws actors differently: `awesome` as a head and shoulders, `hollow` as an outline.

```rockuml
@startuml
skinparam actorStyle awesome
actor Picard
actor Riker
usecase "Make it so" as Command
Picard --> Command
Riker --> Command
@enduml
```

## Use cases

A use case is a name in parentheses, or a declaration with `usecase`:

```rockuml
@startuml
(Mine diamonds)
(Craft a pickaxe) as (Craft)
usecase Smelt
usecase (Build a house) as House
usecase "Tame a wolf" as Wolf
@enduml
```

Use cases can have descriptions over several lines, separated from the name by a line of dashes, dots or equal signs:

```rockuml
@startuml
usecase Heist as "Steal the Arkenstone
--
The plan
..
1. Wake the dragon
2. Run
"
@enduml
```

## Links

Connect actors and use cases with arrows: `-->` for an association with a head, `--` for a plain line, `..>` for a dashed arrow. A label goes after a colon. More dashes make a longer line.

```rockuml
@startuml
actor Batman
actor Robin
(Patrol Gotham) as Patrol
(Fight crime) as Fight
Batman --> Patrol
Batman ---> Fight : at night
Robin -- Fight
Robin ..> Patrol : sometimes
@enduml
```

### Include, extend and generalisation

Dashed arrows labelled `include` or `extends` show how use cases build on each other; a triangle head, `--|>`, shows that one actor or use case is a special kind of another.

```rockuml
@startuml
actor Hunter
actor "S-Class hunter" as SClass
SClass --|> Hunter
(Clear a dungeon) as Clear
(Fight the boss) as Boss
(Loot) as Loot
Hunter --> Clear
Clear .> Boss : include
Loot .> Clear : extends
@enduml
```

### Directions and styles

`-left->`, `-right->`, `-up->` and `-down->` place the far end of a link in that direction. A colour and a style after the link restyle it: `#red;line.dashed;text:red`.

```rockuml
@startuml
:Ash: as Ash
Ash -up-> (Catch them all)
Ash -left-> (Pick Pikachu)
Ash -right-> (Lose to Gary) #red;line.dashed;text:red : again
Ash -down-> (Become a master) #green;line.bold
@enduml
```

## Systems and packages

`rectangle` draws the system boundary around its use cases. `package` groups elements too, with a folder tab:

```rockuml
@startuml
left to right direction
actor Developer
actor "Product owner" as PO
rectangle "Hooli XYZ" {
  usecase "Ship the Box" as Box
  usecase "Rename everything" as Rename
}
package Marketing {
  usecase "Announce at keynote" as Keynote
}
Developer --> Box
PO --> Rename
PO --> Keynote
@enduml
```

`left to right direction` lays the diagram out from left to right, which usually suits use cases better than the default top-to-bottom layout.

## Business use cases

A slash after `actor` or `usecase` makes it a business actor or business use case, drawn with a stroke:

```rockuml
@startuml
actor/ "Iron Bank of Braavos" as Bank
usecase/ "Collect the debt" as Collect
actor Lannisters
Bank --> Collect
Collect --> Lannisters
@enduml
```

## Notes and stereotypes

Notes attach to elements with `note left of`, `right of`, `top of` or `bottom of`, or float on their own with an alias. Stereotypes, `<< like this >>`, label elements with their kind.

```rockuml
@startuml
actor Gandalf << wizard >>
usecase "You shall not pass" as Pass << spell >>
Gandalf --> Pass
note right of Pass : Balrogs only.
note "Fly, you fools!" as Shout
Shout .. Gandalf
@enduml
```

Colours work as everywhere: `actor Hulk #green`, `usecase Smash #lightgreen`. [Colours and styles](docs/styling) has the details.

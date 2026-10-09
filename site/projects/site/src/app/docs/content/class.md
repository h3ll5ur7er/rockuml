```rockuml tldr
@startuml
abstract class Hero {
  # name : String
  - secretIdentity : String
  + {abstract} usePower()
}
interface Flyer {
  + fly(altitude : int)
}
class IronMan {
  + usePower()
  + fly(altitude : int)
}
class Suit {
  mark : int
}
enum Team {
  AVENGERS
  GUARDIANS
}

Hero <|-- IronMan
Flyer <|.. IronMan
IronMan "1" *-- "many" Suit : owns >
IronMan --> Team
@enduml
```

A class diagram shows the types of a system, what they hold and do, and how they relate: inheritance, composition, dependencies. It is the standard map of an object-oriented code base, and also a fine way to model any domain, from a bookshop to the Marvel Cinematic Universe.

## Classes and their members

Declare a class with `class Name`, and list its members between braces. Members with parentheses are methods, the others fields; rockuml sorts them into two compartments.

```rockuml
@startuml
class Bender {
  model : String = "Bending Unit 22"
  serialNumber : int = 2716057
  bend(girder : Girder)
  drink(beer : Beer) : Fuel
  {static} kill(all : Humans)
}
@enduml
```

You can also add members one at a time, outside the braces, with `Class : member`:

```rockuml
@startuml
class Creeper
Creeper : fuse : int = 30
Creeper : charged : boolean
Creeper : hiss()
Creeper : explode()
@enduml
```

### Visibility

A first character `-`, `#`, `~` or `+` makes a member private, protected, package private or public, drawn as an icon:

```rockuml
@startuml
class Shinobi {
  - chakraNature : Element
  # village : String
  ~ rank : Rank
  + name : String
  - trainInSecret()
  # protectVillage()
  ~ reportToHokage()
  + performJutsu(signs : HandSign[])
}
@enduml
```

### Static and abstract members

`{static}` underlines a member and `{abstract}` writes it in italics. `{classifier}` is another word for static.

```rockuml
@startuml
abstract class Pokemon {
  {static} caughtSoFar : int
  {abstract} attack(target : Pokemon)
  {abstract} cry() : String
  evolve()
}
class Pikachu {
  attack(target : Pokemon)
  cry() : String
}
Pokemon <|-- Pikachu
@enduml
```

### Separators

Lines of `--`, `..`, `==` or `__` divide the members into more compartments, with an optional title between the marks:

```rockuml
@startuml
class StrawHatPirates {
  captain : Luffy
  .. crew ..
  swordsman : Zoro
  navigator : Nami
  == treasure ==
  bounty : Berry
  __ goals __
  findOnePiece()
}
@enduml
```

## Kinds of classes

Other keywords declare other kinds of types; each has its own letter in the circle:

```rockuml
@startuml
abstract class Vehicle
class Car
interface Drivable
annotation Autonomous
enum Fuel
entity Driver
exception OutOfGas
struct Coordinates
protocol Honkable
@enduml
```

`record` and `dataclass` work like `class` with their own spot, and `circle` and `diamond` draw small shapes for connection points.

### Enums

An enum's values go between braces:

```rockuml
@startuml
enum Hogwarts {
  GRYFFINDOR
  HUFFLEPUFF
  RAVENCLAW
  SLYTHERIN
}
@enduml
```

### Generics

Type parameters go in angle brackets after the name:

```rockuml
@startuml
class Box<T> {
  open() : T
}
class InfinityGauntlet<S extends Stone>
interface Supplier<T> {
  get() : T
}
Supplier <|.. Box
@enduml
```

### Stereotypes and spots

A stereotype, `<< Name >>`, labels a class with its role. `<< (S,#FF7700) Singleton >>` also changes the letter in the circle and its colour.

```rockuml
@startuml
class Rick << (G,#7FD1F7) Genius >>
class PortalGun << (P,lightgreen) >>
class Morty << Grandson >>
Rick --> PortalGun
Rick --> Morty : drags along
@enduml
```

## Relations

Relations are lines between classes. Their ends say what they mean:

| Relation | Syntax | Meaning |
|---|---|---|
| Extension | `<\|--` | is a kind of (inheritance) |
| Implementation | `<\|..` | implements an interface |
| Composition | `*--` | is made of, and owns |
| Aggregation | `o--` | has, without owning |
| Dependency | `-->` or `..>` | uses |
| Association | `--` | is related to |
| Dashed link | `..` | loosely related to |

```rockuml
@startuml
Avenger <|-- Hulk
Shapeshifter <|.. Loki
Mjolnir *-- Uru
Asgard o-- Asgardian
Thor --> Mjolnir
Strange ..> TimeStone
Thor -- Loki
Loki .. Thanos
@enduml
```

The ends can point either way (`--|>`, `--*`, `--o`) and there are more heads: `#--`, `x--`, `}--`, `+--` and `^--`. One dash (`->`) makes a shorter line that lays out sideways; more dashes make it longer.

### Labels and multiplicities

A label after a colon names the relation; `>` or `<` at its end shows which way to read it. Quoted texts next to the classes are multiplicities.

```rockuml
@startuml
class Jedi
class Padawan
class Lightsaber
Jedi "1" -- "0..1" Padawan : trains >
Jedi "1" *-- "1..*" Lightsaber : builds >
Padawan "1" --> "1" Lightsaber : borrows <
@enduml
```

### Association classes and qualifiers

An association class hangs off the relation between two classes with `(A, B) .. C`. A qualifier is written in brackets at the end of the relation:

```rockuml
@startuml
class Student
class House
class Sorting {
  hatMood : String
}
Student "*" -- "1" House
(Student, House) .. Sorting

class GringottsBank
class Vault
GringottsBank [vaultNumber] --> "0..1" Vault
@enduml
```

### Lollipop interfaces

`()-` draws a provided interface as a lollipop:

```rockuml
@startuml
class CentralPerk
class Gunther
CentralPerk ()- Gunther : serves coffee
@enduml
```

### Relation styles and directions

Like other links, relations take colours and styles, `#red;line.dashed`, and directions: `-up-|>`, `-left->`. `left to right direction` turns the whole layout.

```rockuml
@startuml
left to right direction
class "Stark Tower" as Tower
class "Avengers Tower" as Avengers
Tower -[#red,bold]-> Avengers : renamed
Avengers -[#blue,dashed]-> Compound : moved
@enduml
```

## Packages and namespaces

`package` groups classes in a folder. Packages nest, and take styles: `<<Node>>`, `<<Rectangle>>`, `<<Folder>>`, `<<Frame>>`, `<<Cloud>>` and `<<Database>>`.

```rockuml
@startuml
package "Planet Express" {
  class Fry
  class Leela
  package "Robots" <<Rectangle>> {
    class Bender
  }
}
package "Mom Corp" <<Cloud>> {
  class Mom
}
Fry --> Bender : best friend
Mom --> Bender : built
@enduml
```

`namespace` works the same way, and also qualifies names: `net.dummy.Person` is a different class from `net.foo.Person`.

```rockuml
@startuml
namespace shinobi.leaf {
  class Naruto
  class Kakashi
  Kakashi --> Naruto : teaches
}
namespace shinobi.sand {
  class Gaara
}
shinobi.leaf.Naruto -- shinobi.sand.Gaara : friends
@enduml
```

`together { … }` keeps classes close to each other without drawing a frame.

## Notes

Notes attach to classes (`note left of Hero`), to members (`note right of Hero::name`) or to the last relation (`note on link`), or float with an alias:

```rockuml
@startuml
class Ring {
  power : Unlimited
  bearer : Hobbit
}
Ring --> MountDoom : destroyed in
note on link : eventually
note right of Ring::bearer
  usually the wrong one
end note
note "One Ring to rule them all" as Inscription
Inscription .. Ring
@enduml
```

## Showing and hiding

`hide` and `show` choose what is drawn: `hide empty members`, `hide fields`, `hide methods`, `hide circle`, `hide stereotype`, or the same for a single class (`show Hero fields`).

```rockuml
@startuml
hide empty members
hide circle
class Sheldon {
  iq : int = 187
  knock(times : 3)
}
class Penny
class Leonard
Sheldon --> Penny : "Penny, Penny, Penny"
Sheldon --> Leonard : roommate agreement
@enduml
```

## Mixing in other elements

`allowmixing` lets a class diagram contain components, actors, use cases and the other elements of [component](docs/component) and [use case](docs/use-case) diagrams:

```rockuml
@startuml
allowmixing
actor "Jian Yang" as Jian
component "SeeFood app" as App
class Classifier {
  isHotDog(image) : boolean
}
Jian --> App
App --> Classifier
@enduml
```

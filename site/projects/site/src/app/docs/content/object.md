```rockuml tldr
@startuml
object "Monkey D. Luffy" as Luffy {
  bounty = 3 000 000 000
  devilFruit = "Gomu Gomu"
}
object Zoro {
  swords = 3
}
object "Thousand Sunny" as Sunny
map Crew {
  captain *-> Luffy
  swordsman *-> Zoro
}
Luffy --> Sunny : sails
@enduml
```

An object diagram is a snapshot: instead of the classes of a system, it shows particular objects, the values of their fields, and the links between them at one moment. Use it to explain an example, a test fixture, or what your data structure looks like after the third bug report.

## Objects

Declare an object with `object`, and give it fields between braces with `name = value`. Quoted names with spaces take a short alias with `as`.

```rockuml
@startuml
object Hal9000 {
  mission = "Jupiter"
  podBayDoors = closed
  mood = "I'm sorry, Dave"
}
object "Dave Bowman" as Dave
Dave --> Hal9000 : open the pod bay doors
@enduml
```

Fields can also be added from outside, one line each:

```rockuml
@startuml
object enterprise
enterprise : registry = "NCC-1701-D"
enterprise : captain = "Picard"
enterprise : warp = 9.6
@enduml
```

## Links

Objects link like classes do, with every [relation](docs/class#relations) arrow, label and multiplicity:

```rockuml
@startuml
object Sheldon
object Leonard
object Penny
object Apartment4A
Sheldon "1" -- "1" Leonard : roommates
Leonard --> Penny : dates
Sheldon *-- Apartment4A : spot on the couch
Penny ..> Apartment4A : borrows wifi
@enduml
```

A `diamond` joins several links into one association:

```rockuml
@startuml
object Harry
object Ron
object Hermione
object "Defence Association" as DA
diamond trio
Harry --> trio
Ron --> trio
Hermione --> trio
trio --> DA : founds
@enduml
```

## Maps

`map` draws a table of keys and values, written `key => value`:

```rockuml
@startuml
map "Tailed beasts" as Beasts {
  One => Shukaku
  Seven => Chōmei
  Nine => Kurama
}
@enduml
```

A key can point at another object with `*->` instead of `=>`, and links can start at a single key with `Map::key`:

```rockuml
@startuml
object Naruto
object Gaara
map Jinchuriki {
  Kurama *-> Naruto
  Shukaku *--> Gaara
}
map Villages {
  Naruto => Leaf
  Gaara => Sand
}
Villages::Naruto --> Naruto
@enduml
```

## JSON objects

`json` puts a whole [JSON document](docs/json) in the diagram as an object, so you can show a payload next to the objects it describes:

```rockuml
@startuml
json Shipment {
  "id": 3000,
  "from": "Planet Express",
  "to": "Omicron Persei 8",
  "contents": ["popplers", "more popplers"]
}
object Leela {
  role = "captain"
}
Leela --> Shipment : delivers
@enduml
```

## Packages, notes and colours

Objects go into `package`s, take notes and stereotypes, and have colours like any other element:

```rockuml
@startuml
package "Mines of Moria" {
  object Gandalf #LightGray {
    colour = grey
  }
  object Balrog #OrangeRed {
    mood = furious
  }
}
Gandalf --> Balrog : you shall not pass
note bottom of Gandalf : comes back as Gandalf the White
@enduml
```

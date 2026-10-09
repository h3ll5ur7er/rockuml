```rockuml tldr
@startuml
[*] --> Sleeping
Sleeping --> Hungry : wakes up
Hungry --> Eating : finds meat
state Eating {
  [*] --> Chewing
  Chewing --> Swallowing
  Swallowing --> Chewing : more meat
}
Eating --> Sleeping : full
Eating --> Fighting : someone took the meat
Fighting --> [*] : wins
@enduml
```

A state diagram shows the states something can be in and the events that move it from one to the next. It is how you describe the life of an order, a connection, a game character, or Luffy on a typical day.

## States and transitions

A transition is an arrow between two states, with the event that causes it after a colon. States are created as they appear; `[*]` is the start when it is on the left of an arrow and the end when it is on the right.

```rockuml
@startuml
[*] --> Pending
Pending --> Approved : manager signs
Pending --> Rejected : TPS report missing
Approved --> [*]
Rejected --> Pending : add cover sheet
@enduml
```

One dash (`->`) draws a shorter, sideways transition. `-up->`, `-down->`, `-left->` and `-right->` choose the direction.

### Descriptions

`state Name : text` adds a line of description to a state; several lines add up. `state "Long name" as Alias` gives a long name an alias.

```rockuml
@startuml
state "Super Saiyan" as SSJ
state Normal : hair: black
SSJ : hair: blond
SSJ : power level: over 9000
[*] --> Normal
Normal --> SSJ : gets really angry
SSJ --> Normal : runs out of energy
@enduml
```

## Composite states

A state with braces contains its own states and transitions, to any depth. Transitions can enter and leave it from outside.

```rockuml
@startuml
[*] --> Hogwarts
state Hogwarts {
  [*] --> FirstYear
  FirstYear --> SecondYear : survives
  state SecondYear {
    [*] --> Classes
    Classes --> ChamberOfSecrets : follows the spiders
    ChamberOfSecrets --> Classes : defeats the basilisk
  }
  SecondYear --> ThirdYear
}
Hogwarts --> [*] : graduates
@enduml
```

### Concurrent states

Inside a composite state, `--` separates regions that are active at the same time, drawn one above the other. `||` puts them side by side.

```rockuml
@startuml
[*] --> Avengers
state Avengers {
  [*] --> Assembling
  Assembling --> Fighting
  --
  [*] --> Bickering
  Bickering --> Bickering : Tony and Steve
}
@enduml
```

```rockuml
@startuml
state Kitchen {
  [*] --> Cooking
  Cooking --> Burning : Homer cooks
  ||
  [*] --> Waiting
  Waiting --> Ordering : gives up
}
@enduml
```

## Pseudo-states

Stereotypes turn states into the special points of UML:

| Stereotype | Shape |
|---|---|
| `<<choice>>` | a diamond that picks one way out |
| `<<fork>>`, `<<join>>` | bars that split or join concurrent flows |
| `<<start>>`, `<<end>>` | the start and end circles, with a name |
| `<<entryPoint>>`, `<<exitPoint>>` | circles on the edge of a composite state |
| `<<inputPin>>`, `<<outputPin>>` | small squares on the edge |
| `<<expansionInput>>`, `<<expansionOutput>>` | expansion nodes |
| `<<sdlreceive>>` | an SDL receive signal |

```rockuml
@startuml
state "Choose a starter" as Choose
state pick <<choice>>
[*] --> Choose
Choose --> pick
pick --> Charmander : [likes fire]
pick --> Squirtle : [likes water]
pick --> Bulbasaur : [plays the long game]
state done <<end>>
Charmander --> done
@enduml
```

```rockuml
@startuml
state split <<fork>>
state merge <<join>>
[*] --> split
split --> Frodo : to Mordor
split --> Aragorn : to Gondor
Frodo --> merge
Aragorn --> merge
merge --> Coronation
Coronation --> [*]
@enduml
```

```rockuml
@startuml
state Shire {
  state Bag_End <<entryPoint>>
  state Party
  Bag_End --> Party
  Party --> Leaving
  Leaving --> Road <<exitPoint>>
}
[*] --> Bag_End
Road --> Rivendell
@enduml
```

### History

`Name[H]` is the shallow history of a composite state and `Name[H*]` its deep history: going back to it resumes where it left off.

```rockuml
@startuml
[*] --> Playing
state Playing {
  [*] --> Level1
  Level1 --> Level2
  Level2 --> Level3
}
Playing --> Paused : pause
Paused --> Playing[H] : resume
Paused --> Playing[H*] : resume, deep
@enduml
```

## Arrows

Transitions take colours and styles in brackets:

```rockuml
@startuml
[*] --> Green
Green -[#green]-> Yellow : timer
Yellow -[#orange,bold]-> Red : timer
Red -[dashed]-> Green : timer
Red -[#red,dotted]-> [*] : power cut
@enduml
```

## Notes

Notes go beside a state, on the last transition with `note on link`, or float with an alias:

```rockuml
@startuml
[*] --> Alive
Alive --> Dead : Kenny
note on link
  happens in
  every episode
end note
Dead --> Alive : next episode
note right of Dead : they killed Kenny!
@enduml
```

## Colours and styles

A colour after a state colours it, and an inline style stereotype sets more: `<<BackGroundColor:pink;LineColor:red>>`.

```rockuml
@startuml
state Calm #LightGreen
state Angry <<BackGroundColor:#9ACD32;LineColor:darkgreen;FontColor:white>>
[*] --> Calm
Calm --> Angry : provoked
Angry --> Calm : sunset
note right of Angry : Hulk smash
@enduml
```

`hide empty description` hides the empty compartment of states without descriptions, and `left to right direction` lays the diagram out sideways.

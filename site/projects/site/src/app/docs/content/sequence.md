```rockuml tldr
@startuml
autonumber
actor "Tony Stark" as Tony
participant JARVIS
database "Stark Industries\narchive" as Archive

Tony -> JARVIS ++ : run the suit diagnostics
JARVIS -> Archive : load Mark 42 specs
Archive --> JARVIS : specs
alt suit is ready
  JARVIS --> Tony : all systems go
else repulsors offline
  JARVIS --> Tony : I'd advise against flying, sir
end
deactivate JARVIS
note over Tony, JARVIS : He flies anyway.
@enduml
```

A sequence diagram shows the messages that participants send each other, from top to bottom in the order they happen. Participants stand in a row along the top, each with a dashed lifeline running down; every message is an arrow from one lifeline to another. They are the best diagram for explaining a protocol, an API call or a support ticket's journey through five departments.

## Messages

A message is a line `Sender -> Receiver : text`. Participants you haven't declared are created as you mention them, in that order. The arrow says what kind of message it is:

| Arrow | Meaning |
|---|---|
| `->` | a call, or any message |
| `-->` | a reply (dashed) |
| `->>` | an asynchronous message (thin head) |
| `-\` and `-/` | half arrow heads, top or bottom |
| `->x` | a message that is lost |
| `->o` | a message ending in a circle |
| `<->` | both ways |
| `<-`, `<--` | the same arrows, pointing left |

```rockuml
@startuml
Arthur -> Ford : What's happening?
Ford --> Arthur : The world's about to end.
Ford ->> Vogons : thumbs a ride
Vogons ->x Arthur : poetry reading
Arthur <-> Ford : panic, together
Ford -\ Arthur : towel?
Arthur -/ Ford : towel!
Zaphod o->o Trillian : hey, Earth girl
@enduml
```

A message can go from a participant to itself, and its text can span several lines with `\n`:

```rockuml
@startuml
Gilfoyle -> Gilfoyle : refactor the server\nwhile Dinesh isn't looking
Dinesh -> Gilfoyle : did you touch my code?
Gilfoyle --> Dinesh : I improved your code.\nThat's different.
@enduml
```

### Colours and line styles

Put a colour in brackets inside the arrow to colour one message, and add `bold`, `dashed` or `dotted` to change its line:

```rockuml
@startuml
Hulk -[#green]> Loki : puny god
Loki -[#purple,dashed]> Hulk : I am a god!
Thor -[#0000FF,bold]> Loki : brother…
@enduml
```

### Messages from and to the edge

`[->` and `->]` draw a message from or to the edge of the diagram, for things that come from outside it. `?->` and `->?` draw a short arrow that stops near the lifeline instead.

```rockuml
@startuml
participant "Hokage office" as Office
[-> Office : mission request
Office -> Naruto : you're going
Naruto ->] : leaves the village
?-> Naruto : a kunai, from nowhere
Naruto ->? : shadow clones, everywhere
@enduml
```

### Several receivers

`&` sends one message to several participants at once:

```rockuml
@startuml
participant Luffy
participant Zoro
participant Nami
participant Sanji
Luffy -> Zoro & Nami & Sanji : I'm hungry!
Sanji -> Luffy : dinner's ready
@enduml
```

### Messages that take time

`->(10)` slants a message by 10 pixels, for messages that take a while to arrive. Write the delay on the other side, `(10)<-`, for arrows that point left.

```rockuml
@startuml
Earth ->(30) Mars : "hello" (takes 12 minutes)
Mars ->(30) Earth : "hi" (another 12)
@enduml
```

## Participants

Declare participants to choose their shape, their order and their name. The declaration's keyword picks the shape:

```rockuml
@startuml
participant Participant
actor Actor
boundary Boundary
control Control
entity Entity
database Database
collections Collections
queue Queue
@enduml
```

`as` gives a participant a short alias, so a long or quoted name doesn't need repeating. Participants appear in the order they are declared, whatever order the messages mention them in:

```rockuml
@startuml
participant "Central Perk" as Cafe
actor "Rachel Green" as Rachel #pink
actor Ross
Ross -> Rachel : we were on a break!
Rachel -> Cafe : one coffee, please
@enduml
```

A colour after the declaration colours the participant. Names can hold line breaks (`"Planet\nExpress"`) and [text formatting](docs/creole). `order 10` after a declaration sorts participants by number instead of by declaration.

### Boxes

`box` groups participants under a heading, with an optional colour, until `end box`:

```rockuml
@startuml
box "Planet Express" #LightBlue
  participant Fry
  participant Leela
end box
box "Mom's Friendly Robot Company" #MistyRose
  participant Mom
end box
Fry -> Leela : can I drive?
Leela --> Fry : no
Mom -> Fry : I'll buy the company
@enduml
```

### Creating and destroying participants

`create` makes a participant appear at the height of the message that creates it, rather than at the top. `destroy` ends its lifeline with a cross.

```rockuml
@startuml
participant Steve
Steve -> Steve : place 4 iron blocks\nand a pumpkin
create "Iron Golem" as Golem
Steve -> Golem : spawns
Golem -> Zombie : smash
destroy Zombie
@enduml
```

## Activation

An activation bar on a lifeline shows that a participant is busy. `activate` and `deactivate` draw it; activations can nest and have colours.

```rockuml
@startuml
participant Morty
participant Rick
Morty -> Rick : what is my purpose?
activate Rick
Rick -> Rick : think
activate Rick #DarkSalmon
Rick --> Rick : …
deactivate Rick
Rick --> Morty : you pass butter
deactivate Rick
@enduml
```

The shortcuts are quicker: after the receiver, `++` activates it, `--` deactivates the sender, `**` creates the receiver and `!!` destroys it. `return` answers the most recent activation and deactivates it.

```rockuml
@startuml
Client -> API ++ : GET /answer
API -> Cache ++ : lookup
return miss
API -> DeepThought ** : compute
DeepThought -> DeepThought : 7.5 million years
API -> DeepThought !! : cancel, too slow
return 503 Service Unavailable
@enduml
```

`autoactivate on` activates every receiver of a message automatically, and every reply deactivates it again.

## Numbering

`autonumber` numbers the messages. Give it a start, a step and a format: `autonumber 10 5 "<b>[000]"` starts at 10, counts in fives and pads to three digits in bold. `autonumber stop` and `autonumber resume` pause and continue the count.

```rockuml
@startuml
autonumber 42 "<b>Q000"
Arthur -> Computer : what's the question?
Computer --> Arthur : you'll need a bigger computer
autonumber stop
Arthur -> Arthur : (sigh)
autonumber resume
Computer -> Earth : designs it
@enduml
```

Numbers can also have several levels: `autonumber 1.1.1` numbers in a dotted form, and `autonumber inc A` steps the first level.

## Grouping messages

Frames group messages. `alt` and `else` show alternatives; `opt` an optional part; `loop` repetition; `par` messages in parallel; `break` an early way out; `critical` a part that mustn't be interrupted; `group` anything else, with a heading of your choice. Each frame closes with `end`, and they nest.

```rockuml
@startuml
actor Customer
participant "Pied Piper" as PP
database Storage

Customer -> PP : upload(file)
alt file is a video
  PP -> PP : middle-out compression
else file is a hot dog picture
  PP -> PP : not hot dog
end
loop for each shard
  PP -> Storage : store shard
  opt storage is full
    Storage --> PP : ask Gilfoyle
  end
end
group Weissman score [measured by Hooli]
  PP --> Customer : 5.2
end
@enduml
```

The text in brackets after a `group` heading is a second, smaller label.

## Notes

Notes sit `left of`, `right of` or `over` a participant, or `over` several at once. A note right after a message is attached to it. Notes can have colours, several lines (ending in `end note`) and [formatting](docs/creole).

```rockuml
@startuml
participant Naruto
participant Sasuke
participant Sakura
Naruto -> Sasuke : I'll bring you back!
note left : he means it
note right of Sasuke #lightblue : he doesn't care
note over Naruto, Sakura
  Team 7,
  <i>reunited</i> (eventually)
end note
@enduml
```

`hnote` draws a hexagonal note and `rnote` a rectangular one; `note across` spans every participant.

```rockuml
@startuml
participant Server
participant Client
hnote over Server : idle
Client -> Server : connect
rnote over Server : connected
note across : the boring part is over
@enduml
```

## Dividers, delays and space

`== text ==` divides a diagram into phases. `...` is a delay, `... 5 minutes later ...` a delay with a label. `|||` adds some space and `||45||` a given amount.

```rockuml
@startuml
== Initialisation ==
Gandalf -> Frodo : keep it secret
...
... 17 years later ...
Gandalf -> Frodo : keep it safe
|||
== Departure ==
Frodo -> Sam : let's go
||45||
Sam -> Frodo : I'm coming too
@enduml
```

## References

`ref over` draws a box that refers to another diagram, across one or more participants:

```rockuml
@startuml
participant Hogwarts
participant Harry
participant Voldemort
Harry -> Hogwarts : arrives
ref over Harry, Voldemort : the events of books one to six
Harry -> Voldemort : expelliarmus
@enduml
```

## Parallel messages and durations

A message starting with `&` happens at the same time as the one before it. `{name}` in front of a message marks its moment as an anchor, and `{start} <-> {end}` then shows the time between two anchors:

```rockuml
@startuml
participant Thanos
participant Avengers
participant "Ant-Man" as AntMan
participant "Quantum realm" as Quantum
{snap} Thanos -> Avengers : snap
& AntMan -> Quantum : gets stuck
|||
{back} Quantum -> AntMan : lets him out
AntMan -> Avengers : time heist?
{snap} <-> {back} : five years
@enduml
```

## Display options

- `hide footbox` removes the participants repeated at the bottom.
- `hide unlinked` hides declared participants nobody sends a message to.
- `mainframe text` frames the whole diagram with a heading, as in UML.
- `skinparam sequenceMessageAlign center` centres message texts; `left` and `right` are the other choices.
- `skinparam responseMessageBelowArrow true` writes reply texts under their arrows.
- `skinparam maxMessageSize 100` wraps message texts at 100 pixels.
- `skinparam lifelineStrategy solid` draws solid lifelines.

```rockuml
@startuml
hide footbox
mainframe sd Login
skinparam sequenceMessageAlign center
skinparam responseMessageBelowArrow true
actor Kirito
participant "Sword Art Online" as SAO
Kirito -> SAO : log in
SAO --> Kirito : you cannot log out
Kirito -> SAO : wait, what?
@enduml
```

## Splitting a long sequence

`newpage` starts a new page, with an optional title, as described in [Common commands](docs/basics#several-pages). Activations and numbering carry on across the pages.

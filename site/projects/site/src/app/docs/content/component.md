```rockuml tldr
@startuml
package "Pied Piper platform" {
  [Web app] as Web
  [Compression engine] as Engine
  interface "Compression API" as API
  database "Storage" as DB
}
cloud "Hooli Cloud" {
  [CDN]
}
Web --> API
API - Engine
Engine --> DB : stores shards
Web ..> CDN : static files
@enduml
```

A component diagram shows how a system is built from parts: components, the interfaces they offer, and how they depend on each other. It works at any scale, from the modules of one program to the services of a whole company.

## Components

A component is a name in brackets, or a declaration with `component`. Quote long names and give them an alias with `as`.

```rockuml
@startuml
[Flux capacitor]
[Mr. Fusion] as Fusion
component TimeCircuits
component [DeLorean dashboard] as Dash
component "Hoverboard conversion" as Hover
@enduml
```

A description in brackets over several lines replaces the name inside the box, with separators if you like:

```rockuml
@startuml
component JARVIS [
Just A Rather
Very Intelligent System
----
version: Mark II
]
component FRIDAY [
the replacement
]
JARVIS --> FRIDAY : succeeded by
@enduml
```

## Interfaces

An interface is `()` before a name, or a declaration with `interface`. A plain line from a component to an interface means the component provides it; a dependency arrow means it uses it.

```rockuml
@startuml
() "Holodeck API" as Holo
interface "Warp drive" as Warp
[Holodeck] - Holo
[Bridge] ..> Holo : use
[Engineering] - Warp
[Bridge] --> Warp : engage
@enduml
```

`-(` draws the receiving end of an interface, as a socket:

```rockuml
@startuml
DeathStarAPI - [Death Star]
[Rebel Alliance] -( DeathStarAPI
@enduml
```

### UML 1 and UML 2 styles

`skinparam componentStyle uml1` draws components in the older style with two small rectangles on the side; `uml2`, the default, draws an icon instead. `rectangle` drops the decoration entirely.

```rockuml
@startuml
skinparam componentStyle uml1
interface "Data access" as DA
DA - [Hogwarts library]
[Hogwarts library] ..> "Restricted section" : use
@enduml
```

## Ports

Components can have ports on their edge: `port`, `portin` (drawn on the incoming side) and `portout` (on the outgoing side).

```rockuml
@startuml
component "Arc reactor" as Reactor {
  portin power
  portout repulsors
  portout chest
  component Core
}
[Palladium] --> power
power --> Core
Core --> repulsors
Core --> chest
@enduml
```

## Grouping

`package`, `node`, `folder`, `frame`, `cloud`, `database` and `rectangle` group components. They nest to any depth.

```rockuml
@startuml
node "Thousand Sunny" {
  [Kitchen]
  [Aquarium bar]
}
folder "Log pose" {
  [Map collection]
}
frame "Grand Line" {
  [Sabaody]
}
cloud {
  [Sky island]
}
database "Marine HQ" {
  [Bounty posters]
}
[Kitchen] --> [Aquarium bar]
[Map collection] --> [Sabaody]
[Sabaody] ..> [Bounty posters]
@enduml
```

## Links

Links between components take labels, directions and colours like everywhere else. Any [deployment](docs/deployment) element, such as `node`, `database` or `actor`, can join a component diagram.

```rockuml
@startuml
actor Customer
[Shop] -right-> [Payment] : charges
[Shop] -down-> [Warehouse] #blue;line.dashed : ships
Customer --> [Shop]
[Payment] ..> [Bank] #red : talks to
@enduml
```

## Notes and stereotypes

Notes attach to components and interfaces; stereotypes label them with their kind.

```rockuml
@startuml
component "SkyNet" << AI >> as SkyNet
interface "Defence grid" as Grid
SkyNet - Grid
note right of SkyNet
  Became self-aware
  at 2:14 a.m.
end note
note left of Grid : do not connect
@enduml
```

## Hiding unlinked elements

`remove @unlinked` drops every element without a link, `hide @unlinked` leaves a gap where it was. That lets you keep one big list of components and draw only the connected ones.

```rockuml
@startuml
component Batmobile
component Batcave
component Batcomputer
component "Bat-shark repellent" as Repellent
Batmobile --> Batcave
Batcave --> Batcomputer
remove @unlinked
@enduml
```

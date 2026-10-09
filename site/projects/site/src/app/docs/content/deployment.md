```rockuml tldr
@startuml
actor User
cloud "Hooli Cloud" {
  node "Load balancer" as LB
  node "App server" {
    artifact "nucleus.war" as App
  }
  database "Postgres" as DB
  queue "Gavin's inbox" as Queue
}
User --> LB : HTTPS
LB --> App
App --> DB : SQL
App ..> Queue : complaints
@enduml
```

A deployment diagram shows where software runs: machines, containers, clouds and databases, and what is deployed on them. It is the picture that goes on the wall of the operations team, and the first thing everybody looks at when production is down.

## Elements

Each keyword draws its own shape, with the name inside:

```rockuml
@startuml
artifact artifact
card card
cloud cloud
file file
folder folder
frame frame
hexagon hexagon
node node
package package
queue queue
rectangle rectangle
stack stack
storage storage
@enduml
```

There are more: `action`, `circle`, `collections`, `component`, `database`, `interface`, `label` and `process`, as well as the people of use case diagrams, `actor`, `agent`, `boundary`, `control`, `entity` and `person`:

```rockuml
@startuml
database database
collections collections
process process
interface interface
label label
person person
agent agent
actor actor
@enduml
```

Quote names with spaces and give them an alias with `as`. A colour after the element colours it.

```rockuml
@startuml
node "TARDIS" as Tardis #0B3D91
storage "Bigger on the inside" as Inside
Tardis --> Inside
@enduml
```

### Descriptions

A description in brackets after a plain name can span several lines, with separators:

```rockuml
@startuml
node Enterprise [
<b>USS Enterprise</b>
----
NCC-1701-D
====
Galaxy class
]
artifact WarpCore [
dilithium
....
do not touch
]
Enterprise --> WarpCore
@enduml
```

## Nesting

Elements with braces contain other elements, to any depth:

```rockuml
@startuml
cloud "Matrix" {
  node "Zion mainframe" {
    artifact "Agent Smith"
    artifact "Oracle"
  }
  storage "Construct" {
    file "kung-fu.skill"
    file "helicopter.skill"
  }
}
node "Nebuchadnezzar" as Neb
Neb --> "kung-fu.skill" : downloads
"Agent Smith" ..> "Oracle" : hunts
@enduml
```

## Links

Links connect any two elements. The line can be plain (`--`), dashed (`..`), wavy (`~~`) or double (`==`), and the end can have many heads: `-->`, `--o`, `--*`, `--+`, `--#`, `-->>` and `--0`.

```rockuml
@startuml
node Earth
node Moon
node Mars
node Jupiter
node Saturn
Earth -- Moon
Earth .. Mars : someday
Mars ~~ Jupiter : asteroid belt
Jupiter == Saturn
Earth --> Mars
Mars --o Jupiter
Jupiter --* Saturn
@enduml
```

Labels, colours, styles and directions work as on other diagrams:

```rockuml
@startuml
left to right direction
node "Iron Throne" as Throne
node Winterfell
node "The Wall" as Wall
Throne -[#gold,bold]-> Winterfell : taxes
Winterfell -[#gray,dashed]-> Wall : sends recruits
Wall -[#lightblue]-> Winterfell : winter is coming
@enduml
```

## Notes and stereotypes

```rockuml
@startuml
node "Death Star" << battle station >> as DS
database "Plans" << secret >> as Plans
DS --> Plans
note right of Plans
  small thermal exhaust port,
  right below the main port
end note
@enduml
```

Deployment elements also mix into [component diagrams](docs/component) and, with `allowmixing`, into [class diagrams](docs/class#mixing-in-other-elements).

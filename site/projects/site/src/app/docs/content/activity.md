```rockuml tldr
@startuml
start
:Wake up in Minecraft;
if (Is it night?) then (yes)
  :Hide in the dirt hut;
else (no)
  repeat
    :Mine;
  repeat while (Found diamonds?) is (no)
  :Craft a diamond pickaxe;
endif
fork
  :Build a house;
fork again
  :Tame a wolf;
end fork
stop
@enduml
```

An activity diagram is a flowchart: actions in rounded boxes, decisions in diamonds, and arrows for the flow between them. It handles parallel work, loops and responsibilities (swimlanes) too, which makes it the diagram for business processes, algorithms and CI pipelines.

## Actions

An action is text between a colon and a semicolon. Actions follow each other from top to bottom; `start` and `stop` draw the beginning and the end, and `end` an end that looks like a target.

```rockuml
@startuml
start
:Make a towel;
:Learn the Vogon poetry by heart,
just in case;
:Hitchhike off the planet;
stop
@enduml
```

An action can span several lines and use [text formatting](docs/creole), including lists and links:

```rockuml
@startuml
start
:**Rules of Fight Club**
----
* do not talk about Fight Club
* do **not** talk about Fight Club;
:Read [[https://plantuml.com the manual]];
stop
@enduml
```

### Shapes

A stereotype at the end of an action changes its shape. The SDL shapes are `<<input>>`, `<<output>>`, `<<procedure>>`, `<<load>>`, `<<save>>`, `<<continuous>>` and `<<task>>`:

```rockuml
@startuml
start
:Read the coffee order; <<input>>
:Brew the coffee; <<procedure>>
:Save it as a regular; <<save>>
:Serve the coffee; <<output>>
stop
@enduml
```

The UML shapes are `<<object>>`, `<<objectSignal>>`, `<<acceptEvent>>`, `<<sendSignal>>`, `<<trigger>>` and `<<timeEvent>>`:

```rockuml
@startuml
start
:Every 30 seconds; <<timeEvent>>
:Rachel walks in; <<acceptEvent>>
:Ross panics; <<trigger>>
:Text Chandler; <<sendSignal>>
:The couch; <<object>>
stop
@enduml
```

## Decisions

`if (condition) then (label)`, `else (label)` and `endif` branch the flow. `elseif` adds more branches.

```rockuml
@startuml
start
:Choose a house;
if (Brave?) then (yes)
  :Gryffindor;
elseif (Loyal?) then (yes)
  :Hufflepuff;
elseif (Clever?) then (yes)
  :Ravenclaw;
else (ambitious)
  :Slytherin;
endif
:Start the school year;
stop
@enduml
```

A branch without `else` simply joins the flow again. `skinparam conditionStyle diamond` writes the condition inside the diamond instead of beside it.

### Switches

`switch`, `case` and `endswitch` choose between many branches at once:

```rockuml
@startuml
start
switch (Which pill?)
case (red)
  :See how deep\nthe rabbit hole goes;
case (blue)
  :Wake up in bed;
  :Believe whatever you want;
case (both)
  :That's not how it works;
endswitch
stop
@enduml
```

## Loops

`repeat` … `repeat while (condition)` runs its actions at least once. `is (label)` and `not (label)` label the arrows, and `backward:` adds an action on the way back.

```rockuml
@startuml
start
repeat
  :Write the code;
  :Run the tests;
backward:Blame Dinesh;
repeat while (Tests pass?) is (no) not (yes)
:Deploy to production;
stop
@enduml
```

`while (condition) is (label)` … `endwhile (label)` checks first; `break` leaves a loop early.

```rockuml
@startuml
start
while (Hungry?) is (yes)
  :Eat a Scooby snack;
  if (Ghost?) then (yes)
    :Run;
    break
  endif
endwhile (no)
:Unmask the villain;
stop
@enduml
```

## Parallel flows

`fork`, `fork again` and `end fork` run branches at the same time and wait for all of them. `end merge` joins them without waiting. `split` … `split again` … `end split` splits the flow without the synchronisation bars.

```rockuml
@startuml
start
:Assemble the Avengers;
fork
  :Iron Man holds the portal;
fork again
  :Hulk smashes;
fork again
  :Hawkeye shoots arrows;
end fork
:Shawarma;
stop
@enduml
```

```rockuml
@startuml
start
split
  :Email the crew;
split again
  :Text the crew;
split again
  :Send a Den Den Mushi;
end split
:Set sail;
stop
@enduml
```

## Arrows

A line starting with `->` between actions labels the arrow. `-[#red]->`, `-[#blue,dashed]->`, `-[#green,bold]->` and `-[#black,dotted]->` colour and style it.

```rockuml
@startuml
start
:Pick up the One Ring;
-> feels strangely heavy;
:Put it on;
-[#red,bold]-> becomes invisible;
:Sauron sees you;
-[#gray,dashed]-> regrets;
stop
@enduml
```

### Ending a flow

`kill` and `detach` end a branch on the spot, without an arrow to the next step:

```rockuml
@startuml
start
:Hold the door;
fork
  :Bran escapes;
fork again
  :Hodor holds the door;
  kill
end fork
:Winter comes;
stop
@enduml
```

### Connectors and goto

A connector, `(A)`, continues the flow elsewhere without a long arrow. `label` and `goto` jump back to a named point:

```rockuml
@startuml
start
label retry
:Ask Gilfoyle for help;
if (He insults you?) then (yes)
  :Fix it yourself;
  goto retry
endif
:Thank Gilfoyle;
stop
@enduml
```

## Swimlanes

`|Name|` starts a swimlane: the actions after it go into its column until the next one. A colour before the name, `|#pink|Name|`, colours the lane.

```rockuml
@startuml
|Customer|
start
:Order a pizza;
|#AntiqueWhite|Pizza Planet|
:Bake the pizza;
:Hand it to the delivery boy;
|Delivery|
:Deliver it, cryogenically frozen
for a thousand years;
|Customer|
:Eat cold pizza;
stop
@enduml
```

## Grouping

`partition Name { … }` frames part of the flow with a heading; `group`, `package`, `rectangle` and `card` frame it with other shapes.

```rockuml
@startuml
start
partition "Phase 1: Collect underpants" {
  :Sneak into bedrooms;
  :Collect underpants;
}
partition "Phase 2" #LightYellow {
  :?;
}
group "Phase 3" {
  :Profit;
}
stop
@enduml
```

## Notes

Notes go `left` or `right` of the last action; `floating note` puts one on the flow itself.

```rockuml
@startuml
start
:Assemble the IKEA shelf;
note right
  There are 3 screws left over.
  This is <b>fine</b>.
end note
:Put the books on it;
floating note left : it holds, for now
stop
@enduml
```

## Colours

A colour stereotype after an action colours it: `:Do something; <<#pink>>`. Gradients such as `<<#yellow/orange>>` work too, and [styles](docs/styling) change all actions at once.

```rockuml
@startuml
start
:Plant a seed; <<#LightGreen>>
:Wait for the sun; <<#Yellow>>
:Water it; <<#LightBlue/White>>
:Harvest the potato; <<#Peru>>
stop
@enduml
```

The older form with the colour in front, `#pink:Do something;`, still works, but PlantUML 1.2026.8 marks it as deprecated with a note in the diagram.

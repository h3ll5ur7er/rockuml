```rockuml tldr
@startuml
robust "Hogwarts Express" as Train
concise "Harry" as Harry
scale 100 as 60 pixels

@0
Train is Waiting
Harry is "Platform 9"

@200
Harry is "Wall run"

@400
Harry is "Platform 9¾"
Train is Boarding

@600
Train is Departing
Harry -> Train : jumps on
Harry is Aboard

@800
Train is Travelling
@enduml
```

A timing diagram shows how the states of several players change over time, along a shared time axis. It comes from electronics, where it shows signals and clocks, but it is just as good for showing which service is doing what while a request travels through them.

## Players

Each line of the diagram is a player, declared with its kind, its name and an optional alias:

| Player | Draws |
|---|---|
| `robust` | states as levels on a stepped line, one row per state |
| `concise` | states as labelled blocks in a single row |
| `rectangle` | states as labelled rectangles |
| `clock` | a regular clock signal |
| `binary` | a signal that is `high` or `low` |
| `analog` | a value, drawn as a line between two bounds |

```rockuml
@startuml
robust "Arc reactor" as Reactor
concise "Tony" as Tony

@0
Reactor is Stable
Tony is Working

@100
Reactor is Overloaded
Tony is Panicking

@200
Reactor is Stable
Tony is Bragging
@enduml
```

## Time

`@100` moves to time 100; the state changes after it happen at that time. `@+50` moves 50 after the previous time. `@Player` followed by lines `time is State` describes one player at a time, which is often easier to read:

```rockuml
@startuml
robust "Light saber" as Saber
concise "Duel" as Duel

@Saber
0 is Off
+50 is On
+300 is Off

@Duel
0 is Talking
100 is Fighting
350 is "Luke, I am your father"
@enduml
```

### Dates and anchors

Times can be dates, like `@2026/10/09`, and `use date format` chooses how the axis prints them. `@100 as :launch` names a time, so later lines can use `@:launch` or `@:launch+20`.

```rockuml
@startuml
use date format "dd MMM"
concise "Iron Throne" as Throne

@2019/04/14
Throne is Cersei

@2019/05/12
Throne is "Burned down"

@2019/05/19
Throne is Bran
@enduml
```

```rockuml
@startuml
clock "Clock" as clk with period 10
binary "Hyperdrive" as HD

@0 as :start
@40 as :jump
@:start
HD is low
@:jump
HD is high
@:jump+30
HD is low
@enduml
```

## Clocks, binary and analog signals

A `clock` needs a period, and can also have a `pulse` width and an `offset`. A `binary` player is `high` or `low`. An `analog` player takes numbers between its bounds; `ticks num on multiple` adds a scale and `is 150 pixels height` sets its height.

```rockuml
@startuml
clock "Flux clock" as clk with period 50 pulse 15 offset 10
binary "Flux capacitor" as FC
analog "Speed (mph)" between 0 and 100 as Speed
Speed ticks num on multiple 22
Speed is 120 pixels height

@0
FC is low
Speed is 0

@100
Speed is 60

@200
Speed is 88
FC is high

@250
Speed is 0
FC is low
@enduml
```

## States

`Player has A,B,C` (without spaces) declares a robust player's states in the order you want them, and `has "Long label" as L` gives one a short name. A colour after a state colours that stretch.

```rockuml
@startuml
robust "Pikachu" as P
P has Sleeping,Happy,Angry
concise "Ash" as Ash

@0
P is Sleeping
Ash is Training #LightGreen

@100
P is Happy

@200
P is Angry
Ash is "Getting shocked" #Yellow

@300
P is Happy
Ash is Training
@enduml
```

`{-}` leaves a stretch empty and `{hidden}` hides the line entirely. `{A,B}` shows two states at once, for a signal that is not yet decided.

## Messages and constraints

`A -> B : text` sends a message from one player to another at the current time, and `A -> B@+50` arrives later. `@100 <-> @200 : text`, or `Player@100 <-> @200`, measures a duration on the axis.

```rockuml
@startuml
robust "Earth" as Earth
robust "Mars" as Mars

@0
Earth is Sending
Mars is Waiting

@50
Earth -> Mars@250 : "hello"
Earth is Waiting

@250
Mars is Receiving
Earth@50 <-> @250 : {12 light minutes}
@enduml
```

## Highlights and notes

`highlight 100 to 200` shades a stretch of time, with an optional colour and caption. Notes go `top of` or `bottom of` a player.

```rockuml
@startuml
concise "Meeseeks" as M

@0
M is Existing
@100
M is Helping
@400
M is "Existence is pain"
@500
M is {-}

highlight 100 to 400 #Gold : the task
note bottom of M : poof
@enduml
```

## Layout

- `scale 100 as 50 pixels` draws 100 time units in 50 pixels.
- `mode compact` puts each player's name on the same row as its states; `compact` in front of one player does it for that player only.
- `hide time-axis` removes the axis at the bottom.
- `manual time-axis` labels only the times you name.

```rockuml
@startuml
mode compact
scale 100 as 80 pixels
concise "Morning" as M
robust "Coffee level" as C

@0
M is Asleep
C is Empty
@100
M is Grumpy
C is Brewing
@200
M is Human
C is Full
@enduml
```

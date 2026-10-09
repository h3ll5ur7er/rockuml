```rockuml tldr
@startuml
!$crew = ["Fry", "Leela", "Bender"]
!$ship = "Planet Express Ship"

!procedure $deliver($from, $to, $item = "a package")
$from -> $to : delivers $item
!endprocedure

!foreach $member in $crew
participant $member
!endfor

$deliver("Fry", "Leela")
$deliver("Leela", "Bender", "slurm")
!if %strlen($ship) > 10
note over Fry, Bender : $ship has a long name
!endif
@enduml
```

Before rockuml reads a diagram, its preprocessor runs over the source: it replaces variables, expands functions and procedures, evaluates conditions and loops, and pulls in other files. It is a small programming language for generating diagrams, and its lines start with `!`. It works in every diagram type.

## Variables

`!$name = value` sets a variable, and `$name` anywhere in the diagram is replaced by its value. Values are strings, numbers, or JSON (arrays and objects). `?=` sets a variable only if it isn't set yet, which is how a file offers defaults.

```rockuml
@startuml
!$hero = "Saitama"
!$punches = 1
!$total = $punches * 100 + 1
!$cape ?= "yellow"
!$cape ?= "ignored, already set"
$hero -> Monster : $punches punch
note right : cape: $cape, total: $total
@enduml
```

`!global $name` sets a variable visible inside functions too, and `!local $name` one that stays inside the function that sets it.

### JSON values

Variables can hold JSON, and `$data.key` or `$data.list[1]` reads into it:

```rockuml
@startuml
!$avenger = {"name": "Thor", "weapons": ["Mjolnir", "Stormbreaker"], "home": {"realm": "Asgard"}}
participant "$avenger.name of $avenger.home.realm" as T
T -> Thanos : swings $avenger.weapons[1]
Thanos --> T : you should have gone for the head
@enduml
```

## Conditions

`!if`, `!elseif`, `!else` and `!endif` include lines only when a condition holds. Conditions compare with `==`, `!=`, `<`, `>`, `<=` and `>=`, and combine with `&&` and `||`.

```rockuml
@startuml
!$power_level = 9001
Vegeta -> Nappa : what does the scouter say?
!if $power_level > 9000
Nappa --> Vegeta : it's over 9000!
!elseif $power_level > 1000
Nappa --> Vegeta : impressive
!else
Nappa --> Vegeta : weakling
!endif
@enduml
```

`!ifdef NAME` and `!ifndef NAME` test whether something is defined, for example with `-DNAME` on the [command line](docs/cli).

## Loops

`!while condition` … `!endwhile` repeats lines, and `!foreach $item in list` … `!endfor` runs over a list or the keys of an object:

```rockuml
@startuml
!$i = 1
!while $i <= 3
participant "Clone $i" as C$i
!$i = $i + 1
!endwhile
!foreach $jutsu in ["Rasengan", "Sexy Jutsu", "Shadow Clone"]
Naruto -> C1 : $jutsu
!endfor
@enduml
```

## Functions and procedures

A function computes a value with `!return`; a procedure produces diagram lines. Both take arguments, with default values if you like, and are called by name.

```rockuml
@startuml
!function $double($x)
!return $x * 2
!endfunction

!procedure $handshake($a, $b, $greeting = "hi")
$a -> $b : $greeting
$b --> $a : $greeting back, times $double(21)
!endprocedure

$handshake("Rick", "Morty")
$handshake("Morty", "Summer", "ugh")
@enduml
```

Functions can call themselves, and arguments can be passed by name:

```rockuml
@startuml
!function $fib($n)
!if $n < 2
!return $n
!endif
!return $fib($n - 1) + $fib($n - 2)
!endfunction
!function $say($who, $what = "hello", $end = "!")
!return $who + " says " + $what + $end
!endfunction
Fibonacci -> Rabbits : month 10: $fib(10) pairs
Rabbits --> Fibonacci : $say("Bunny", $end = "?")
@enduml
```

`!unquoted procedure` declares a procedure whose arguments need no quotes, for short macros: `!unquoted procedure LINK($a, $b)` is called as `LINK(Alice, Bob)`.

## Built-in functions

Built-in functions start with `%`:

| Function | Result |
|---|---|
| `%strlen("abc")` | `3` |
| `%upper("abc")`, `%lower("ABC")` | `ABC`, `abc` |
| `%substr("rockuml", 0, 4)` | `rock` |
| `%strpos("rockuml", "uml")` | `4` |
| `%splitstr("a,b,c", ",")` | `["a","b","c"]` |
| `%string(42)`, `%intval("42")` | `42` as a string, as a number |
| `%mod(17, 5)` | `2` |
| `%dec2hex(255)`, `%hex2dec("ff")` | `ff`, `255` |
| `%date("yyyy-MM-dd")` | today's date |
| `%darken("red", 20)`, `%lighten("red", 20)` | darker and lighter colours |
| `%is_dark("navy")`, `%is_light("white")` | `1` (true; `0` is false) |
| `%reverse_color("#336699")` | the opposite colour |
| `%hsl_color(120, 100, 50)` | a colour from hue, saturation and lightness |
| `%json_add`, `%json_set`, `%json_remove`, `%json_merge` | changed copies of JSON values |
| `%get_json_keys($obj)`, `%json_key_exists($obj, "k")` | the keys of an object; whether it has one |
| `%variable_exists("$x")`, `%function_exists("$f")` | whether they are defined |
| `%newline()`, `%breakline()` | a new line in a text |
| `%version()` | the PlantUML version rockuml is compatible with |

```rockuml
@startuml
!$name = "Monkey D. Luffy"
!$colour = "#3366CC"
participant "%upper(%substr($name, 10))" as L $colour
participant Zoro %lighten($colour, 40)
L -> Zoro : "%splitstr($name, " ")"
Zoro --> L : name has %strlen($name) letters, hex of 255 is %dec2hex(255)
@enduml
```

## Includes

`!include file.puml` inserts another file at that point, so diagrams can share definitions. `!include_once` ignores a file that is already included, `!include_many` allows it again, and `!includesub file!PART` inserts only a part marked with `!startsub PART` … `!endsub` in that file. `!include <library/file>` takes a file from the [standard library](docs/sprites#the-standard-library).

```plantuml
@startuml
!include common/actors.puml
!include_once common/styles.puml
!includesub common/flows.puml!LOGIN
!include <C4/C4_Container>
Customer -> Shop : browse
@enduml
```

Includes read files and URLs, so they work with the `rockuml` binary and the [server](docs/server), not in the browser. The command line's `-I file` includes a file before every diagram.

## Legacy defines

The older `!define` and `!definelong` macros still work, for sources written before variables and functions existed:

```rockuml
@startuml
!define HERO(name) participant name #Gold
!define FIGHT(a, b) a -> b : fights
!definelong TEAM_UP(a, b)
a -> b : team up
b --> a : agreed
!enddefinelong
HERO(Goku)
HERO(Vegeta)
FIGHT(Goku, Vegeta)
TEAM_UP(Goku, Vegeta)
@enduml
```

`!undef NAME` removes a definition again.

## Debugging

`!assert condition : message` stops with an error when the condition is false. The command line's `--preproc` flag (and the `preproc` format of the [JavaScript API](docs/javascript)) shows the source after preprocessing, which is the quickest way to see what a macro did.

```bash
rockuml --preproc diagram.puml
```

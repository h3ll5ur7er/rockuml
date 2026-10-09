```rockuml tldr
@startuml
!theme cyborg
title Theme: cyborg
actor Neo
participant "The Oracle" as Oracle
Neo -> Oracle : am I the one?
Oracle --> Neo : have a cookie
note right of Oracle : try another theme\nin the first line
@enduml
```

A theme restyles a whole diagram with one line: colours, fonts, borders and shadows, for every kind of element. `!theme name` goes near the top of the diagram, and everything after it follows the theme. rockuml carries all 44 themes of PlantUML 1.2026.8; they work in the browser too, so edit the template above and try them.

## Choosing a theme

The theme names are:

| Family | Themes |
|---|---|
| Light | `plain`, `mono`, `lightgray`, `silver`, `sandstone`, `materia`, `materia-outline`, `minty`, `united`, `spacelab`, `spacelab-white`, `cerulean`, `cerulean-outline`, `cloudscape-design`, `aws-orange`, `vibrant`, `toy`, `mimeograph`, `sketchy`, `sketchy-outline` |
| Dark | `cyborg`, `cyborg-outline`, `superhero`, `superhero-outline`, `black-knight`, `hacker`, `crt-green`, `crt-amber`, `metal`, `mars`, `amiga`, `sunlust`, `carbon-gray` |
| Blue and grey | `bluegray`, `blueprint` |
| reddress | `reddress-darkblue`, `reddress-darkgreen`, `reddress-darkorange`, `reddress-darkred`, `reddress-lightblue`, `reddress-lightgreen`, `reddress-lightorange`, `reddress-lightred` |

`_none_` is the theme that changes nothing.

## A few of them

```rockuml
@startuml
!theme blueprint
title Blueprint of the Death Star
class "Death Star" as DS {
  diameter = 120 km
  superlaser()
}
class "Exhaust port" as Port {
  width = 2 m
}
DS *-- Port : unfortunately
@enduml
```

```rockuml
@startuml
!theme hacker
title rm -rf / --no-preserve-root
start
:sudo make me a sandwich;
if (Root?) then (yes)
  :Okay.;
else (no)
  :Make it yourself.;
endif
stop
@enduml
```

```rockuml
@startmindmap
!theme mars
* Mars
** Matt Damon
*** Grows potatoes
*** Listens to disco
** Rovers
*** Curiosity
*** Perseverance
@endmindmap
```

```rockuml
@startuml
!theme sketchy-outline
actor Customer
participant Bakery
Customer -> Bakery : one croissant, please
Bakery --> Customer : sold out
Customer -> Customer : existential crisis
@enduml
```

## Themes and your own styles

A theme is a style sheet with `skinparam` settings, so anything after `!theme` overrides it: add a `<style>` block or a `skinparam` for the parts you want different. A later `!theme` line replaces the earlier one.

```rockuml
@startuml
!theme superhero
skinparam ArrowColor Gold
<style>
participant {
  BackgroundColor DarkRed
}
</style>
participant Superman
participant "Lex Luthor" as Lex
Superman -> Lex : not today
Lex --> Superman : kryptonite?
@enduml
```

`%get_current_theme()` gives the name of the theme in use, and `%get_all_theme()` the list of all of them, for use with the [preprocessor](docs/preprocessor).

## Themes from files

`!theme name from path` loads a theme from a folder, `puml-theme-name.puml` in it. This reads files, so it works with the `rockuml` binary and not in the browser:

```plantuml
@startuml
!theme company from ./themes
Alice -> Bob : in our corporate colours
@enduml
```

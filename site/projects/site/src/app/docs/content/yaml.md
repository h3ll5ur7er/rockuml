```rockuml tldr
@startyaml
#highlight "crew" / "captain"
ship: Thousand Sunny
crew:
  captain: Monkey D. Luffy
  swordsman: Roronoa Zoro
  navigator: Nami
goals:
  - find the One Piece
  - become King of the Pirates
@endyaml
```

`@startyaml` draws a YAML document the way [JSON](docs/json) diagrams are drawn: maps as tables of keys and values, lists as lists, nested values to the right. It is handy for showing configuration files, Kubernetes manifests and CI pipelines without making anyone read indentation.

## Documents

rockuml reads the YAML people actually write: nested maps, lists of values and of maps, inline lists in brackets, comments, and `|` blocks of several lines.

```rockuml
@startyaml
# The Rebel Alliance deployment
service: x-wing-squadron
replicas: 12
pilots:
  - name: Luke Skywalker
    callsign: Red Five
  - name: Wedge Antilles
    callsign: Red Two
target: |
  thermal exhaust port
  two metres wide
weapons: [proton torpedoes, lasers]
@endyaml
```

A list at the top is drawn as a list:

```rockuml
@startyaml
- name: Sheldon Cooper
  field: theoretical physics
  spot: on the couch
- name: Leonard Hofstadter
  field: experimental physics
  spot: anywhere else
@endyaml
```

## Highlights and styles

`#highlight` marks values by their path, and stereotypes give highlights their own style. A `yamlDiagram` style sheet styles the whole document.

```rockuml
@startyaml
<style>
yamlDiagram {
  node {
    BackGroundColor #EEF6FF
    LineColor SteelBlue
  }
  highlight {
    BackGroundColor Gold
  }
}
</style>
#highlight "village" / "hokage"
village:
  name: Konohagakure
  hokage: Naruto Uzumaki
  advisor: Shikamaru Nara
ninja: 3000
@endyaml
```

```rockuml tldr
@startjson
#highlight "crew" / "0" / "name"
{
  "ship": "Planet Express Ship",
  "captain": "Leela",
  "crew": [
    { "name": "Fry", "role": "delivery boy" },
    { "name": "Bender", "role": "bending unit" }
  ],
  "insured": false,
  "cargo": null
}
@endjson
```

Between `@startjson` and `@endjson`, rockuml draws any JSON document as nested tables: objects become tables of keys and values, arrays become lists, and nested values hang off to the right with arrows. It turns an API response or a config file into something you can put on a slide.

## Documents

Any valid JSON works: objects, arrays, strings, numbers, booleans and `null`, nested as deep as you like. The root does not have to be an object.

```rockuml
@startjson
{
  "name": "Deep Thought",
  "answer": 42,
  "runtime": "7.5 million years",
  "questionKnown": false,
  "successor": {
    "name": "Earth",
    "status": "demolished",
    "reason": ["hyperspace bypass", "Vogons"]
  }
}
@endjson
```

```rockuml
@startjson
["Kurama", 9, true, null, {"tails": [1, 2, 3, 4, 5, 6, 7, 8, 9]}]
@endjson
```

Texts in JSON are drawn as they are: creole markup such as `**bold**` stays plain text, as it should for data.

## Highlights

`#highlight` before the document highlights a value by its path, keys and array positions separated by slashes. `"**"` matches any path, so `#highlight "**" / "name"` highlights every `name`.

```rockuml
@startjson
#highlight "avengers" / "1" / "name"
#highlight "**" / "status"
{
  "avengers": [
    { "name": "Iron Man", "status": "snapped back" },
    { "name": "Captain America", "status": "retired" },
    { "name": "Thor", "status": "with the Guardians" }
  ]
}
@endjson
```

A stereotype after a highlight picks a style for it, defined in a style sheet:

```rockuml
@startjson
<style>
  .danger {
    BackGroundColor Crimson
    FontColor White
    FontStyle bold
  }
  .safe {
    BackGroundColor PaleGreen
  }
</style>
#highlight "reactor" / "core temperature" <<danger>>
#highlight "reactor" / "safety inspector" <<safe>>
{
  "reactor": {
    "core temperature": "critical",
    "safety inspector": "Homer Simpson",
    "donuts in stock": 0
  }
}
@endjson
```

## Styling

A `jsonDiagram` style sheet styles the tables (`node`), the arrows and the highlights:

```rockuml
@startjson
<style>
jsonDiagram {
  node {
    BackGroundColor #FFF7ED
    LineColor #C2410C
    FontName Monospaced
    RoundCorner 6
  }
  arrow {
    LineColor #C2410C
    LineThickness 2
  }
  highlight {
    BackGroundColor #FB923C
    FontColor White
  }
}
</style>
#highlight "inventory" / "pickaxe"
{
  "player": "Steve",
  "inventory": { "pickaxe": "diamond", "torches": 64, "bread": 12 }
}
@endjson
```

A title and `scale` can come before the document, and `MaximumWidth` in the node style wraps long values.

## In other diagrams

The `json` keyword puts a JSON object into [class, object](docs/object#json-objects) and other UML diagrams, where it can be linked to the other elements.

```rockuml tldr
@startregex
title Email
[\w.+-]+@[\w-]+(\.[\w-]+)+
@endregex
```

`@startregex` draws a regular expression as a railroad diagram, with the same tiles as [EBNF diagrams](docs/ebnf). Each line holds one expression; when there are several, only the last is drawn.

## What is drawn

- Letters in a row, and escaped punctuation like `\.` or `\(`, join into one plain box.
- Classes (`.`, `\d`, `\w`, `\s` and their negations, `\p{Lu}`, `\x{1F600}`, `é`, octal `\0123`, POSIX `[[:alpha:]]`) and anchors (`^`, `$`, `\b`, `\B`, `\A`, `\z`, `\Z`, `\G`) are drawn in rounded boxes.
- A set `[…]` lists its characters and ranges in a dashed box; `[^…]` shows the `^` apart from what it excludes.
- `|` stacks alternatives, and groups `( … )` and `(?: … )` only group.
- `?` makes a part optional, `*` and `+` repeat it, and `{n}`, `{n,}` and `{n,m}` write the count over the loop. A lazy `?` after `?`, `*` or `+` changes nothing; after a count PlantUML reads it as a bad quantifier.
- A named group `(?<name> … )` is drawn on a tinted background under its name, and lookarounds `(?= … )`, `(?! … )`, `(?<= … )` and `(?<! … )` in a dashed frame. Comments `(?# … )` are left out.

```rockuml
@startregex
title Dates
(?<year>\d{4})-(?<month>0[1-9]|1[0-2])-\d{2}(?!\d)
@endregex
```

```rockuml
@startregex
^(cat|dog|bird(s|es)?)\b[^,;]*$
@endregex
```

## Styles

The diagram uses the `ebnf` element style, as EBNF diagrams do; `<style>` blocks apply wherever they are written.

```rockuml
@startregex
<style>
element {
  ebnf {
    LineColor #6A1B9A
    BackGroundColor #F3E5F5
  }
}
</style>
0x[0-9A-Fa-f]{1,8}
@endregex
```

## Errors

An unclosed parenthesis or comment, or a count like `{x}`, makes the diagram an error ("Error parsing: …"). So does an expression PlantUML's own reader fails on, such as one starting with a quantifier, with the message PlantUML gives.

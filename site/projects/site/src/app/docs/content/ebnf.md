```rockuml tldr
@startebnf
title Numbers
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
number = [ "-" ], digit, { digit } ;
@endebnf
```

`@startebnf` draws a grammar written in Extended Backus–Naur Form as railroad diagrams: one track per rule, from a start circle to an end circle, under the rule's name in bold.

## Rules

A rule is a name, `=`, and a definition ending with `;`. It may run over several lines, up to the line that ends with `;`.

- Names of other rules are drawn in rounded boxes, quoted strings (`"…"` or `'…'`) in plain boxes, and special sequences `? … ?` in dashed boxes. An empty string is drawn as a space.
- `,` puts parts one after the other, `|` stacks alternatives, and parentheses group them.
- `[ … ]` is optional: the track may go around it below.
- `{ … }` repeats any number of times and `{ … }-` at least once: the track loops back above it.
- `3 * x` repeats `x` three times, the count written over the loop; `0 * x` may also skip it.
- `a - b` draws `b` after the word `NOT`. A rule with nothing after `=` is an empty track.

```rockuml
@startebnf
statement =
  assignment
  | call
  | "return", [ expression ] ;
assignment = identifier, "=", expression ;
call = identifier, "(", [ expression, { ",", expression } ], ")" ;
identifier = letter, { letter | digit | "_" }- ;
letter = ? any letter ? - digit ;
triple = 3 * "a" ;
@endebnf
```

## Comments

`(* … *)` inside a rule becomes a note pointing at the box it follows, or the one it precedes; around a rule's name it goes above or below the name. A comment alone on its line, or `(* … *)` and `(** … **)` over several lines, is a note between the rules.

```rockuml
@startebnf
(* The greeting rules *)
(* above *) greeting (* below *) = "hello", (* who *) name ;
name = first (* given name *), [ last (* family name *) ] ;
@endebnf
```

## Styles

Rules are drawn in the `ebnf` element style, and notes in its `note` style. `title`, `caption`, `header`, `footer` and `legend` work as in other diagrams.

```rockuml
@startebnf
<style>
element {
  ebnf {
    LineColor #2E5C8A
    BackGroundColor #E8F0FA
    FontColor #102030
    note {
      BackGroundColor #FFF4D0
    }
  }
}
</style>
caption Styled rules
(* the entry point *)
program = "begin", { statement (* any number *) }, "end" ;
@endebnf
```

## Errors

A rule PlantUML cannot draw, such as two names without `,` or `|` between them or an unclosed bracket, is drawn as `Syntax error!`. A character EBNF does not have, like `#`, makes the whole diagram an error: "Unparsable expression". So does a string left unclosed, with the message PlantUML gives.

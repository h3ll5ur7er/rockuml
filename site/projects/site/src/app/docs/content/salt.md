```rockuml tldr
@startsalt
{+
  <b>Log in to the Matrix
  Username | "Neo        "
  Password | "****       "
  [X] Remember me
  () Blue pill
  (X) Red pill
  [ Cancel ] | [  Log in  ]
}
@endsalt
```

Salt draws wireframes, sketches of user interfaces, from text. Buttons, text fields, checkboxes, tables, trees, tabs and menus are each a few characters. It is perfect for a quick mock-up in a ticket or a design discussion, before anybody opens a design tool.

## Widgets

A wireframe is a block in braces. Inside, each line is a row, and the characters say what each element is:

| You write | You get |
|---|---|
| `Plain text` | a label |
| `[Button]` | a button |
| `"Some text   "` | a text field, as wide as the quotes |
| `[ ]`, `[X]` | a checkbox, unchecked or checked |
| `( )`, `(X)` | a radio button, unchecked or checked |
| `^Option^` | a drop-down list |

```rockuml
@startsalt
{
  Order a Krusty Burger
  "Name of the customer     "
  ^Burger size^
  [X] Extra cheese
  [ ] Mystery sauce
  ( ) Eat here
  (X) Take away
  [Order]
}
@endsalt
```

## Grids

`|` separates the columns of a row. A brace with a sign draws grid lines: `{#` all of them, `{!` the vertical ones, `{-` the horizontal ones, `{+` only a frame. A `*` makes a cell span the cell to its left, and `.` leaves a cell empty.

```rockuml
@startsalt
{#
  . | Strength | Speed | Brains
  Hulk | 10 | 6 | *
  Black Widow | 6 | 8 | 9
  Ant-Man | 2 | 4 | 7
}
@endsalt
```

`{^"Title"` draws a group box with a title:

```rockuml
@startsalt
{^"Hokage application form"
  Name | "Naruto Uzumaki "
  Village | ^Leaf^
  Ninja rank | "genin          "
  [Withdraw] | [Submit]
}
@endsalt
```

### Separators

A line of `..`, `==`, `~~` or `--` draws a dotted, double, wavy or plain separator.

```rockuml
@startsalt
{
  Settings
  ==
  [X] Notifications
  ..
  [ ] Dark mode
  ~~
  [ ] Experimental features
  --
  [Save]
}
@endsalt
```

## Trees

`{T` draws a tree, with one `+` per level:

```rockuml
@startsalt
{
{T
 + Middle-earth
 ++ The Shire
 +++ Hobbiton
 +++ Bywater
 ++ Gondor
 +++ Minas Tirith
 ++ Mordor
 +++ Mount Doom
}
}
@endsalt
```

`{T!` adds columns with grid lines, for a tree table:

```rockuml
@startsalt
{
{T!
+ Crew member | Bounty
+ Straw Hats | 8 816 001 000
++ Luffy | 3 000 000 000
++ Zoro | 1 111 000 000
++ Chopper | 1 000
}
}
@endsalt
```

## Tabs and menus

`{/` draws tabs, `{*` a menu bar. A menu bar's second line lists the entries of an open menu, with `-` for a separator.

```rockuml
@startsalt
{+
{* File | Edit | Tools | Help
 File | New | Open | - | Save | Exit }
{/ <b>General | Network | Advanced }
{
  { Theme: | ^Dark^ }
  [X] Start with the system
  [ ] Show tips
}
[Close]
}
@endsalt
```

## Scroll bars

`{S` adds scroll bars to a block, `{SI` only a vertical one and `{S-` only a horizontal one:

```rockuml
@startsalt
{
{S
  Terms and conditions
  .
  You agree to everything.
  .
  Really everything.
}
[X] I have read the terms
}
@endsalt
```

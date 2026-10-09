```rockuml tldr
@startuml
participant "**Bold** and //italic//" as A
participant "<color:red>Red</color> and <size:18>big</size>" as B
A -> B : __underlined__, --struck--, ~~wavy~~, ""mono""
note right of B
  = A heading
  * a list
  ** nested
  |= Table |= Header |
  | cell | <#LightGreen> green |
end note
@enduml
```

Creole is the light markup language that every text in a diagram understands: participant names, messages, notes, actions, titles, legends. It mixes wiki-style marks (`**bold**`) with HTML-like tags (`<b>bold</b>`), and goes as far as lists, tables, icons and headings. `@startcreole` draws a creole text on its own, which is how most examples on this page work.

## Emphasis

| You write | You get |
|---|---|
| `**bold**` | bold |
| `//italic//` | italic |
| `""monospaced""` | monospaced |
| `__underlined__` | underlined |
| `--struck--` | struck through |
| `~~wavy~~` | wavy underline |

```rockuml
@startcreole
This is **bold**, //italic//, ""monospaced"",
__underlined__, --struck through-- and ~~wavy~~.
**Bold with //italic// inside.**
@endcreole
```

The same effects have HTML-like tags, which also take colours: `<b>`, `<i>`, `<u>`, `<s>` or `<strike>`, `<w>` for wavy, and `<u:red>`, `<s:green>`, `<w:#0000FF>` with a colour. `<sup>` and `<sub>` raise and lower text, `<back:yellow>` highlights it, and `<plain>` turns the formatting off again.

```rockuml
@startcreole
E = mc<sup>2</sup>, and water is H<sub>2</sub>O.
<u:red>Red underline</u>, <w:#0000FF>blue wave</w>, <s:green>green strike</s>.
<back:yellow>Highlighted</back>, <back:#FFAAAA|#AAAAFF>with a gradient</back>.
**Bold, <plain>except here</plain>, and bold again.**
@endcreole
```

## Colours, sizes and fonts

`<color:red>`, `<size:18>` and `<font:monospaced>` change the colour, size and font of the text up to their closing tag, or to the end of the line. `<font color=blue size=18>` sets several at once.

```rockuml
@startcreole
<color:red>Red</color>, <color:#00AA00>green</color>, <color:RoyalBlue>royal blue</color>.
<size:24>Big</size>, <size:10>small</size>.
<font:monospaced>Monospaced</font>, <font:serif>serif</font>.
<font color=purple size=18>Purple and 18 points</font>
@endcreole
```

Colours are names (`Red`, `LightGoldenRodYellow`, all the HTML colour names) or hexadecimal codes (`#FF8800`, `#F80`).

## Lists

Lines starting with `*` make a bulleted list, with `#` a numbered one. More marks make a deeper level.

```rockuml
@startuml
:Rules of the Fellowship
* Never use the Ring
* Never split up
** unless Boromir
*** especially Boromir
# Walk to Mordor
# Throw it in;
@enduml
```

## Headings

Lines starting with `=`, `==`, `===` and so on are headings, from large to small:

```rockuml
@startcreole
= The Guide
== Earth
=== Mostly harmless
Plain text after the headings.
@endcreole
```

## Separators

A line of `----`, `====` or `....` draws a separator, and `== Title ==` or `.. Title ..` a separator with a title. They work wherever several lines do, such as notes and legends.

```rockuml
@startuml
note as Spec
  <b>Stark Industries spec sheet</b>
  ----
  Arc reactor: miniaturised
  ====
  Suit: Mark 85
  .. Classified ..
  Weakness: none
  == Status ==
  Retired
end note
@enduml
```

## Tables

Rows start and end with `|`, and `|=` marks a header cell. `<#color>` at the start of a cell colours it, and at the start of the row colours the whole row (with a second colour for the borders).

```rockuml
@startcreole
|= Hero |= Power |= Weakness |
| Superman | flight | kryptonite |
| <#LightBlue> Aquaman | talks to fish | dry land |
<#Pink>| Batman | money | none |
@endcreole
```

## Trees

`|_` lines draw a tree, indented by levels:

```rockuml
@startcreole
|_ Hogwarts
  |_ Gryffindor
    |_ Harry
    |_ Hermione
  |_ Slytherin
    |_ Draco
|_ Durmstrang
@endcreole
```

## Code

`<code>` … `</code>` keeps the lines between it as they are, in a monospaced font, without interpreting markup:

```rockuml
@startcreole
The ultimate program:
<code>
fn main() {
    println!("<b>42</b>");
}
</code>
@endcreole
```

## Links

`[[url]]` is a link, `[[url label]]` a link with its own text, and `[[url{tooltip} label]]` adds a tooltip. Links are clickable in SVG output.

```rockuml
@startcreole
Read [[https://plantuml.com]] or [[https://github.com/h3ll5ur7er/rockuml rockuml on GitHub]].
With a tooltip: [[https://example.com{Not a real site} example]].
@endcreole
```

## Icons

`<&name>` draws an icon from the [Open Iconic](https://github.com/iconic/open-iconic) set; `<&heart*2>` doubles its size. The [sprites page](docs/sprites#open-iconic) lists more.

```rockuml
@startcreole
<&heart> Love, <&star> stars, <&cloud> clouds, <&lock-locked> secrets.
<color:red><&heart*2></color> Big and red.
@endcreole
```

## Special characters

A tilde, `~`, writes the next character as it is: `~**` is two asterisks, not bold. `<U+221E>` writes a character by its Unicode code point, and `&#8734;` by its HTML code.

```rockuml
@startcreole
This is ~**not bold~** and ~//not italic~//.
Infinity: <U+221E> or &#8734;, arrow: <U+2192>.
@endcreole
```

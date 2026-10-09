```rockuml tldr
@startchen
left to right direction
entity WIZARD {
  Name <<key>>
  Wand
}
entity HOUSE {
  Name <<key>>
  Founder
}
relationship BELONGS_TO {
  Since
}
WIZARD -N- BELONGS_TO
BELONGS_TO -1- HOUSE
@endchen
```

An entity relationship diagram in Peter Chen's notation models data: *entities* are rectangles, *relationships* between them are diamonds, and their *attributes* are ovals. It is the classic way to design a database before a single table exists. Chen diagrams have their own start line, `@startchen`.

## Entities and attributes

An entity is `entity NAME`, with its attributes between braces. Stereotypes mark special attributes:

| Stereotype | Meaning | Drawn as |
|---|---|---|
| `<<key>>` | identifies the entity | underlined |
| `<<multi>>` | can have several values | a double oval |
| `<<derived>>` | computed from other attributes | a dashed oval |

An attribute with braces is a composite attribute, made of the attributes inside.

```rockuml
@startchen
entity JEDI {
  Id <<key>>
  Name {
    First
    Last
  }
  Lightsaber <<multi>>
  MidichlorianCount
  Rank <<derived>>
}
@endchen
```

## Relationships

A relationship is `relationship NAME`, with optional attributes of its own. Lines with the cardinality between dashes connect it to entities: `-1-`, `-N-`, `-M-`, or any other text.

```rockuml
@startchen
entity PIRATE {
  Name <<key>>
}
entity SHIP {
  Name <<key>>
}
entity CREW {
  Flag <<key>>
}
relationship SAILS_ON {
  Role
}
relationship MEMBER_OF {
}
PIRATE -N- SAILS_ON
SAILS_ON -1- SHIP
PIRATE -M- MEMBER_OF
MEMBER_OF -1- CREW
@endchen
```

## Weak entities

A weak entity, `<<weak>>`, depends on another entity for its identity, through an identifying relationship, `<<identifying>>`. A double line, `=N=`, shows that every weak entity must take part.

```rockuml
@startchen
entity STARSHIP {
  Registry <<key>>
}
entity DECK <<weak>> {
  Number <<key>>
  Purpose
}
relationship HAS <<identifying>> {
}
STARSHIP -1- HAS
HAS =N= DECK
@endchen
```

## Layout

`left to right direction` lays the diagram out sideways, which often reads better for chains of entities:

```rockuml
@startchen
left to right direction
entity HERO {
  Alias <<key>>
}
entity TEAM {
  Name <<key>>
}
entity HEADQUARTERS {
  Address <<key>>
}
relationship JOINS {
  Year
}
relationship BASED_IN {
}
HERO -N- JOINS
JOINS -M- TEAM
TEAM -1- BASED_IN
BASED_IN -1- HEADQUARTERS
@endchen
```

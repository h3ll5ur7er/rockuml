```rockuml tldr
@startpacketdiag
packetdiag {
  colwidth = 32
  node_height = 72

  0-15: Source Port
  16-31: Destination Port
  32-63: Sequence Number
  64-95: Acknowledgment Number
  96-99: Data Offset
  100-105: Reserved
  106-111: Flags
  112-127: Window
  128-143: Checksum
  144-159: Urgent Pointer
  160-191: (Options and Padding)
  192-223: data [height = 3]
}
@endpacketdiag
```

A packet diagram shows the layout of a protocol header or any other binary record: its fields side by side, bit by bit, in rows of a fixed width under a numbered bit scale. The syntax comes from packetdiag, blockdiag's tool for the job, and goes between `@startpacketdiag` and `@endpacketdiag`, usually inside `packetdiag { … }`.

## Fields

Each line names a field by its bits, then gives its label:

- `0-15: Source Port` covers bits 0 to 15,
- `16: Flag` covers bit 16 alone, or as many bits as `[len = …]` says,
- `* Next` starts right after the field before it, also one bit long unless `len` says otherwise.

The colon after the bits is optional. A field that does not fit in what is left of its row continues on the next rows. Labels are cut short with `...` when they are wider than their field.

```rockuml
@startpacketdiag
packetdiag {
  * Version [len = 4]
  * IHL [len = 4]
  * Type of Service [len = 8]
  * Total Length [len = 16]
  * Identification [len = 16]
  * Flags [len = 3]
  * Fragment Offset [len = 13]
  * Time to Live [len = 8]
  * Protocol [len = 8]
  * Header Checksum [len = 16]
  * Source Address [len = 32]
  * Destination Address [len = 32]
}
@endpacketdiag
```

## Rows

These settings shape the rows and the scale above them:

| Setting | Meaning | Default |
|---|---|---|
| `colwidth = 32` | bits in a row | 16 |
| `node_height = 72` | height of a field, in pixels | 32 |
| `scale_interval = 4` | how often the scale is numbered, in bits | half a row |
| `scale_direction = rtl` | numbers the bits from the right (`ltr` from the left) | `ltr` |
| `same_height = true` | every field of a row takes the height of its tallest | `false` |

A field's `[height = 2]` attribute makes it two `node_height`s high.

```rockuml
@startpacketdiag
packetdiag {
  colwidth = 12
  scale_direction = rtl
  scale_interval = 3
  same_height = true
  0-3: low
  4-7: middle [height = 2]
  8-15: high
  16-23: next
}
@endpacketdiag
```

PlantUML reads a `rotate` attribute as well, but draws every label upright, and so does rockuml. Numbers in attributes may be written in any script's digits (`[len = ٣]`); one that is no number leaves the default.

A field that ends before it starts, such as `5-3`, and a first row without bits, such as one made of `5-4` alone, make PlantUML draw a crash report; rockuml reports them as not ported.

## Styles and decorations

The `packetdiagDiagram` style sets the font of the scale and the labels, and how wide a bit is: three times the font size. Its `rectangle` style draws the fields. Titles, legends, captions, headers and footers work as in any diagram (see [Common commands](docs/basics)).

```rockuml
@startpacketdiag
<style>
packetdiagDiagram {
  FontSize 11
  LineColor #2266aa
  rectangle {
    BackGroundColor #ddeeff
    RoundCorner 8
    Shadowing 2
  }
}
</style>
title A short record
packetdiag {
  0-7: kind
  8-15: flags
  16-31: length
}
@endpacketdiag
```

```rockuml tldr
@starthcl
resource "aws_instance" "millennium_falcon" {
  ami           = "ami-kessel12"
  instance_type = "yt-1300"
  tags = {
    Captain = "Han Solo"
    CoPilot = "Chewbacca"
  }
}
@endhcl
```

`@starthcl` draws a Terraform or other HCL file the way [JSON](docs/json) and [YAML](docs/yaml) diagrams are drawn: blocks as tables of attributes and values, maps and lists nested to the right.

## Blocks

Each block is named by its type and labels. A file with a single block shows just its attributes; with several, each block gets a row. A later block of the same name replaces an earlier one.

```rockuml
@starthcl
# Comment lines starting with # are skipped
provider "aws" {
  region = "us-east-1"
}

variable "ring_bearer" {
  type    = string
  default = "Frodo Baggins"
}

output "destination" {
  value = mordor.mount_doom
}
@endhcl
```

## Values

Values are drawn as written: strings, numbers, booleans and references alike. Maps nest inside a block with `name = { ... }`, lists hold quoted strings and maps, and a function call is drawn with its arguments.

```rockuml
@starthcl
locals {
  fellowship = ["Aragorn", "Legolas", "Gimli"]
  members    = 9
  quest = {
    item = "the One Ring"
    by   = "the Council of Elrond"
  }
  started = timestamp()
  route   = lookup(var.maps, "Moria")
  wizards = [
    {
      name   = "Gandalf"
      colour = "grey"
    },
    {
      name   = "Saruman"
      colour = "white"
    }
  ]
}
@endhcl
```

## What PlantUML reads

rockuml reads HCL exactly as PlantUML 1.2026.8 does, which is a small part of the language:

- Nested blocks need an equals sign: `tags = { ... }`, not `tags { ... }`. Their attributes go on lines of their own, without commas.
- Lines are joined without a separator, so attributes must be indented or end with a quoted value: `count = 2` on a line of its own at the start of the line runs into the next one.
- Lists keep only quoted strings and maps; `[80, 443]` is drawn empty.
- `//` and `/* */` comments, heredocs and expressions are not read. HCL that PlantUML cannot read gives "Your data does not sound like HCL data", and so does a `<style>` left open, which takes in the end line.
- PlantUML ignores a `title` or a `<style>` block in HCL data; `scale` still applies.

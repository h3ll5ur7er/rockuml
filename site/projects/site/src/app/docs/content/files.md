```rockuml tldr
@startfiles
/Cargo.toml
/src/main.rs
/src/lib.rs
/tests/parity.rs
<note>
compares every output
with PlantUML's
</note>
@endfiles
```

Between `@startfiles` and `@endfiles`, rockuml draws a file tree: write one path per line, starting with `/`, and the folders on the way are drawn for you, each level indented below its parent, with a folder or a page in front of every name. It is the quickest way to show the layout of a project in a README or a design document.

## Paths

Every line that starts with `/` adds a path. Paths that share a beginning share their folders, in the order they first appear; a path that ends with `/` adds a folder and nothing in it.

```rockuml
@startfiles
/src/main/java/App.java
/src/main/java/util/Strings.java
/src/main/resources/app.properties
/src/test/java/AppTest.java
/docs/
/pom.xml
@endfiles
```

Only lines that start with `/` count: indented lines, comments and anything else are ignored, and so are `title`, `legend` and the other decorations of other diagrams. Names are drawn as they are written, without text formatting; `\n` breaks a name onto two lines.

```rockuml
@startfiles
/home/marvin/brain the size of a planet.txt
/home/marvin/two\nlines.md
/home/arthur/towel
@endfiles
```

## Notes

Lines between `<note>` and `</note>` make a note, written with [text formatting](docs/creole). A note goes below the last file added, at its level; after a folder, or before any file, it goes to the top level.

```rockuml
@startfiles
/project/Cargo.toml
<note>
The **manifest**:
* name
* version
</note>
/project/src/main.rs
<note>
//binary// entry point
</note>
/project/target/
<note>
build output, not committed
</note>
@endfiles
```

## Styles

A `filesDiagram` style sets the font of the names, and its `note` the look of the notes.

```rockuml
@startfiles
<style>
filesDiagram {
  FontColor #1F4E79
  FontSize 16
  note {
    BackGroundColor #E0F0FF
    LineColor #1F4E79
    Shadowing 4
  }
}
document {
  BackGroundColor #FAFAF0
}
</style>
/home/alice/notes.txt
/home/bob/todo.txt
<note>
bob's list
</note>
/var/log/syslog
@endfiles
```

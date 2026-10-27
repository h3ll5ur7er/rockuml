```rockuml tldr
@startbpm
:Request;
new branch
:Approve;
else
:Reject;
end branch
:Notify;
@endbpm
```

Between `@startbpm` and `@endbpm`, rockuml draws a BPM diagram: events in a row from a start circle, with merges, branches, gotos and resumes laid out on a grid. PlantUML has this diagram type but does not document it, and draws it as the unfinished sketch it is: the grid's lines are drawn, the elements show red stubs on the sides they connect by, and the connectors between them are short blue strokes. rockuml draws it exactly as PlantUML does.

## Events and merges

`:label;` adds an event, a rounded box, to the right of the previous element. `name<+>` or `name:<+>` adds a merge, a diamond that `goto` and `resume` can name. A name is made of letters, digits, `_`, `.` and `@`. Labels are written with [text formatting](docs/creole), and `\n` breaks them.

```rockuml
@startbpm
:Read **the** order\nand its lines;
packed:<+>
:<color:blue>Ship</color>;
@endbpm
```

## Resume and goto

`resume name` goes on from the merge `name`, on a new line below it, so the next event starts a second way out of the merge. `goto name` connects the last element to the merge `name`, then goes on from there, on a new line below it as well.

```rockuml
@startbpm
:Start;
work<+>
:Work;
resume work
:Fix;
goto work
:Done;
@endbpm
```

## Branches

`new branch` opens a branch with a merge at its entry. Each `else` ends a way and starts the next one below the entry: the first `else` closes the first way with the branch's exit merge, the others go to that exit. `end branch` sends the last way to the exit, and the events after it follow the exit. Branches nest, and a way can be empty.

```rockuml
@startbpm
:A;
new branch
:B;
new branch
:B1;
else
:B2;
end branch
else
:C;
end branch
:E;
@endbpm
```

## Limits

The layout is PlantUML's, and so are its limits:

- A connector that has to cross an element, for example a `goto` back along the row it is on, makes PlantUML fail with a crash report; rockuml reports such a diagram as not supported. So does a `goto` or `resume` naming a merge that does not exist.
- An `else` or `end branch` without an open branch is a syntax error.
- The diagram has no other commands: `title`, `skinparam`, `<style>` and the like are syntax errors, and the elements keep their look.

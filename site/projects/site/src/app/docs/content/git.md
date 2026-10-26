```rockuml tldr
@startgit
* 4c5d6e7 (HEAD -> main) Fix a typo
| * 9f8e7d6 (feature) Add the feature
|/
* 1a2b3c4 Initial commit
@endgit
```

Between `@startgit` and `@endgit`, rockuml draws a commit graph from the output of `git log --graph --oneline --decorate`: paste it in, and every commit becomes a box, with dashed arrows from each commit to its parents, newest at the top. The graph is laid out like a Graphviz `dot` graph, so it stays readable where the text art gets tangled.

## Commits and parents

Every line with a `*` is a commit; its name is the first word after the drawing characters. The `|`, `/` and `_` around it lead to its parents, as Git draws them. A commit with two lines below it is a merge, and a commit that nothing leads to below is a root.

```rockuml
@startgit
* d4e5f6a (HEAD -> main) Work on main
| * c3d4e5f (feature) Merge branch 'main' into feature
|/|
* | b2c3d4e Fix main
| * a1b2c3d Start the feature
|/
* 9a8b7c6 Initial commit
@endgit
```

A line that ends with `\` is joined to the next one, as everywhere in a source, so a merge drawn as `|\` at the end of a line loses its branch. Write the merge from the other side with `|/|`, as above, or keep something after the backslash.

## Branches, tags and messages

The text in the first parentheses of a line, usually Git's branch and tag names, is written above the box of a commit that has nothing newer. The rest of the line, the message, is not drawn.

```rockuml
@startgit
* 7c6b5a4 (HEAD -> main, tag: v2.0, origin/main) Ship it
| * 6b5a493 (origin/topic) Try an idea
|/
* 5a49382 (tag: v1.0) First release
* 4938271 Initial commit
@endgit
```

## Straight runs

Commits in a straight run, one parent and one child each, are drawn in one box, so long histories stay short. The newest commit of a branch and the commits where branches meet keep a box of their own.

```rockuml
@startgit
* f6e5d4c (HEAD -> main) Release 1.1
* e5d4c3b Update the changelog
* d4c3b2a Fix the parser
* c3b2a19 Add a parser
* a190807 Initial commit
@endgit
```

## What is ignored

Lines without a `*` hold no commit and are skipped, and so are the `title`, `skinparam` and `<style>` lines of other diagrams: a git graph always has black text and dark grey boxes on white. The preprocessor works as everywhere, so `!include` can bring in a log written to a file.

```rockuml
@startgit
title This title is not drawn
* 3c2b1a0 (HEAD -> main) Third
some text between the commits
* 2b1a09f Second

  * bbbbbbb (gh-pages) Publish the site
  * aaaaaaa Start the site
@endgit
```

A graph whose lines lead nowhere, such as a `|` that stops without a commit below it, cannot be followed: PlantUML draws a crash report for it, and rockuml reports it as not ported.

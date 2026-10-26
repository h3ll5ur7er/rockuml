```rockuml tldr
@startboard
Sign up
+Enter email
++Verify email
+Pick a plan
Use the app
+Create a project
++Invite people
@endboard
```

Between `@startboard` and `@endboard`, rockuml draws a board: a story map of grey cards. A line without `+` starts an activity, drawn in the top row; the lines after it, each starting with one `+` or more, are its releases, drawn in the rows below. The number of `+` is the row, and dashed lines separate the rows.

## Rows and columns

A release goes below the last card of a lower row. The first release below a card shares its column; each further one opens a new column to the right, so an activity is as wide as the releases it fans out to. Rows can be skipped: `+++` right under an activity leaves the two rows between empty.

```rockuml
@startboard
Search
+Keyword
++Suggestions
++Spelling
+++Did you mean
+Filters
+++Saved filters
Checkout
+Cart
++Coupons
@endboard
```

Activities stand side by side in the order they are written. A release before the first activity is an error in PlantUML; rockuml reports it as not supported.

## Cards

Every card is 150 by 70 pixels, whatever its text: long names run over the edge. Names are written with [text formatting](docs/creole), so bold, colours, sizes, sprites and OpenIconic icons work; `\n` does not break a name. The cards keep their grey look and shadow: skin parameters and styles do not change them.

```rockuml
@startboard
**Onboarding**
+<color:red>Must</color> have
++<&person> Profile
+//Nice// to have
Activity with a name too long for its card
+<size:18>Big</size>
@endboard
```

## Titles and decorations

`title`, `header`, `footer`, `caption`, `legend`, `mainframe` and `scale` work as in other diagrams, and so does the `document` style for the background. Space after the `+` is ignored, and a line that starts with spaces before its `+` is an activity whose name starts with `+`.

```rockuml
@startboard
<style>
document {
  BackGroundColor #FFFFEE
}
</style>
title Release plan
caption Two activities, three releases
One
+  Two
Three
+Four
++Five
@endboard
```

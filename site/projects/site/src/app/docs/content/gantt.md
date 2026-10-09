```rockuml tldr
@startgantt
Project starts 2026-11-02
saturday are closed
sunday are closed

[Write the screenplay] as [Script] lasts 10 days
[Cast the heroes] as [Cast] lasts 5 days
[Cast] starts at [Script]'s end
then [Shoot the movie] as [Shoot] lasts 3 weeks
[Shoot] is 30% completed
[Premiere] happens at [Shoot]'s end
@endgantt
```

A Gantt chart puts a project's tasks on a calendar: a bar for each task, arrows for what waits on what, diamonds for milestones. In rockuml you write it as plain English sentences between `@startgantt` and `@endgantt`, and the calendar works itself out, weekends and holidays included.

## Tasks

A task is a name in brackets with a duration: `lasts 5 days`, `lasts 2 weeks`, `lasts 1 week and 3 days`. (`requires` is another word for `lasts`.) An alias, `as [T]`, saves typing later.

```rockuml
@startgantt
[Build the Death Star] as [DS] lasts 3 weeks
[Hire stormtroopers] lasts 10 days
[Train their aim] lasts 1 day
@endgantt
```

`Project starts 2026-11-02` fixes the calendar; without it, the chart counts days from the start. Dates also work as `the 2nd of november 2026`.

## Ordering tasks

Tasks start at the project start unless a sentence says otherwise:

| Sentence | Meaning |
|---|---|
| `[B] starts at [A]'s end` | B begins when A ends |
| `[B] starts 2 days after [A]'s end` | with a gap |
| `[B] ends at [A]'s end` | both end together |
| `[B] starts 2026-11-10` | on a date |
| `then [B] lasts 3 days` | B follows the task before it |
| `[A] -> [B]` | B follows A, with an arrow |

```rockuml
@startgantt
Project starts 2026-11-02
[Collect underpants] as [C] lasts 5 days
[Profit] as [P] lasts 2 days
[P] starts 3 days after [C]'s end
[Plan phase 2] ends at [P]'s end
[Plan phase 2] lasts 4 days
@endgantt
```

`then` and arrows make chains quick to write:

```rockuml
@startgantt
[Design the suit] as [D] lasts 5 days
then [Build Mark I] as [B] lasts 3 days
then [Escape the cave] as [E] lasts 1 day
[D] -> [B]
[B] -> [E]
@endgantt
```

## Milestones

A task that `happens` at a moment is a milestone, drawn as a diamond:

```rockuml
@startgantt
[Forge the One Ring] as [Forge] lasts 10 days
[Ring completed] happens at [Forge]'s end
[Lose the Ring] lasts 5 days
[Lose the Ring] starts at [Forge]'s end
[Bilbo finds it] happens 2 days after [Lose the Ring]'s end
@endgantt
```

## Calendar

Days and dates can be closed, so tasks skip them; a closed weekday closes it every week. Days can also be coloured or named.

```rockuml
@startgantt
Project starts 2026-12-14
saturday are closed
sunday are closed
2026-12-24 to 2026-12-26 are closed
2026-12-31 is colored in salmon
2026-12-21 to 2026-12-23 are named [Crunch time]
[Ship the game] lasts 10 days
@endgantt
```

`2026-12-28 is open` reopens a single day. `[Task] pauses on 2026-12-18` interrupts one task on a date, and `pauses on monday` every Monday. `today is 2026-12-18 and is colored in #AAF` marks the current day.

## Resources

`on {Name}` assigns a task to a resource, `{Name:50%}` to part of one. Tasks of the same resource take turns, and the chart lists every resource's workload at the bottom.

```rockuml
@startgantt
[Fix the bug] on {Gilfoyle} lasts 3 days
[Write the tests] on {Dinesh:50%} lasts 4 days
then [Review] on {Gilfoyle} {Dinesh} lasts 2 days
[Pitch the investors] on {Richard} lasts 2 days
@endgantt
```

`hide resources names` and `hide resources footbox` hide the names on the bars and the list at the bottom.

## Colours and progress

`is colored in Fuchsia/FireBrick` gives a task a fill and an outline colour, and `is 40% completed` shows its progress. Both fit into the task's sentence with `and`.

```rockuml
@startgantt
[Paint the TARDIS] lasts 6 days and is 50% completed
[Paint the TARDIS] is colored in RoyalBlue/Navy
[Polish the sonic screwdriver] lasts 3 days and is 100% completed
[Fix the chameleon circuit] lasts 4 days
@endgantt
```

## Separators and notes

`-- Text --` draws a separator between groups of tasks. `note bottom` after a task attaches a note to it, until `end note`.

```rockuml
@startgantt
[Discover the island] lasts 4 days
then [Find the treasure map] lasts 3 days
-- Grand Line --
then [Cross Reverse Mountain] lasts 2 days
note bottom
  Watch out for Laboon.
  He is friendly, mostly.
end note
then [Reach Whiskey Peak] lasts 3 days
@endgantt
```

## Scale and zoom

`printscale weekly`, `monthly`, `quarterly` or `yearly` changes the unit of the calendar, and `zoom 2` stretches it. `hide footbox` removes the calendar at the bottom.

```rockuml
@startgantt
printscale weekly
Project starts the 7th of september 2026
[Season 1] as [S1] lasts 70 days
[S1] is colored in Lavender/SlateBlue
[Season 2] lasts 63 days
[S1] -> [Season 2]
@endgantt
```

```rockuml
@startgantt
printscale daily zoom 2
hide footbox
Project starts 2026-11-02
[Make coffee] lasts 1 day
then [Drink coffee] lasts 2 days
@endgantt
```

`language de` (or `fr`, `es`, `ja`, and many more) prints month and day names in another language.

## Styling

A `ganttDiagram` style sheet styles tasks, milestones, arrows and separators:

```rockuml
@startgantt
<style>
ganttDiagram {
  task {
    BackGroundColor GreenYellow
    LineColor DarkGreen
  }
  milestone {
    BackGroundColor Gold
  }
  arrow {
    LineColor DarkGreen
    LineThickness 2
  }
}
</style>
[Plant crops] lasts 5 days
[Plant crops] -> [Harvest]
[Harvest] lasts 3 days
[Feed the village] happens at [Harvest]'s end
@endgantt
```

> **Not yet supported:** working hours (`from 9:00 to 17:00 are working hours`). rockuml reports the chart as not ported.

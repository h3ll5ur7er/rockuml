```rockuml tldr
@startchart
h-axis "Quarter" [Q1, Q2, Q3, Q4]
v-axis "Revenue" 0 --> 100
bar "Sales" [45, 62, 78, 91] #4472C4
line "Target" [50, 60, 70, 80] #ED7D31
legend right
@endchart
```

Between `@startchart` and `@endchart`, rockuml draws a chart: series of values as bars, lines, areas or scatter markers, over a horizontal axis of categories and a vertical axis of numbers. The plot is 300 pixels high and 60 pixels per category wide, at least 400.

## Axes

`h-axis [A, B, C]` names the categories, in quotes or not, and an optional quoted title comes first. `v-axis "title" min --> max` gives the value axis its title and range; without a range, the axis runs from 0 to 100 and grows to take in every value. The values get six ticks, unless `spacing` sets the step between them or `ticks` names them; `label-top` puts the title level above the axis instead of along it. `h-axis … spacing 2` labels every second category, and `label-right` puts the title at the right end of the axis.

```rockuml
@startchart
h-axis "Day" [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] spacing 2 label-right
v-axis "Load" 0 --> 100 ticks [0:"Idle", 50:"Busy", 80:"High", 100:"Full"] label-top
line "CPU" [10, 35, 50, 45, 80, 95, 60, 40, 30, 20]
@endchart
```

`v2-axis` adds a second value axis on the right; a `line`, `area` or `scatter` followed by `v2` goes by it. `grid` after an axis draws grid lines across the plot: at the categories and at tenths of the horizontal axis's range, and at tenths of the vertical one's, or at its `spacing`. When a range holds zero, the other axis crosses it there.

```rockuml
@startchart
h-axis "Month" [Jan, Feb, Mar, Apr, May, Jun]
v-axis "Temperature" -5 --> 30 grid
v2-axis "Rain" 0 --> 200
line "Paris" [5, 7, 11, 15, 19, 23]
line "Oslo" [-3, -2, 2, 7, 12, 16] #9467bd
line "Rain" [50, 40, 45, 50, 60, 55] #17becf v2
legend left
@endchart
```

## Series

`bar`, `line`, `area` and `scatter` each add a series: an optional `<<stereotype>>`, an optional quoted name (`bar0`, `line1`… otherwise), the values in brackets, an optional colour and `labels` to print each value. Several bars stand side by side in each category, or on top of each other with `stackMode stacked`, negative values below zero. Areas always stack, each on the ones before it. Scatter markers are circles unless `<<square>>` or `<<triangle>>` follows.

```rockuml
@startchart
h-axis ["Jan", "Feb", "Mar", "Apr"]
v-axis "Balance" -50 --> 150
stackMode stacked
bar "Income" [60, 70, 55, 80] #2ca02c labels
bar "Bonus" [20, 15.5, 30, 10] #bcbd22
bar "Costs" [-20, -35, -10, -40] #d62728 labels
legend bottom
@endchart
```

```rockuml
@startchart
h-axis [Mon, Tue, Wed, Thu, Fri]
v-axis "Visitors" 0 --> 300
area "Mobile" [50, 80, 65, 90, 120] #1f77b4 labels
area "Desktop" [40, 60, 70, 55, 80] #2ca02c
scatter "Outages" [10, 250, 30, 200, 20] <<triangle>>
legend right
@endchart
```

A series without a colour takes its style's: the background of the `bar` and `area` styles, the line colour of the `line` style, as in PlantUML, which gives every series the same grey unless styles say otherwise. Scatter markers are drawn in the `scatter` style's `MarkerColor`, black by default, whatever colour the series names.

## Coordinates

Lines and scatters can give points as `(x, y)` pairs instead of values by category, over a numeric horizontal axis: `h-axis "x" min --> max`, ticked every tenth of its range or every `spacing`. Every point must lie in the range, and all series must be written the same way.

```rockuml
@startchart
h-axis "x" -5 --> 5 spacing 1 grid
v-axis "y" -10 --> 30 grid
line "Square" [(-5,25), (-4,16), (-3,9), (-2,4), (-1,1), (0,0), (1,1), (2,4), (3,9), (4,16), (5,25)] #1f77b4
scatter "Samples" [(-4.5,20), (-2.5, 5), (0.5, -2), (3.5, 14)] labels
legend right
@endchart
```

## Horizontal bars

`orientation horizontal` lays bars across: `v-axis [A, B, C]` names the categories, down the left side, and `h-axis "title" min --> max` the values, along the bottom.

```rockuml
@startchart
orientation horizontal
v-axis [Apples, Oranges, Pears]
h-axis "Kilograms" 0 --> 50
bar "Harvest" [32, 18.25, 44] #8FBC8F labels
legend top
@endchart
```

## Legends and annotations

`legend left`, `right`, `top` or `bottom` lists the series beside the plot. A `legend` block with text, as in other diagrams, is drawn around the chart. `annotation "text" at (category, value)` writes a text above a point, or 40 pixels away with an arrow to it when `<<arrow>>` follows; the text goes below the point when it is too near the top. Annotations at unknown categories are left out.

```rockuml
@startchart
h-axis [Jan, Feb, Mar, Apr, May]
v-axis "Users" 0 --> 1000
line "Signups" [120, 300, 650, 900, 820] #1f77b4
annotation "Launch" at (Mar, 650) <<arrow>>
annotation "Peak" at (Apr, 900) <<arrow>>
annotation "Dip" at (May, 820)
@endchart
```

## Styles

The `chartDiagram` style sets the chart's colours and fonts, with `bar`, `line`, `area`, `scatter`, `axis` (and `hAxis`, `vAxis`), `grid`, `legend` and `annotation` inside it. Bars take `BackGroundColor`, `LineColor`, `LineThickness` and `BarWidth`, the part of a category a single bar takes (0.6 by default); scatters take `MarkerColor`, `MarkerShape` and `MarkerSize`. A series' stereotype picks a style of its name. `title`, `header`, `footer`, `caption` and `scale` work as in other diagrams.

```rockuml
@startchart
<style>
chartDiagram {
  bar {
    BackGroundColor #8FBC8F
    LineColor #2F4F4F
    BarWidth 0.8
  }
  .warm {
    BackGroundColor #FF8C00
  }
  scatter {
    MarkerColor #DC143C
    MarkerShape square
  }
  axis {
    LineColor #0000FF
    FontColor #000080
  }
}
</style>
title Points per team
h-axis "Team" [Red, Blue, Green]
v-axis "Points" 0 --> 50
bar "Plain" [20, 35, 25]
bar <<warm>> "Warm" [15, 28, 40]
scatter "Hits" [10, 20, 30]
legend right
@endchart
```

Values the chart cannot read are errors: a number that is not one, coordinate pairs in a bar or area, points outside the horizontal axis's range.

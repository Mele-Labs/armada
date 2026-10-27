# Spacing and shape

**Part of:** [Design System — UI & Voice](../design-system.md).

4px base grid: `1`=4, `2`=8, `3`=12, `4`=16, `6`=24, `8`=32, `12`=48.
Deliberately tight.
Table rows and list rows 28px, header rows 28px.
Card padding 20px, not 24.
Section gaps 24px, not 48.
If it feels slightly cramped against normal web instincts, it is correct: this window holds a job list, a diff, and a graph view at once.

**Three control heights, for every tab: 24, 28 and 32.**
`--h-control-sm` is 24px, for a control inside a row or a toolbar.
`--h-control` is 28px, for a default button, input or select.
`--h-control-lg` is 32px, for the title bar's places and its search.
A fourth height is a design conversation, not a `height:` in a component.
The owner chose the Cockpit's density as the app's on 26 Oct 2026: the Cockpit had 24 and 28 in its own scope, and the other tabs 32, 36 and 38.
This supersedes the 20 Aug 2026 lift, which raised controls to 32 and 36 with the type.

**One page gutter and one panel gap, the same on every tab.**
`--pad-page` is 12px: the margin round a page, at its sides, its top and its foot.
`--gap-panel` is 6px: the space between two panels on a page, and between a panel and the one beside it.
The shell holds the gutter, so a page sets no side padding of its own.
Why: Studios, Logbook, Issues and System measured 32, 32, 22 and 16px from the window's edge, and the Cockpit 6.

**Strokes are one width and three roles.**
Every edge is `--border-width`, 1px.
A panel takes `--border-default`.
A control takes `--border-strong`.
An inner tile, a bay or a chip takes `--border-subtle`.
Measured on 26 Oct 2026, the same panel edge drew as 15% white on the Cockpit, solid slate on the Kit browser and Logbook, and 10% white on tiles.
A bay drew at 2px.

Every one of those is a token in `tokens/spacing.css`, which is the
authority on the value. Read it rather than retyping a number from here.

**A row height token is a floor, not a cap.** Rows are padding-driven
where content can grow, so the token keeps rows aligned down a column
without truncating one that needs more. Labels align with their fields,
and content left edges align with their header's left edge.

**Controls repeated down a list sit in fixed columns.**
A switch, a mark or an act that more than one row of a list can carry has a column of its own, one control wide, and every row keeps that column whether it carries the control or not.
A row's controls share the centre line of its first line; anything a row adds under its title, such as a pull request or a Job, is a line of its own beneath and never moves them.
Lay the row out as a grid with the trailing columns sized by `--h-control`, never as a flex row whose controls take the width they happen to need.
Why: a flex row shifts its switch left by one control wherever a neighbour has one more mark, and the owner asked twice on 23 Oct 2026 for the Mods rows' controls to line up.

```
--radius-sm  3px    chips, badges
--radius-md  6px    buttons, inputs, rows and tiles, a flat card
--radius-lg  8px    dialogs, panels, a card on the canvas
```

`--radius-md` is 6px in `tokens/spacing.css` itself.
It was 5px there and 6px in this contract and in the look, so a control drew at one or the other by whether the look loaded.

**A card's radius follows its surface.** On the canvas it takes the panel
treatment under Depth and `--radius-lg`, the same corner as the panel beside
it. Flat — inside a sheet, a dialog or another card — it is `--radius-md`.

**Circles belong to avatars.**
A status dot, a lamp and the drone and ring drawings are round too, because they are marks and not controls.
A button, a badge or a step is a rounded box.
A Radio, a Switch's thumb and a Slider's thumb stay round: a square one reads as another control.

**Four heights of shadow**, under Depth: rest, hover, lifted and overlay. A card lifts off the
canvas on `--shadow-rest`. A floating layer (dialog, popover, dropdown) lifts off the card on
`--shadow-overlay`, the deepest. Nothing inside a card takes a
shadow: a shadowed row inside a card reads as a second card. The one solid
accent on a view takes `--shadow-primary` (see Button).

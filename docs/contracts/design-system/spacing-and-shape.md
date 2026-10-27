# Spacing and shape

**Part of:** [Design System — UI & Voice](../design-system.md).

4px base grid: `1`=4, `2`=8, `3`=12, `4`=16, `6`=24, `8`=32, `12`=48.
Deliberately tight. Table rows 36px, header rows 32px. Card padding
20px, not 24. Controls 32px (sm) / 36px (default). Section gaps 24px,
not 48. If it feels slightly cramped against normal web instincts, it is
correct — this window holds a job list, a diff, and a graph view at
once.

Every one of those is a token in `tokens/spacing.css`, which is the
authority on the value. Read it rather than retyping a number from here.

**That grid is Default, one of three spacings a person picks.**
Settings, Bridge, Spacing (`spacing.density`) offers Compact, Default and Roomy, as Gmail does (26 Oct 2026).
Compact is a quarter tighter and Roomy a quarter looser: the steps, the row and control heights and the paddings move, and type, hairlines, radii and the title row keep their size.
Bridge puts the choice on the root as `data-spacing`, and `tokens/spacing-sets.css` holds the two other sets.
A surface spells its sizes in the tokens and so follows the choice with nothing of its own.
Where a smaller step would cut what a control holds, the control keeps Default's size in Compact: Flight control's keys do (`deck.css`).

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
--radius-sm  3px    badges, small controls
--radius-md  6px    buttons, inputs, rows and tiles, a flat card
--radius-lg  8px    dialogs, panels, a card on the canvas
```

**A card's radius follows its surface.** On the canvas it takes the panel
treatment under Depth and `--radius-lg`, the same corner as the panel beside
it. Flat — inside a sheet, a dialog or another card — it is `--radius-md`.

No full-round pills except avatars.

**Four heights of shadow**, under Depth: rest, hover, lifted and overlay. A card lifts off the
canvas on `--shadow-rest`. A floating layer (dialog, popover, dropdown) lifts off the card on
`--shadow-overlay`, the deepest. Nothing inside a card takes a
shadow: a shadowed row inside a card reads as a second card. The one solid
accent on a view takes `--shadow-primary` (see Button).

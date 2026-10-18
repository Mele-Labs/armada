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

**A row height token is a floor, not a cap.** Rows are padding-driven
where content can grow, so the token keeps rows aligned down a column
without truncating one that needs more. Labels align with their fields,
and content left edges align with their header's left edge.

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

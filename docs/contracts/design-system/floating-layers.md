# Floating layers

**Part of:** [Design System — UI & Voice](../design-system.md).

Where a layer opens, which edge it aligns to, and what happens when it does
not fit. Anchored layers open against a trigger; framed layers open against
the window. Flip and alignment apply to the anchored family only.

**Placement is CSS anchor positioning.** The trigger carries an `anchor-name`,
the layer names a preferred area and an ordered list of fallbacks. Bridge's
renderer is one known Chromium, so this costs no positioning library and no
measuring in JavaScript.

## Anchored layers

| Layer | Opens | Preferred alignment |
| --- | --- | --- |
| Dropdown menu | Below the trigger | Trailing edges flush, caller may set leading |
| Popover | Below the trigger | Leading edges flush, caller may set trailing |
| Tooltip | Below the element it wraps | Leading edges flush |
| Inspect card | Below the trigger, or beside it for a row in a column | Leading edges flush; flips where there is no room |
| Split-button menu | Below the whole control, not the caret | Leading edges flush |

**Alignment is a preference, not a rule.** The trigger's edge is what the
layer tries first; the window's edge is what overrides it.

## Collision

**A layer that does not fit flips. It never squashes.** Width and height come
from tokens, so a narrow gap is not a reason to reflow what is inside.

**Fallbacks are tried in order and the first that does not overflow wins.**

| Order | Try | Gives up |
| --- | --- | --- |
| 1 | Preferred side, preferred alignment | Nothing |
| 2 | Preferred side, opposite alignment | The trigger's edge |
| 3 | Opposite side, preferred alignment | The side |
| 4 | Opposite side, opposite alignment | Both |

Overflow is measured against the window, never against the layer's parent. A
menu that fits inside its card and runs off the screen has not fitted.

## Framed layers

These have no trigger, so flip and alignment do not apply to them.

| Layer | Opens |
| --- | --- |
| Dialog | Centred in the window on both axes |
| Guide card | Centred in the window on both axes |
| Sheet | Full height, flush to one side edge, trailing by default. **Floating**: under the title row and held `--space-4` off every edge, rounded and bordered — Helm's dock as a layer. A screen's own reading takes it where a sheet contained in the content column read as cut off (the owner, 29 Sep 2026) |
| Toast | Bottom trailing corner, inset `--space-6` |
| Command palette | Opens horizontally centred across the window, the field top-anchored at 20% of the viewport's height, and opens where it was last dragged. It does not open on the title row's one bar, which is not centred (the owner, 15 Oct 2026) |
| Dispatch sheet | Horizontally centred across the window and wider than the palette, its top at 12% of the viewport's height, over a scrim. The one sheet for `n`, the deck's Dispatch tile, New Job and the palette's `⌘↵` |

**A guide card is framed even though a `?` raised it.** It is the one layer
that can also open with no trigger at all — the first time a person meets a
piece — and a layer that anchored when it was pressed for and centred when it
was not would be two layers wearing one name. It takes `--w-dialog-wide`,
because a guide is three paragraphs and a picture.

The palette's offset, and why it is not centred, are under Command palette.

**A sheet's width is a fraction of the ground it takes, not a column width.**
`default` is `--w-sheet`, 480px. `wide`, `widest` and `reading` are read
against the window instead, because what has to fit each is a reading rather
than a value: `wide` (62%) is the activity log's file rail beside a line that
does not wrap, `widest` (76%) is the diff's file rail beside a patch line that
does not wrap, and `reading` (88%) is the run sheet's own — a fixed 240px list
of the Manifest's Setup, Checks and Commands, with everything past it the run's
own output. `--armada-sheet-wide`, `--armada-sheet-widest` and
`--armada-sheet-reading` have no token behind them: `packages/tokens` carries
no width scale for an overlay panel, only the sidebar's own range, which does
not describe this. Reported, not minted here. `dock` is `--w-dock`, where
Plan's task, group and wave-Job panels open.

**Every sheet resizes, and each kind of sheet remembers its own width** (the
owner, 2 Oct 2026). Helm's handle sits on a sheet's inner edge, contained or
floating, and drags it from `--w-dock-min` to the far edge of the area it is
drawn in, keeping only the handle's gap and a floating sheet's margin. There is
no other ceiling: "I should be able to go as far as I want." A width
remembered from a wider window draws clamped to today's area. A sheet beside
another keeps its `--space-4` gap at whatever width that one was dragged to,
so it never passes that one's edge. Each sheet names its kind (`plan-task`, `drone`, `job-diff`),
and a kind opens at the width it was last dragged to, across a restart: a log
panel dragged wide leaves Plan's task panel as it was. One shared width and
none at all were the alternatives, and he chose this over both, at a little
more to remember. A kind never dragged opens at its size. Helm folded into a
sheet has no kind: it shares the dock's width, so a drag of either sets both.
At the floor a sheet is flush to both edges and has no handle.

## Stacking

**A floating layer takes a stacking token and never a written number.** The
order is the order a person meets them.

| Token | Layer | Why it sits here |
| --- | --- | --- |
| `--z-dock` | Helm's dock, and the workflow canvas's inspector | Over the content, under anything opened from inside it |
| `--z-menu` | Dropdown, popover, split-button menu | Opens over the surface |
| `--z-tooltip` | Tooltip, inspect card | Explains the thing a menu is over |
| `--z-modal` | Dialog, sheet | Interrupts both |
| `--z-toast` | Toast | Over every panel, its scrim and every dialog, so a failure raised from inside one is pressable where it appears |
| `--z-palette` | Command palette | The way out of anything |

A number meaning "above my sibling" means "under every other layer" the moment
its layer resolves against the window rather than its parent.

**The palette stays over a toast.** A person summons it, it takes the keyboard
while it is open, and it closes on `Esc`, so a toast is never under it for
longer than one command. Its own *Copy debug info* acts on the newest failure
toast, so nothing a toast offers is out of reach while the palette is up.

**Esc belongs to the toast while focus is inside it.** There it dismisses that
toast and nothing else; the panel under it stays. From anywhere else, `Esc`
closes the top panel as it always has. A dialog opened from a toast (File an
issue) takes `Esc` itself.

**`--z-dock` is the layer a person works beside rather than through**, so it is
the one with no scrim and the only one under `--z-menu`. Two things take it, for
one reason: Helm's dock over the content, and the workflow canvas's inspector
over the canvas — a person reads a node and replies to its Drone, so what is
under each stays live and one press away. It still needs a token: a card lifts
itself a step while it is hovered, which is enough to paint over a layer left at
`auto`.

## Placement resolves before paint

**Nothing animates into place.** Motion forbids an entrance animation on a
data surface, and a floating layer is one as soon as it carries content.

Hover, focus and dismissal still transition on `--duration-fast`. What is
forbidden is the layer arriving, not what it does once it is there.

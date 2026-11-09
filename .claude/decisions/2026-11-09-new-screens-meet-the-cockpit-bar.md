# What should a new screen look like?

**Said 2026-11-09.** Three times he found a page an agent designed monotone, and asked where the rule came from.

| When | He said |
|---|---|
| 23 Oct 2026 | "Why is everything about armada by default from agents so monotone? I thought we removed all of those rules." |
| 26 Oct 2026 | "Is there a design document in the repo that says to be monotone?" |
| 9 Nov 2026, on the Operator's Log | "Once again a newly designed page lacks color, depth and meaning" |
| 9 Nov 2026 | "Whatever design doc in the repo is dictating that agent designs are monotone, flat and lack meaning please dispatch a job to remove those docs or rewrite." |

The Cockpit is the bar for any new surface.

| | What it means on the screen |
|---|---|
| Colour | State colour on frames and edges, coloured glyphs for a row's kind and state, chips with fill, header bands with weight |
| Depth | Elevation by lightness steps, hairlines and shadow, so a card reads as a thing over its ground |
| Meaning | Colour says what a thing is or what state it is in, so a glance tells kinds apart |
| Weight | Bold means weight, not volume. The `PoolSlots` slot cards (4 Oct 2026): a solid state-coloured header band with icon and state, a dark body, one accent per card |

The only no's that stand are his of 16 Oct 2026: no gradients on controls, no glows, no grain or sheen.

What it changed: `docs/contracts/design-system/the-product.md` states the bar, and `tokens.md` (Depth), `hard-rules.md` (rules 3 and 5), `spacing-and-shape.md` (the header band), `iconography.md` (rule 1) and the skills that brief a screen were rewritten to it.
What it kept: status hue maps to the Job state machine and is never chosen, and the accent is for interactive affordance.

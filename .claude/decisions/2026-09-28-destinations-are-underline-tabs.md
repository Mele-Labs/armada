# Two segmented controls, one over the other. Which one gives way?

**Decided 2026-09-28.**

Job detail drew its six destinations as the contract's segmented control — a
`--bg-raised` track with an `--accent-muted` fill on the chosen one — and the
Record destination drew its filters as a second segmented control of the same
weight directly beneath it. Two identical controls stacked, neither subordinate
to the other.

He put the built strip beside the artboard:

> "When we have two segment controls that are stacked, there is no hierarchy to
> them. In the designs this was accounted for because the job destinations were
> tabs and then the segment controls were nestled in the panel that was
> associated with them"

**Chosen:** the destinations become underline tabs — plain text, no track, a
rule under the active one — and the filled segmented control stays what a
panel's own filters are.

**Cost he took:** it reverses half of `#1383`, which made tabs filled on the
argument that the rail's selected row is a fill and a panel's tabs sit in the
same eyeline. That argument still holds for the filters; it is the destinations
that stop agreeing with the rail.

**Why it is in character:** the hierarchy is bought with two kinds of control
rather than with spacing, which is
[[2026-09-22-helm-overlays]]'s instinct — remove the fact that needed working
around rather than document it. A destination is a place you go and a filter
narrows what is already on screen, and the two had been made to look alike.

**Where it landed:** `packages/screens/src/detail-tabs.tsx`, and
`.armada-destinations` in `packages/components/src/screens/screens.css`. The
filters moving inside the panel they filter is Record's own board pass and is
not built here.

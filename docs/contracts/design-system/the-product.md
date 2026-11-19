# The product

**Part of:** [Design System — UI & Voice](../design-system.md).

Armada is a personal AI workflow orchestration system. It dispatches AI
coding agents (Drones) against real Git repositories, monitors them, and
escalates when they misbehave. Single user, local, always open on a
second monitor across a working day.

**Surfaces:** Bridge is the operational surface group — Job Board,
Alerts, Doctor, Manifest. The count is not a contract: a surface earns a
place in the window's tabs or System strip where a journey needs one, and the roster lives on
Bridge. Active Jobs, Reviews and the Activity Feed were retired into the
Board, which holds every Job with state as a filter. Alerts stays because
an alert is a condition on a Job rather than a status a Job holds. The Operator
is not a surface — see Window and layout model, Content area, below.

**The screen's job:** at a glance, tell one person what is running, what
needs them, and what broke.

This is an instrument panel, not a marketing page. No hero sections, no
decorative iconography, no illustration. Density and legibility win over
impact. The two illustrations are the owner's: an empty merge line, and a
review tool that found nothing (`iconography.md`, Where the answer is no icon).

**The Cockpit is the bar for every new surface.** The owner has asked for it
three times: *"Why is everything about armada by default from agents so
monotone? I thought we removed all of those rules."* (23 Oct 2026), *"Is
there a design document in the repo that says to be monotone?"* (26 Oct), and
on the Operator's Log, *"Once again a newly designed page lacks color, depth
and meaning"* (9 Nov). A surface meets it on three counts.

| | What it means on the screen |
|---|---|
| Colour | State colour on frames and edges, a coloured glyph for each row's kind and state, chips with fill, header bands with weight |
| Depth | Elevation by lightness steps, hairlines and shadow, so a card reads as a thing over its ground |
| Meaning | Colour says what a thing is or what state it is in, so a glance tells kinds apart |

Bold means weight, not volume: a pass that piles everything on reads as a
kitchen sink. The `PoolSlots` slot cards are his example (4 Oct 2026): a solid
state-coloured header band with icon and state, a dark body, one accent per
card. The only no's are those under Depth in Tokens: no gradients on a
control, no glows, no grain or sheen.

One more no, for the pages, sketches and mocks agents draw (19 Nov 2026: *"I hate these little edge borders"*):
no coloured stripe on one side of a box, card, callout, table cell or diagram node.
That is `border-left: 3px solid`, `border-top: 3px solid`, `box-shadow: inset 4px 0 0`, or a strip down a node's side in SVG.
Use a tint fill of the hue, a full hairline outline in the hue, a coloured glyph or dot, a chip, or a header band across the whole top.
Bridge's own leading edges in this contract (fenced blocks, blockquotes, sketch node kinds) stay.

**An instrument panel still has depth.** It sits open all day, and on a quiet
day no state is lit, so a screen whose only colour is state reads as a dead
one. A row's kind keeps its hue and a card keeps its lift on a quiet day. Cards are glass lifted off a canvas lit in faint pools of light (see
Depth, under Tokens). Light, glass and shadow carry no information. A gradient
is allowed only as that light, and never where a person reads a state.

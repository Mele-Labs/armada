# What does a Job's Field draw?

**Decided 2026-10-22.** His words, on the Field in the right column of an open
Job: *"It only shows a subset of the repo it's working on. It should show the
whole repo and maybe drones from other jobs can be faded so the drones in the
current job are focused and easy to see."* The Field drew only the folders the
Job had touched, so its shape changed as the Job moved.

**Chosen: the whole repository, other Jobs faded, a glow on each file, and a
replay slider.** He picked among SVG mockups of one Job and of four drones.
Seeing them together: *"This is sick"*, then *"Let's land it"*. Built in stages;
the first is the four rows of the table.

| Asked | He chose | The cost he took |
|---|---|---|
| What the floor is | Every area Fleet names, always, with the plots the Job touched outlined in the accent | The Job's own plots are smaller on a big repository, and the camera opens on its drones to make up for it |
| What other Jobs show | Their drones at about a quarter strength, with no beam, label or action, and their card on hover | The Job's Field is no longer only the Job |
| How the Job's past shows | A per-file glow: amber for a write, a cool hue for a read, fading linearly over about ten minutes and five, from Fleet's burst times | It is gone after ten minutes. A file touched an hour ago looks like one never touched |
| How to look back | A slider along the foot scrubs the Job's reads and writes: glow, drone and a numbered route as they were, live at its right end | One more control under the plan |

**Approved for later stages, so they are on file.**

| Stage | What he approved |
|---|---|
| A diff lens | The file being written lifts as a slab and a card types out the change, following the drone. With several drones one card follows whoever started writing last and pressing a drone pins it; the other writers show a smaller lifted file with a `+N −M` count. **Stage 3, built**: the card types only the lines new since Fleet's last answer, and the earlier files behind it come from the Job's `session.edited` entries |
| Off the code floor | A drone not coding leaves the floor: a pad at the edge with one light per check and a red line from a failing check to the file it blames (the file and line Fleet reads from the failed run's log, decided the same day; the run's `target` is not read for it, so a whole-repository run blames too and a run with no location in its log draws no line), a floating window while walking, dots circling while thinking, an arc to another Job while messaging it. **Stage 2, built** with the subagents below and a strip of the Job's drones over the Field, all but the arc to another Job. **Stage 4, built**: the arc, from Fleet's `session.messaged`, with packets travelling from the sender to the receiver and the message quoted at its middle; the other Job's drone un-fades while linked |
| Subagents | Smaller, linked to the main drone by a faint line, lifting off it and returning into it. **Stage 2, built** |

**Rejected.** File towers. A magnifier lens dragged round the field. A heat map
that accumulates and never fades. A second ring for checks: the ring under the
drone stays the Job's progress, `2026-10-14-a-session-builds-a-ring.md`.

**What it keeps.** Nothing is drawn from what an agent said: the glow, the route
and the drone on the slider are read from the `session.edited` and
`session.read` entries Fleet recorded, at their own times. Reduced motion changes
none of it, since a glow is not motion.

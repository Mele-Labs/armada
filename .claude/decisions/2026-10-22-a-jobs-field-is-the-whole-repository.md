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
| What the floor is | Every area Fleet names, always, with the plots the Job touched outlined in the accent | The Job's own plots are smaller on a big repository: the camera rests on the whole of it, and he zooms or presses a drone to come in |
| What other Jobs show | Their drones at about a quarter strength, with no beam, label or action, and their card on hover | The Job's Field is no longer only the Job |
| How the Job's past shows | A per-file glow: amber for a write, a cool hue for a read, fading linearly over about ten minutes and five, from Fleet's burst times | It is gone after ten minutes. A file touched an hour ago looks like one never touched |
| How to look back | A slider along the foot scrubs the Job's reads and writes: glow, drone and a numbered route as they were, live at its right end | One more control under the plan |

**Approved for later stages, so they are on file.**

| Stage | What he approved |
|---|---|
| A diff lens | The file being written lifts as a slab and a card types out the change, following the drone. With several drones one card follows whoever started writing last and pressing a drone pins it; the other writers show a smaller lifted file with a `+N −M` count. **Stage 3, built**: the card types only the lines new since Fleet's last answer, and the earlier files behind it come from the Job's `session.edited` entries |
| Off the code floor | A drone not coding leaves the floor: a pad at the edge with one light per check and a red line from a failing check to the file it blames (the file and line Fleet reads from the failed run's log, decided the same day; the run's `target` is not read for it, so a whole-repository run blames too and a run with no location in its log draws no line), a floating window while walking, dots circling while thinking, an arc to another Job while messaging it. **Stage 2, built** with the subagents below and a strip of the Job's drones over the Field, all but the arc to another Job. **Stage 4, built**: the arc, from Fleet's `session.messaged`, with packets travelling from the sender to the receiver and the message quoted at its middle; the other Job's drone un-fades while linked |
| Subagents | Smaller, linked to the main drone by a faint line, lifting off it and returning into it. **Stage 2, built** |

**Changed after he used it on his own Jobs, the same day.**

| What | Now | Why |
|---|---|---|
| A walk and a write | A fresh write while the turn works takes the drone to its file, past a walk still open; the walk has it back once the write is not fresh | A walk waits on him for hours, and the diff card never showed on a Job that had one. He chose *"Writing wins"* |
| The camera | The Field rests on the whole repository with the Job's drones on it, as the Cockpit map does; neither the card nor a pick in the Drones menu moves the camera; he zooms to come in | Framed on the Job's drones it was always zoomed in, and each new card size moved it. *"It should show the whole field so I can see the drones for this specific job"* |
| The card in the panel, and the pin | The card is about half the panel wide with the six newest lines, over the map. A pinned drone that stops writing hands the card to whoever writes; the pin stays on the strip | The card took the whole panel, and a pin on a drone that stopped writing left no card until the same strip button was pressed again. *"I somehow get out of it and then can never enter it again"* |
| The hover card | It takes the pointer. Off the thing it is about, it waits the inspect card's grace before closing or going to another thing, and the pointer on it holds it | Crossing from a drone to its card passed over plots that took the card in 90 ms. *"The hover registers on something else on the field and the card goes away"* |
| Cards past the panel | While the pointer or focus is in the panel, the diff card and the hover card are drawn over the app at the size the Field large draws them, reaching toward the thread. A grace after both leave, or on Esc, they go back inside, the diff card small | He asked for it: *"When I am not actively focused on the panel nothing should come out of it. But if I'm interacting with the panel the cards could come outside the panel"* |
| The drones over the map | One Drones menu. Its trigger names the drone the card follows and what it is doing, its name cut short before its word. Newest writer first, then the Job's drone and each subagent with its word, a finished one greyed; a pick pins it, a second pick of the same drone keeps it, and Newest writer lets go | In the side panel the strip of buttons cut off mid-word. *"Can you turn this into a stylish dropdown?"* |

**Rejected.** File towers. A magnifier lens dragged round the field. A heat map
that accumulates and never fades. A second ring for checks: the ring under the
drone stays the Job's progress, `2026-10-14-a-session-builds-a-ring.md`.

**What it keeps.** Nothing is drawn from what an agent said: the glow, the route
and the drone on the slider are read from the `session.edited` and
`session.read` entries Fleet recorded, at their own times. Reduced motion changes
none of it, since a glow is not motion.

# How does Bridge show the worktree slot pool?

**Decided 2026-10-04.**

He asked for *"somewhere in Bridge where I can see the worktrees that are set up: whether each is leased out or open, whether it's warm, how far behind it is, what branch it's checked out to, what job is assigned"*, and chose rows in the existing Cleanup surface over a surface of its own.

**Chosen: a bay grid, from three previews.** He walked the table first: *"The icons and colors dont do enough to indicate free/held/stale slots. I think we need to think outside of the box. Maybe something better than a table. Like a card/panel per slot with more style around its availability."*

- **A bay per slot**, equal tiles in a grid. Held is a filled card under a band in the leased hue; free an open dashed outline holding its name, FREE and its warmth; stranded hatched in the warning hue with its reason; a slot not made a faint ghost.
- **Colour on free and held bays.** *"there should be some color/style added to it to make things stand out more. Color on icons and rows that are open/leased."* The hues are `--slot-*`, aliases of Job hues in `packages/tokens/src/status.css`, and rule 6 of `docs/contracts/iconography.md` carries the bay's exception.
- **The state word is in caps**, HELD, BUSY, STRANDED, FREE, as the preview drew it: a third place in the design system's ALL CAPS list.
- **A Job holder reads as a Job and a link**: *"There should be an icon or something indicating this is a job. It hsould be clear also that I can click on it"*. `box`, the Record's drawing of the Job, before its title in `--accent`.
- **Each figure is named.** *"I had no idea this was commits behind."* `7 behind` and `2 hours` say what they are on hover.
- **The eight glyphs of group `Worktree slot`** in `packages/icons/icons.toml` are Specified, accepted on the walk `aPoolOfWorktreeSlots`.

**Cost he took:** slot states borrow Job hues below Job level, and a bay's marks carry their bay's hue rather than a badge's.

**Not here:** rescuing a stranded slot, which ships on a later branch. The bay leaves room for it.

**Where it landed:** `fleet/slot-pool-in-cleanup`.

## Reshaping the pool from the grid

**Decided 2026-10-04**, after the grid: *"instead of the number of slots a manifest can have, we can quickly choose to just add a new slot from here, or remove a slot or temporarily restrict the number of slots."*

- **The size is this machine's**, kept in `.armada/slots/pool` beside the lease records and never committed. `setup.worktrees` stays the default for a fresh machine.
- **A closed bay stays closed until reopened**, across restarts. A held bay finishes its lease first.
- **Remove only a free or a not-made bay.**
- `door-closed-locked`, `folder-plus`, and the `door-open` and `trash-2` uses for Reopen and Remove are Specified, accepted on the walk `reshapingTheSlotPool`. `--slot-closed` aliases rejected's violet.

**Where it landed:** `fleet/slot-pool-controls`.

## Cleanup as one grid, and a panel per tile

**Decided 5 Oct 2026.** He found Cleanup confusing: *"The section below the bays is very wordy but maybe I should have been looking there. Why are they two separate sections. I should be able to clean up by clicking on a bay to open a panel and manage that bay."*

- **The lower list is gone**, folded into the grid. It predates the pool: each Job's worktree is a slot, so the same worktree showed twice with different acts. A Job's worktree outside the pool is a tile after the bays and opens the same panel.
- **The bulk "Clean up what you choose" press is gone** for a per-tile Clear. The safety stays: its confirm names the files it destroys and how long they have sat, and the branch it keeps. Delete branch and Forget Job confirm too.
- **One trailing panel for every tile**, holding what the worktree holds and the acts that fit it. The bay's own icon acts and the separate Finding sheet moved into it, so a tile says only its state.
- **Pause and Resume are not here.** Another branch adds them; the acts row has room.

**Where it landed:** `bridge/cleanup-one-grid`.

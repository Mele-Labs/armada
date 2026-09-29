# Where do a Job's Drones go, and what does opening one show?

**Decided 2026-09-29.**

On the Record sheet he asked for a tab nobody had drawn: *"we need a drones tab
on the job page. This will show the running drones and I can open them up to
follow what that drone is doing."* Then what it is for, in the redesign where
each task gets its own Drone: *"it would show running Drones, drones that are
completed or have been killed, and I can peek into the entire transcript for
that drone."*

## Settled before the build

- **Which Drones:** every one the Job has used across its tasks — running,
  done, and killed.
- **What opening one shows:** its whole transcript.

## Asked, and answered

| Question | Chosen | Cost he took |
|---|---|---|
| Where in the strip, and does it carry a count? | **After Record, no count.** Pulse keeps its *Drones running* | Nothing on the tab says how many are running; a reader looks at Pulse or opens the tab |
| A note to one Drone from its sheet? | **Mocked, as the Plan task sheet does** | One more place drawn ahead of Fleet, which redirects the Job and not one Drone (#1536) |
| Running first, or task order? | **Both, and the reader toggles** — his words: *"Build both and let the user toggle."* | A second control in the panel's head, beside the filter |
| Where a Drone is killed, once there is one per task? | **In its sheet, and the Job header becomes *Kill job*** — his words: *"Sheet + Header just becomes 'Kill Job' which kills the entire job."* | The header no longer ends a Drone and leaves the Job open; that is only reachable from the Drones tab. Fleet ends the Job's one Drone, so the sheet's kill is mocked against the Drone it names |
| What the transcript's left column says, where it read the stream's raw `called`, `said`, `instructed`? Asked because he wrote *"Should this be 'Tool' or something instead of 'Called'?"* | **Who, not what**: Drone, Armada or Fleet, the activity log's three voices. The row's body already says what: a tool row leads with the tool | A refused call says so in its body, since the column no longer does |
| The same, once he had seen the column built | **Reversed: a card per speaker.** *"instead of a column that is showing who, and it just repeats pretty much the same thing on every row … maybe the grouping is broken up."* He drew it: consecutive rows from one speaker are one card, headed DRONE (or Armada, Fleet), and the rows inside read `Read <path>`, `Edit <path>` and the Drone's prose | The per-row speaker is gone; a reader finds who by the card's head |

The order answer is [[2026-09-22-the-rail-is-a-choice]] again: offered two orders
to pick between, he gave the person the control instead.

**Where it landed:** `packages/screens/src/tab-drones.tsx`, the composition
`packages/components/src/compositions/JobDrones/`, and the draft
`packages/screens/src/draft/drone.ts`.

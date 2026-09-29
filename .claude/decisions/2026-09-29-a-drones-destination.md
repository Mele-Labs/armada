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
| How a tool call differs from what the Drone said, where *"the lines all just kind of blend together"* | **A quiet tool block**: consecutive calls become one compact run, smaller mono, muted, behind a left rule and set in; sentences stay full size and bright. Chosen over a tag per tool name | A path a reader is hunting for is dimmer than the sentences around it |
| Where the Drone's thinking sits among its tool calls, once real transcripts showed a block per call | **Folded into the block**: a thinking run is a dim line inside it, still openable, and only the Drone's sentences break a block. Chosen over one total per block, and over leaving it | A block holds two kinds of line |
| What an opened thinking run shows, where every row read the harness's raw `system/thinking_tokens`. Asked: *"Is there not more information supplied with this?"* — there is: an estimated token count Fleet drops, and the reasoning text the decoder never carries | **Words and a token count**: rows read *Thinking ~400 tokens* or *Reasoned*, the closed line totals them. With a note: *"Maybe when I hover a tooltip shows the reasoning text if this is possible."* Mocked on the draft to be judged by looking | Fleet owes keeping the count. The hover, if kept, reverses `docs/scope.md`'s line that Armada does not carry the model's reasoning, costs storing long text per turn, and is blank where the vendor withholds it |
| Having seen the hover and the opened rows | **Reasoning dropped, for now; the run is one row.** *"lets drop the reasoning for now … instead of multiple thinking rows with the token count for each can we just aggregate it into one row?"* A thinking run reads *Thinking ~1,200 tokens* (or *Working* while live) and does not open | Which call the Drone was thinking about stays visible by position; what it thought and how the count split do not. `docs/scope.md` stands |
| The filter reading *All 6* over six drawn rows, against the standing rule on aggregate counts | **The number goes** from the trigger; the menu's counts stay, since those rows are not drawn | Nothing |

The order answer is [[2026-09-22-the-rail-is-a-choice]] again: offered two orders
to pick between, he gave the person the control instead.

**Where it landed:** `packages/screens/src/tab-drones.tsx`, the composition
`packages/components/src/compositions/JobDrones/`, and the draft
`packages/screens/src/draft/drone.ts`.

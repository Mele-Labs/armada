# A waiting item's steps

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`. **Additive only**: one optional field on each of two DTOs. No operation, event or route.

| Where | Carries | Notes |
| --- | --- | --- |
| `WaitingInput.steps?` (`waiting_for`, Fleet) | The procedure the person follows, in order, as short lines | Fleet trims each and drops a blank one. An empty list is left off, never sent as `null` |
| `WaitingItem.steps?` (Bridge) | The same lines, as the agent's item was kept | Only an agent's item carries them. Bridge opens a press on one as a panel beside the thread, and Done or Send answers through `answer_waiting` `text`: Done is the word `Done`, with the person's corrections as lines after it |

**Skew.** The ID moves, so a Bridge and a Fleet that differ are refused and the lifeboat serves. Bridge's half is `packages/protocol/src/sessions.ts`, written by hand. **Kept.** `session_waiting.steps`, a JSON array column (store migration `20261015T1200Z-session_waiting.steps`), additive with a default of `[]`, so an item kept before it reads as no steps. **The event stream.** `session.row` carries the lines with the item, a few short lines at most, so this is neither better nor worse for the unbounded sink.

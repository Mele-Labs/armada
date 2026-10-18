# A handed-off Job

**Part of:** [Job](../session.md).

**A Job that has grown too large asks the owner whether to carry on in a new Job, and Fleet starts nothing until he says yes.** The ask is the Job's own question card: one question, header `Context`, two options labelled exactly `Carry on in a new Job` and `Stay here` (`ipc::HANDOFF_YES`, `ipc::HANDOFF_STAY`).
The *Context usage* tell is the only prompt for it, and it tells the Job to ask this way and no other.
Fleet reads the owner's own press at the permission door; it does not read the Job's word that he agreed.

| Step | What happens |
|---|---|
| 1. Ask | The card is raised and answered as any question is. `Stay here` is an ordinary answer and nothing else happens |
| 2. Start | On `Carry on in a new Job` the Job's call is held, a `handoff_starting` row stands on its ledger, and the new Job is started as a fork at the old one's latest message of its own, with the same title, model, effort and mode |
| 3. Hand | `release_session_holds` runs with the new Job as `to_session_id` and no `rows`: every standing slot, branch and pull request row moves, each slot's lease with it. A moved pull request is registered to the new Job in the central registry |
| 4. Link | `replaces` on the new Job and `replaced_by` on the old one, both `spent` |
| 5. End | The old Job is closed as `move_session` closes one, so its process ends. Its held call is never answered |

**The new Job starts idle.** Nothing is sent to it; the owner writes first.
Every process of it has a replace note in its system prompt in place of the fork note: the slots, branches and pull requests came to it, and the old Job's subagents, background tasks and shells did not.

**A failure moves nothing.** If the new Job does not start, or the hand is refused, the old Job's thread, ledger and process are untouched, a standing `handoff_failed` row (`detail.reason`) is written to its ledger, and the held call is answered with a refusal that tells the agent it is still the Job.
No second card is raised.
The owner's next message to the Job settles the row.
Bridge draws it as the band "Failed to hand off to new job".

**What the hand does not move.** Artifacts, subagents and background commands stay on the old Job's ledger and are reached through the earlier Job; a server follows the slot it runs in.
The agent cannot act after the yes: its call is held until the hand is done and then its process ends.

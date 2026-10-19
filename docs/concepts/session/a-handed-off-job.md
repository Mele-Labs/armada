# A handed-off Job

**Part of:** [Job](../session.md).

**A Job that has grown too large asks the owner whether to carry on in a new Job, and Fleet starts nothing until he says yes.** The ask is the Job's own question card: one question, header `Context`, two options labelled exactly `Carry on in a new Job` and `Stay here` (`ipc::HANDOFF_YES`, `ipc::HANDOFF_STAY`).
The *Context usage* tell is the only prompt for it, and it tells the Job to ask this way and no other, and to write its brief (where the work is, what is open, what it holds) in the description of `Carry on in a new Job`.
Fleet reads the owner's own press at the permission door; it does not read the Job's word that he agreed.

| Step | What happens |
|---|---|
| 1. Ask | The card is raised and answered as any question is. `Stay here` is an ordinary answer and nothing else happens |
| 2. Start | On `Carry on in a new Job` a `handoff_starting` row stands on the Job's ledger, its call is answered with a refusal telling the agent the work moved and to stop, and the hand runs on a task of its own. A fresh Job is started with the same title, model, effort and mode. **It is not a fork**: a fork copies the whole thread and would start as heavy as the Job it replaces. Fleet reads the brief off the question it holds, and composes the same handoff a Job started from another Job gets (ids, slots, branches, pull requests, the last of the thread) |
| 3. Link | `replaces` on the new Job, carrying the handoff and the brief as `detail.context`, and `replaced_by` on the old one, both `spent`. Both are published, so Bridge folds the old Job behind the new one at once |
| 4. Hand | `release_session_holds` runs with the new Job as `to_session_id` and no `rows`: every standing slot, branch and pull request row moves, each slot's lease with it. A moved pull request is registered to the new Job in the central registry |
| 5. End | The old Job is closed as `move_session` closes one, so its process ends |
| 6. Carry on | Fleet tells the new Job to carry on from what it was left, which starts its first process |

**The hand never runs on the old Job's call.** Ending the old Job ends the process the call came from, and the connection with it, and the server drops whatever runs on that call.
A hand run there died partway, after the new Job was made and before it was linked, leaving two Jobs and a `handoff_starting` that never settled (19 Oct 2026).

**The new Job carries on by itself.** Its first turn is Fleet's word to carry on; the owner need not write.
Every process of it has a replace note in its system prompt, ending in the handoff and the brief: it is a new Job that continues the old one's work, the slots, branches and pull requests came to it, and nothing else the old Job started did.
The old Job's thread stays reachable behind the new one through `replaces` and `replaced_by`.
A card with no brief still hands on, with the handoff alone.

**A failure leaves one live Job, the old one as it was.** If the new Job does not start, the old Job is untouched.
If a step fails after the new Job is up, the ledger is handed back to the old Job, the links are given back and the new Job is closed, so two Jobs are never live.
Either way a standing `handoff_failed` row is written to the old Job's ledger with `detail.reason`, a bare fact in lowercase, and `detail.brief`.
No second card is raised.
The owner's next message to the Job settles the row.
Bridge draws it as the band "Failed to hand off to new job", with Retry.

**Retry** (`retry_handoff`) runs the same steps from the row, with the brief it kept; the owner's press is the consent and no card is raised.
On success the row is given back; on failure a fresh row replaces it.
The closed Job a rolled-back attempt leaves is not removed.

**A Fleet that stops mid-hand settles it at its next start.** A `handoff_starting` row still standing at boot is a hand that died.
Where the old Job had ended, the new one is the live one and is told to carry on.
Otherwise the hand is rolled back as a failed step is, and `handoff_failed` stands with the reason `fleet stopped during the hand off`.
The mark names the new Job (`detail.new`); a mark an older Fleet wrote without it is matched to the live Job of the same repository and title started within a minute of it.

**What the hand does not move.** Artifacts, subagents and background commands stay on the old Job's ledger and are reached through the earlier Job; a server follows the slot it runs in.
After the yes the agent is told to stop, and its process ends when the old Job does.

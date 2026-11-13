# A handed-off Job

**Part of:** [Job](../session.md).

**A Job that has grown too large is put to the owner on one card, and Fleet starts nothing until he presses Carry on.**
The *Context usage* tell is the only prompt for it.
It tells the Job its figure and the limit, to write only a brief (where the work is, what is open, what it holds), to send it with `handoff_brief`, and to ask nothing, start nothing and end its turn.
The tell keeps `handoff_asked`, `standing`, on the Job's ledger, and `handoff_brief` is refused without it, so a Job sends one brief a telling.
The brief raises Fleet's own card, a `handoff` row in the thread with the Job's context, the limit, the brief and three figures, which is also `handoff:<row>` in his Needs you.
The card is Fleet's, read off the row it holds; Fleet never reads the Job's word that he agreed.
**The one place it reads the Job's prose for this tell is a refusal.** A Job that turns the tell down in words, with no brief, would be left alone, so Fleet reads the first reply after the tell with the language model's `refused` head and, on a refusal, raises `refused:<tell sent at>` in Needs you with the sentence that says no.
He decides whether it stands; dismissing it tells the Job nothing.
**It is built and not in force** (`session_host/tell_refusal.rs`): out of fold it caught none of the refusals of the tells it was for, and `docs/spikes/tell-refusal-reader.md` says what would move it.
Until then a Job that declines in prose is not called to him by this tell; the tell is recorded (`session_tells`) either way.
The setting *Large context* can stand in for the card, or silence the tell (below).

**One press answers the card, and the Job is told nothing.** The card asks `Carry on in a new Job?` with six options in one list.
No second question follows any of them, and no message goes to the agent, so no prose follows his press.
Only he answers it: an answer the Operator or Autopilot gives in his place is refused.

| Option | What Fleet does |
|---|---|
| `Carry on in a new Job` | Runs the hand below with the card's brief |
| `Stay here` | Nothing. His Dismiss is the same |
| `Don't ask again` | Keeps `handoff_quiet`, `spent`. The Context rule never tells the Job again |
| `Remind at 400k`, `Remind at 450k`, `Remind at 500k` | Keeps `handoff_remind_at`, `standing`, its target the tokens. The rule tells the Job nothing until its context passes the figure, then once, and the row is `spent`. From there the rule is as it always is |

The figures are the next 100k step above the Job's context, then 50k and 100k past it: a Job at 312k is offered 400k, 450k and 500k, one at 450k is offered 500k, 550k and 600k.
A second figure gives back the first.
The row is replaced by id with `answer` set to what he pressed, so the thread says it and the item leaves his list.

| Step | What happens |
|---|---|
| 1. Start | On `Carry on in a new Job` a `handoff_starting` row stands on the Job's ledger and the hand runs on a task of its own. A fresh Job is started with the same title, model, effort and mode. **It is not a fork**: a fork copies the whole thread and would start as heavy as the Job it replaces. Fleet takes the brief off the card's row, and composes the same handoff a Job started from another Job gets (ids, slots, branches, pull requests, the last of the thread) |
| 2. Link | `replaces` on the new Job, carrying the handoff and the brief as `detail.context`, and `replaced_by` on the old one, both `spent`. Both are published, so Bridge folds the old Job behind the new one at once |
| 3. Hand | `release_session_holds` runs with the new Job as `to_session_id` and no `rows`: every standing slot, branch and pull request row moves, each slot's lease with it. A moved pull request is registered to the new Job in the central registry |
| 4. End | The old Job is closed as `move_session` closes one, so its process ends |
| 5. Carry on | Fleet tells the new Job to carry on from what it was left, which starts its first process |

**Large context.** `plan.contextHandoff` in settings.json, drawn under Agents, *Plan*, with the tooltip "Context limit". Live.

| Value | When the Context rule trips |
|---|---|
| `ask` (shipped) | The Job is told to send its brief for the card |
| `carry on automatically` | No card. Fleet tells the Job to reply with its brief only, waits up to three minutes for the turn that answers, and runs the hand above with what it said as the brief; with nothing said, with the handoff alone. `handoff_starting` stands from the trip. The setting is the owner's standing consent, and nothing else starts a successor |
| `never` | No Job is told |

A Job's own answer wins over the setting: a quiet Job is never told or handed on, and a Job with a figure is told, never handed on, once it passes it.

**The hand never runs on a call.** Ending the old Job ends its process, and the connection with it, and the server drops whatever runs on a call that process made.
A hand run there died partway, after the new Job was made and before it was linked, leaving two Jobs and a `handoff_starting` that never settled (19 Oct 2026).

**To the owner a hand is one Job.** One row, one title, one thread, and one name in what a parent is told (6 Nov 2026: after a hand he saw two Jobs in Pocket and asked for them to read as the same one).
The two keep their own ids underneath, because a Job's id is its agent's conversation, which the next process resumes, and the hooks, the thread and every ledger table key on it.
What makes them one is that every place folds the chain:

| Where | How the old Job folds into the new one |
|---|---|
| Bridge's lists, Pocket's Home | An ended Job with `replaced_by` is never a row of its own |
| A Job's thread, in Bridge and in Pocket | The old Job's turns are read above the new one's, over a rule. Opening the old Job opens the new one |
| A parent's started Jobs | Once the old Job has ended, the parent's `started_to` on it is given back, so its lists, `list_started_sessions` and Pocket's links name the new Job alone |
| A parent's thread | The row that tells the parent of the hand carries `replaces` and is not drawn: the row that started the old Job stands for both, and opens the new one. Its agent is still told the new Job's id |
| A wait, or a change to one, naming the old Job | It names the last Job of the chain (`job_now`), so a wait made after the hand is not refused as closed |
| What the person chose for the Job | Auto land, the workflow and how it was started carry to the new Job (`settings_carried`) |

**The Operator reads a red pull request only off a reading taken after the turn ended.**
A new Job usually pushes a fix in its first turn, and the reading a minute old still says failed.
On 6 Nov 2026 that started a "Rest of" Job 18 seconds after the fix was pushed, and that second Job was what he saw. A stall is now only raised to him (*Operator*).

**The new Job carries on by itself.** Its first turn is Fleet's word to carry on; the owner need not write.
Every process of it has a replace note in its system prompt, ending in the handoff and the brief: it is a new Job that continues the old one's work, the slots, branches, pull requests and pages came to it, and nothing else the old Job started did.
The old Job's thread stays reachable behind the new one through `replaces` and `replaced_by`.
A hand the setting runs with no brief carries the handoff alone.

**A Job another Job started passes its link on.** Once the old Job has ended, the new one gets `started_by` naming the same parent and the parent gets `started_to` naming the new one, each with the old link's `detail.group`.
The parent is told the old Job was handed on and to which Job, in place of hearing that it ended, and follows the new Job's merges and its end from then on (*Linked Jobs*).
The link waits for the end because a hand undone before then closes the new Job, which the parent would hear as an end.
A Fleet that stopped after the old Job ended writes the link at its next start.

**A wait on the old Job moves to the new one**, once the old Job has ended and the new one has its first word.
The new Job is told of each wait as the old one was, and an end by a hand starts nothing (*A Job that waits on another*).
Seen 30 Oct 2026: a parent told only that its child ended started a duplicate, and watched the successor's pull request itself (armada-3903).

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

**What the hand copies.** Each [document](../document.md#a-job-that-takes-anothers-place) comes to the new Job with its revisions and his answer, and the old Job keeps its own.

**What the hand does not move.** Other artifacts, subagents and background commands stay on the old Job's ledger and are reached through the earlier Job; a server follows the slot it runs in.
The old Job's agent is told nothing of the hand: its process ends when the old Job does.

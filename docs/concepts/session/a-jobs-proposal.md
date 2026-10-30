# A Job's proposal

**Part of:** [Job](../session.md).

**A Job is read once**, by the model call the proposer makes (`job-proposer.md`, *How a Job's request is read*), and what the call settles is the Job's own.
`StartSession.request` carries the words.
The press never waits: the start answers at once with the row, `proposal.reading` true and no process, and **the process starts when the answer lands**, on the title, workflow, model and effort it settled.
A first message sent meanwhile is written to the thread at once and its turn is held, in the order the messages came.
A failed, late or unreadable call starts the Job on the machine's defaults.
Fleet's local language model has a workflow reader (`session_host/workflow_pick.rs`), not in force: on 38 requests no pick agreed with Claude throughout, and the call also settles the title, model and effort, so a local pick would skip none (`docs/spikes/workflow-pick-by-model.md`).

| On the record | Meaning |
|---|---|
| `workflow` | The one workflow the Job follows, or absent: **a custom Job**. Never a default |
| `proposal.reading` | The call is still out |
| `proposal.picked` | The fields the proposer filled (`title`, `workflow`, `model`, `effort`), each dropped when the person sets that field |

**A title follows his renames.** A Job he renames in Bridge is kept against the name it had, and the next Job's call is shown his last 12 as examples (`job-proposer.md`, *Names he gave*).

**Which starts are read.** Every Job is read once, at the first message that is its brief.
A start with a `request` (the dispatch sheet) is read at the start.
Any other start is read at the first message it is sent or given, and that turn is held behind the reading exactly as a dispatch's is.

| Start | Read | When |
|---|---|---|
| Dispatch sheet (`request`) | Yes | At the start. The Job is held, and the reading sends nothing |
| Annotations, Lessons, Hand to Job, any Bridge start with no `request` | Yes | At the first message the Job is given: kept on a held Job, sent on any other |
| `start_another_session` (an agent's start, with a plan group or a wait) | Yes | At creation, from the agent's brief, so the proposal is in the dispatcher before he presses Start. Start does not read it again |
| A fork (`fork_this_session`, a fork in `start_session`, red main's fork) | No | It carries its source's conversation, model and settings. A fork an agent titled is asked for its title alone when Start sends it (`job-proposer.md`, *How a Job's request is read*) |
| Helm, and Setup's Job | No | Helm is a conversation with fixed values, not a brief; Setup has no Manifest to read against |
| Autopilot, Rescue's continuation | No | Fleet's own dispatch: its brief is fixed, and Autopilot has picked the workflow |
| Fleet's words to a Job (`told_by_fleet`) | No | A notice is not a request |

**Only the first.** A Job is read when it has not run, holds no proposal and has not been asked; the mark is in memory, so a Fleet that restarts before the first turn may read once more.
What the Job already carries (a title, a model, an effort, a workflow) stands over the answer, as it does for a dispatch.
No proposer, or a failed one, starts the Job as it is.

**What a person sets wins, and an answer never overwrites it.** A field given on the start, or changed in a panel while the call was out, stays as set.

**A change reaches the agent, because it cannot read the panel.**

| Changed | What the agent is told | When | Timeline |
|---|---|---|---|
| The workflow (`set_session_workflow`) | A message from Fleet naming the workflow and its step labels, or saying it is now a custom Job; none where the agent made the change itself | At once: mid-turn at the next tool call, and an idle Job wakes. A Job that has not run is told with its first turn | `workflow: <old> → <new>` |
| The model or effort (`tune_session`) | A line ahead of the turn that starts on the new values: `from this turn your model is X, not Y` | On the next turn, with no wake | `model: <old> → <new>`, `effort: <old> → <new>`, entered when that turn starts |

**An unchanged model tells nothing.** Fleet keeps the model and effort each turn ran on, and the first turn only sets the baseline, so a restart does not tell again.
The three events are `session.workflow`, `session.model` and `session.effort` on the timeline, where committed and pushed rows come from.

**Held turns are in memory.** A Fleet that restarts while a Job is still being read loses them; the message row stays on the thread.
A [held Job](the-start-card.md) is the exception: its hold and its first message are kept, and its other turns are read back off the thread.

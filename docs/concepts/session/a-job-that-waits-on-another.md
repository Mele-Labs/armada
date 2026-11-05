# A Job that waits on another

**Part of:** [Job](../session.md).

A Job can be dispatched to **start when another Job reaches a point**, with that Job's handoff first. The owner approved the design on 16 Oct 2026 (walk `sessionAfterAnother`).

| | |
|---|---|
| What waits | A Job with no message sent: it holds its brief, takes **no slot and runs no process**. The wait is a fact on its record (`waits`), kept in `session_waits` and absent once it has started. It stands in Work under the Job it waits on, in the state `after` |
| The condition | `ends` (the default), `group_done` (a group of the plan of the Job waited on), `task_done` (one task), `pr_pushed`, `pr_lands`. Fleet **reads it from what it observes**: the Job's state, its plan's progress and its pull request events, never from an agent's call. The first pull request seen is the one `pr_pushed` and `pr_lands` mean, and it is kept |
| Where it starts from | `main` (the default) or `branch`, the branch of that pull request. Dispatch defaults `pr_pushed` to `branch` |
| The start | Once, when the condition is met: the Job is held (*A held Job*), its first message the **handoff of the Job waited on** (the builder `hand_to_session` uses) followed by the brief. A restart cannot start it twice |
| Who sets it | Dispatch, from a brief that tags a Job with `@` and says when, read by the proposer (`read_session_wait`). A tag with no condition in the words is **plain**: the new Job does not wait, is held at once, and the tag goes with its first message; or an agent's `start_another_session` with `after`. A reading that cannot be had is the defaults: it ends, from `main`, asking nothing |
| The Job waited on | Is told what the waiting one needs, once, and woken if idle, as any news. A Job waited on that itself still waits is told once it has started, since a word would start it ahead of what it waits for. It may change the condition or the base with `change_waiting`, which is allowed only for a Job that waits on it; the owner may change any, also where the Job waited on has ended, whose thread then keeps the row with nobody told. Each change writes a `waiter` row in both threads. The owner can start a waiting Job at once (`start_waiting_now`) |
| The Job that started it, and its wait | May change the wait of its own child with `command_started_jobs`, **whichever Job the child waits on and whether or not that Job has ended**, or drop the wait so the child stays held for the person (*A Job that started Jobs*). The Job waited on is told, naming the starter, unless it is the starter. `change_waiting` stays the Job waited on's and the person's |
| The Job that started it | Is not held by it. A Job that waits has not begun, so it refuses nothing of its parent's, and a wait on the parent's own `ends` can be met: the parent finishes, is ended, and it starts. It holds a parent still live once it runs ([the rail](the-rail.md)). The owner, 3 Nov 2026 |
| Refused before anything is made | The Job waited on is unknown, is the Job itself, has ended, or has no such group or task in its plan. A change to a wait that stands is refused for the same reasons but one: the Job waited on may have ended |
| When the Job waited on is handed on | The wait moves to the Job that takes its place, down a whole chain of hands, and that Job is told as the old one was. A pull request the wait named stays its own, since the row moved with its number and branch. An end by a hand is not an end of the work, so `ends` waits for the last Job to end. A wait on `group_done` or `task_done` becomes `ends`, with a `waiter` row in both threads and the new Job told which part it was: a new Job holds no plan, and a plan of its own numbers from `g1` again, so an id could name another group. Its agent can point the wait at a part of its own plan with `change_waiting`. A Fleet that stopped before the wait moved moves it at its next start. Seen 3 Nov 2026: a Job waited on a pull request its Job's successor merged, and never started |
| When the wait can no longer be met | The Job waited on ended with no successor and the condition unmet, so nothing will meet it: a closed Job gives its pull requests back and hears no merge. **The waiting Job leaves its wait held on its start card**, its first message saying why, such as `The agent ended with no pull request`, even where he approved it to start alone. What he approved has not happened, so it is his to start or close |

> **Rule.** A waiting Job starts on what Fleet observed, and what it waits for can be changed by the Job it waits on.
> Why: the Job doing the work knows when the point is reached and what is needed of it, and a hand that cannot move the condition leaves a Job waiting on a point that will not come.

When the condition is met the Job is held with that message in its composer, whoever dispatched it, so nothing runs until he presses Start. While it waits it is not held and holds no first message.

Bridge draws it from the wire, with no state of its own:

| Where | What |
|---|---|
| Dispatch | The sentence over the request, `Starts when [Session] [condition] · from [base] · needs [..]`, filled by the proposer's reading when it arrives, and a send waits for the answer. A tag that names no condition draws no sentence. `⌥A` opens Dispatch with `@` typed |
| Work | The waiting row's state line says when it starts. `docs/concepts/cockpit.md` |
| The waiting Job | Its brief, who it waits on, the condition (changeable), what it needs and where it starts from, and **Start now** |
| The threads | A `waiter` row: in the thread of the Job waited on when it is dispatched, changed or started; in the waiting Job's own thread when its condition is changed. A change Fleet made at a handover names no `by` |

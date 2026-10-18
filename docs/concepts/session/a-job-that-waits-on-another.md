# A Job that waits on another

**Part of:** [Job](../session.md).

A Job can be dispatched to **start when another Job reaches a point**, with that Job's handoff first. The owner approved the design on 16 Oct 2026 (walk `sessionAfterAnother`).

| | |
|---|---|
| What waits | A Job with no message sent: it holds its brief, takes **no slot and runs no process**. The wait is a fact on its record (`waits`), kept in `session_waits` and absent once it has started. It stands in Work under the Job it waits on, in the state `after` |
| The condition | `ends` (the default), `group_done` (a group of the plan of the Job waited on), `task_done` (one task), `pr_pushed`, `pr_lands`. Fleet **reads it from what it observes**: the Job's state, its plan's progress and its pull request events, never from an agent's call. The first pull request seen is the one `pr_pushed` and `pr_lands` mean, and it is kept |
| Where it starts from | `main` (the default) or `branch`, the branch of that pull request. Dispatch defaults `pr_pushed` to `branch` |
| The start | Once, when the condition is met: the first message is the **handoff of the Job waited on** (the builder `hand_to_session` uses) followed by the brief. A restart cannot start it twice |
| Who sets it | Dispatch, from a brief that tags a Job with `@` and the words after it, read by the proposer (`read_session_wait`); or an agent's `start_another_session` with `after`. A reading that cannot be had is the defaults: it ends, from `main`, asking nothing |
| The Job waited on | Is told what the waiting one needs, once, and woken if idle, as any news. It may change the condition or the base with `change_waiting`, which is allowed only for a Job that waits on it; the owner may change any. Each change writes a `waiter` row in both threads. The owner can start a waiting Job at once (`start_waiting_now`) |
| Refused before anything is made | The Job waited on is unknown, is the Job itself, has ended, or has no such group or task in its plan |

> **Rule.** A waiting Job starts on what Fleet observed, and what it waits for can be changed by the Job it waits on.
> Why: the Job doing the work knows when the point is reached and what is needed of it, and a hand that cannot move the condition leaves a Job waiting on a point that will not come.

The owner's Job sends that message when the condition is met. An agent's waiting Job holds it for the owner instead, as any start an agent makes, so nothing runs until he sends it.

Bridge draws it from the wire, with no state of its own:

| Where | What |
|---|---|
| Dispatch | The sentence over the request, `Starts when [Session] [condition] · from [base] · needs [..]`, filled by the proposer's reading: the defaults at once, its answer when it arrives, and a send waits for the answer. `⌥A` opens Dispatch with `@` typed |
| Work | The waiting row's state line says when it starts. `docs/concepts/cockpit.md` |
| The waiting Job | Its brief, who it waits on, the condition (changeable), what it needs and where it starts from, and **Start now** |
| The threads | A `waiter` row: in the thread of the Job waited on when it is dispatched, changed or started; in the waiting Job's own thread when its condition is changed |

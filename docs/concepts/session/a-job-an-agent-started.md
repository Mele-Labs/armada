# A Job an agent started

**Part of:** [Job](../session.md).

A Job's agent may start a Job, or fork its own conversation into one, through two tools on its door (23.80). The owner approved it on 12 Oct 2026 on the condition that **the new Job waits for him**.

| | |
|---|---|
| `start_another_session` | `manifest_id`, `title`, `message`. A new Job in any repository Fleet serves |
| `fork_this_session` | `title`, `message`, optional `at`. A copy of the caller's own conversation, to the row `at` or to its latest message of its own that the transcript holds. Allowed while the caller is live, since it is a fork at a message |
| The caller | Placed by the connection of its hosted process, as `show_window` places it. **No call names a Job**, so an agent cannot fork another's conversation, and a call from no hosted Job is refused 422 `fleet.agent_start_unplaced` |
| Not offered | Tune, close, send and every other hosted-session command stay `agent_access` No |

> **Rule.** An agent's Job is created and listed at once, held, and nothing runs until the owner presses Start.
> Why: a Job that started itself is a model spending tokens in a repository nobody chose to open. Holding the message makes the start a proposal he reads and may edit.

**The Job opens held** (*A held Job*): its title in the header, an empty thread under a *Started by* line naming the Job whose agent started it, and the agent's `message` in the composer, where he may edit it.
Workflow and Delivery sit in the strip above the composer, filled and never blank.
Send reads Start, and Start sends the box as he left it.

**The first message is kept off the thread** in `held_first_turns`, and read back on every read as a waiting item: id `unsent:<session id>`, text *First message*, source `first_message`, act `message` on the Job.
It has its own source and act so the Cockpit draws no reply box for it and a press opens the Job.
No `unsent` attachment stands for it.
`answer_waiting` refuses the item, since Start is the answer.
Start settles the item, and ending the Job discards both.
While sleep is on, the item is left on the list and not decided for him.

Nothing runs before Start, and no step prompt goes out either: the first turn carries the workflow line, the step's instructions and the message (*The rail*).

**The caller waits on it too**, in its own Needs you, while the Job is live and held: id `start:<child id>`, text *Press Start on "<child title>"*, source `held_child`, act `message` on the child, so a press opens it.
Fleet reads it off the child on every read of the caller, so Start or ending the child takes it off, and both publish the caller again.
Bridge draws Start on that item and on the child's Started row in the caller's thread, at the row's end before its chevron, so he starts it from either without opening it. The two are one press, and both read *Starting…* while the start waits.
`answer_waiting` and `dismiss_waiting` refuse it, since Start is the answer.
An item of the caller's agent whose act is `message` on a held child is left off, as Fleet's already says it, and the tool tells the agent to list none.

**The brief is read at creation**, so the proposal (title kept as the agent gave it, workflow, model, effort) is on the new Job before he presses Start, and Start goes out on it.
A fork is not read (*A Job's proposal*).

**Start pressed while the brief is still read is queued.** The Job stays held, `start.queued` says so and the caller's item stands.
When the reading ends the Job begins with the press's edit and landing, or, where the press named none, the landing the card then picks (`start.picked`: the one the first message names, then his usual, then Auto land).
A second press meanwhile is refused as settled already.
The queue is in memory, as the reading is: a Fleet that restarts mid-reading loses both, the Job is still held and not queued, and he presses Start again.

**The caller's agent is told when it starts**: once, with a `linked` row `started` in the caller's thread and a message that wakes it, `Linked Job "<title>" (<id>) started.` and the plan group after it where there is one (*Linked Jobs*).

**Both ledgers record it.** A start writes `started_by` on the new Job and `started_to` on the caller; a fork writes the `forked_from` and `forked_to` of *A forked job*, with `at` and `cut`.
The caller's thread gets a `started` row naming the Job and whether it is a fork, which Bridge draws as the press that opens it.

**Nothing bounds how many an agent starts.** Each is held, so none spends anything, but a loop would fill the list. A limit waits for one being seen.

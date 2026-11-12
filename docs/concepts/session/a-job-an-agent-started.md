# A Job an agent started

**Part of:** [Job](../session.md).

A Job's agent may start a Job, or fork its own conversation into one, through two tools on its door (23.80). The owner approved it on 12 Oct 2026 on the condition that **the new Job waits for him**.

| | |
|---|---|
| `start_another_session` | `manifest_id`, `title`, `message`. A new Job in any repository Fleet serves. Or `jobs` alone, a list of those, for several in one call: each starts as its own call would, in order, and the answer gives each one's id or refusal in that order |
| `fork_this_session` | `title`, `message`, optional `at`. A copy of the caller's own conversation, to the row `at` or to its latest message of its own that the transcript holds. Allowed while the caller is live, since it is a fork at a message |
| The caller | Placed by the connection of its hosted process, as `show_window` places it. **No call names a Job**, so an agent cannot fork another's conversation, and a call from no hosted Job is refused 422 `fleet.agent_start_unplaced` |
| Not offered | Tune, close, send and every other hosted-session command stay `agent_access` No. What a Job may do to the Jobs it started is its own pair of acts (*A Job that started Jobs*) |

> **Rule.** A Job an agent starts follows one word per project, `agent_starts`: `at_once` (the default) begins it, and `ask_me` holds it on its start card until the owner presses Start.
> Why: he asked for both. A Job that starts itself is a model spending tokens in a repository nobody chose to open, and holding the message makes the start a proposal he reads and may edit; for the projects where he would rather not press Start every time, it begins.
> A Job he starts himself is never held by this word, and neither is a quick terminal or a fork.

**`agent_starts` is a value** (*Values*): Kit's, a project's or a repository's, the nearest winning.
Fleet reads it live for the calling Job's project and the new Job's repository, so a Job that started before the word changed is not held by a later edit.
A project that does not read or does not hold the repository falls back to none, and any fault reads as `at_once`: a fault never holds his work.
**It also covers `start_job` in a lifecycle's rules and the Jobs the Operator starts** (*The Operator*).

**`at_once` begins the Job with the brief as its first turn from the calling Job**, as another Job's word reads.
The brief is still read for the proposal, and the caller is told when it starts, as the next section says.
The answer's `held` is `false`.
Where the brief cannot be sent, the Job is held on its start card with the brief in the box, and `held` is `true`.
A Job that waits on another (`after`) is held once its wait is met, whatever the word says.

The sections below describe the Job as it opens held, which is `ask_me`.

**The Job opens held** (*A held Job*): its title in the header, an empty thread under a *Started by* line naming the Job whose agent started it, and the agent's `message` in the composer, where he may edit it.
Workflow and Delivery sit in the strip above the composer, filled and never blank.
Send reads Start, and Start sends the box as he left it.

**The first message is kept off the thread** in `held_first_turns`, and read back on every read as a waiting item: id `unsent:<session id>`, text *First message*, source `first_message`, act `message` on the Job.
It has its own source and act so the Cockpit draws no reply box for it and a press opens the Job.
No `unsent` attachment stands for it.
`answer_waiting` refuses the item, since Start is the answer.
Start settles the item, and ending the Job discards both.
Autopilot never answers it: Start is the person's press.

Nothing runs before Start, and no step prompt goes out either: the first turn carries the workflow line, the step's instructions and the message (*The rail*).

**The caller waits on it too**, in its own Needs you, while the Job is live and held: id `start:<child id>`, text *Press Start on "<child title>"*, source `held_child`, act `message` on the child, so a press opens it.
Fleet reads it off the child on every read of the caller, so Start or ending the child takes it off, and both publish the caller again.
Bridge draws Start on that item and on the child's Started row in the caller's thread, at the row's end before its chevron, so he starts it from either without opening it.
The two are one press, and both read *Starting…* while the start waits.
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

**The caller reads and commands what it started** with `list_started_jobs` and `command_started_jobs`: where each stands, and withdraw, end, pause, resume, change and drop (*A Job that started Jobs*).

**Nothing bounds how many an agent starts.** Each is held, so none spends anything, but a loop would fill the list. A limit waits for one being seen.

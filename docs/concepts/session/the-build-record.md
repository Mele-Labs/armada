# The build record

**Part of:** [Job](../session.md).

**`SessionRecord.build` says how far a Job has got, read from what Fleet observed.** It asks the model for nothing: there is no tool for the agent to call and no line in any prompt.
(That was the rule of 14 Oct 2026. The owner, 15 Oct 2026, wanted a plan he approves before the work, so a Job now has `record_plan`; *The plan* has it. The ring still asks nothing: it is read from the same observations.) The agent states a plan with the task tools it already uses, the mod hears those calls, and Fleet checks the plan against the edits it saw.
Bridge draws the record as a ring under the Job's drone (the decision of 14 Oct 2026, *A Job builds a ring*).
The field is absent for a Job that has done nothing Fleet observed.

**What the mod tells.** `TodoWrite`, `TaskCreate` and `TaskUpdate` become one fact, `tasks`, holding the whole list after the call.
`ExitPlanMode` becomes `plan_left`.
A successful `git commit` and `git push` become `committed` and `pushed`, read from what git printed and not from the command alone, so a commit with nothing staged, a push with nothing to send, a rejected push and a `--dry-run` tell nothing.
**A subagent's `TaskCreate` and `TaskUpdate` are the Job's list too** (the owner, 15 Oct 2026: *"An entire session contributes to the workload for that session from a scope/plan to implementation"*).
The task tools are shared: a subagent's `TaskCreate` takes the next id of the parent's list and its `TaskList` shows the parent's tasks (measured on claude 2.1.300, headless with the task tools on), so the mod hears them into the one list and marks each task with the `agentId` of the subagent that made it.
Whoever updates a task, it stays its maker's.
**A new process starts from the harness's list, not from nothing** (mod 0.3.27): a hosted Job's process is replaced when Fleet restarts or moves it into a slot, and a terminal Job can be resumed, and either leaves the mod with no list in memory, so a `TaskUpdate` named a task nothing there had made and was never told (Job 7c5094db, 16 Oct 2026: nothing after its process was replaced was heard).
At its first task tool the mod reads `~/.claude/tasks/<list>/*.json`, the list being the job's id or `CLAUDE_CODE_TASK_LIST_ID`, takes it as the list, and applies the call on top, so a rename of a subject is heard as well as a status.
The harness's files are the harness's own record of what its tools did, which is why they are read rather than the change being trusted to arrive whole.
A store that cannot be read tells nothing, and is tried again at the next task tool.
A subagent's `TodoWrite` is a list of its own (the older generation) and is not told.
**Its reads, edits, commands, commits and pushes are the Job's work**: in a terminal Job the mod's hooks fire inside subagents and report under the parent, and a hosted Job's process stream carries a subagent's tool calls, which Fleet hears without drawing its words.
A hosted Job feeds the same path from its gate (`TodoWrite`, `ExitPlanMode`) and from its process's tool events.
**A hosted Job is started with `CLAUDE_CODE_ENABLE_TODO_TOOLS=1`**: a headless process has no task tools without it, so until 15 Oct 2026 its agent could write no list and its build record never had pieces.

**Subagents on the ring.** Fleet turns a task's `agentId` into the label `workers` uses, `Type: description`, from the Job's `subagent` attachment, and keeps it on the task (`session_build_tasks.made_by`) because the attachment is spent by the time a finished subagent's pieces are last read.
Each piece carries it as `by`.
Bridge draws a running subagent as a worker inside the ring (`workers`), and the pieces whose `by` is its label are the segments it wrote, so the ring fills as subagents tick their own steps.
Piece kinds follow the same rules whoever wrote the task: an edit the Job saw while it was worked is what makes it solid.

**What Fleet keeps.** Two tables (`session_build`, `session_build_tasks`): the instants of the first edit, the last edit, the last tool activity, plan mode left and the list's first write, and the list with the instant each item was first seen completed and whether an edit was seen while it was worked.
Items are matched by id and text, else by text, so a reordered list keeps what was seen of each.
An item that goes back to not completed loses its instant.
Everything else is derived on read.
A walk shown, a commit and a push are timeline entries (`session.walked`, `session.committed`, `session.pushed`), and a pull request's stops are the entries it already has (`pr.opened`, `pr.checks`, `pr.queued`, `pr.merged`); the row is published again when one is written.

**A row with no first edit is filled from the timeline.** When a Job's row is first made, and on each fact heard while the row still has no first edit, Fleet reads that Job's `session.edited`, `session.committed` and `session.pushed` entries in one query, from the later of the Job's start and the last merge of a pull request it held.
The earliest of the three is the first edit, the last `session.edited` entry is the last edit, and the build's start moves back to the earliest.
This covers work heard before this Fleet started or before the row existed, which the timeline keeps for 30 days.
A row that has a first edit is not read again, and no row is made from nothing kept.
An entry is dated by its burst's first call, so a seeded last edit is when the last burst began.

**A commit or a push counts as the first edit** where none was heard, since it proves code changed even when a script made the edits through a shell.
A read or a command does not.
A commit or a push does not start a new build after a merge, as a read does not.

**A build ends where its pull request merged.** The first edit, plan or task list heard after the merge starts another build with no list and no road. A read or a command after the merge does not.

| Field | Read from |
|---|---|
| `phase` | `landed` when the pull request merged. `road` when a stop was reached at or after the first edit, or any stop where no edit, commit or push was ever observed. `plan` when a task list was written or plan mode left and no edit followed. `implement` when edits, a commit or a push were seen. Otherwise `scope` |
| `pieces` | One per task, in list order. `solid`: completed, and an edit was seen between the previous completion (or the list's first write) and this one. `hollow`: completed with no edit seen in that window. `scaffold`: the first task not completed, while an edit has landed since the last completion. `ghost`: every other. With no list there are none: progress is never invented. `working` marks a task the agent's tool has in progress, whatever piece it is |
| `stops` | The road as drawn: the stops reached in the order they were, then the forecast ones not yet reached, each with no `at` |
| `forecast` | The stops the repository's recent landed Jobs usually reached |
| `landed_at` | When the pull request merged, as the forge dated it |
| `quiet_since` | The last read, edit, command or task list. It is as of the last time the row went out, which a call after a minute of quiet does |
| `stalled` | The Job's `standing` is `stuck` |
| `workers` | The subagents still standing, each as `Type: description` |

**The stops.** `walk` is the first walk shown after the first edit.
`pushed` is a push that updated a ref.
`pr` is the pull request opened, which is never later than its first check, queue entry or merge.
`checks` is its first CI state.
`auto_merge` is its first queue state.
Merging is not a stop: it closes the road and is `landed_at`.
The pull request is the newest one the Job holds that is not closed and did not merge before this build began.

**The forecast rule.** For the Job's repository, read the pull requests that merged most recently and that a Job held, up to ten.
For each, take the stops reached between that Job's previous landing and this one, in the order they were reached.
The forecast is the most common whole sequence among them, and a tie goes to the more recent.
A landing with no stop observed takes no part, and a repository with no history has no forecast.
A stop already reached is not drawn faint twice.

**What is not observed, and left absent.** A task made by an earlier process of the Job is read from Claude Code's own list on disk (see *What the mod tells*), so it has no maker: a subagent's task from before a restart reads as the Job's own.
Edits made through a shell command (`sed`, a redirect) are not edits, though the commit that follows them is the first edit if none was heard, and nor are files outside the Job's directory.
A task completed before its edits were filled in from the timeline stays hollow, and a seeded last edit is only as exact as the timeline's bursts.
A Job with no new fact since a Fleet restart keeps the row it had until its next tool call.
A task the agent never marks completed is never solid or hollow.
Auto-merge set through the forge's page, and checks Fleet's merge line read only after the merge, may have no stop.

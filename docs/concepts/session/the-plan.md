# The plan

**Part of:** [Job](../session.md).

Since 16 Oct 2026. The owner, 15 Oct 2026: a Job plans with him first, and what he approves is what gets built.
**Nothing in a plan is the agent's word about progress**: the plan is its statement of the work, and how far it got is Fleet's observation, as for the build record.

**`record_plan`** is a tool on the agent's door.
It carries `groups` in order, each with a `title`, an `agent` (the subagent type that takes it, one word, free text such as `rust-engineer`), `paths` (from the repository's root; an edit under one marks the group worked) and `tasks` (titles), and an optional `sketch` (`title` and `scene`), checked by the code `show_sketch` uses, so a scene that would draw as an empty frame is refused with the part at fault.
A group or task may send back the `id` Fleet gave it.
Fleet refuses a plan with every reason at once and says to call `record_plan` again; nothing is kept from a refused plan.
The agent gets back the version and each id Fleet gave.

**Recording again replaces the plan as a new version.** Fleet keeps the current version and the diff from the one before: a group or task is matched to the old one by its id, then by its text with case and spacing ignored, and what differs is `added`, `renamed`, `moved` (to another group) or `removed`.
An id the old version never held is not an id.
The sketch's scene is a ledger artifact `sketch:<title>`, as `show_sketch` keeps it, and the plan names it.

**A recorded plan is pending until the owner approves it, and pending is one of the Job's calls.** It is a waiting item, id `plan:<version>`, source `plan`, where `ask:` and `perm:` items go.
Nothing blocks the agent's tools meanwhile; the wait is the agent's to keep, and the Job rules say to keep it (`docs/contracts/agent-prompt.md`, 5f).
**The item is raised when it can be answered: only a window he can see or still owes holds it** (`plan_holds`, `planning.rs`).
**A pending plan waits while any of the Job's walks waits for his Approve**, closed window or not, and arrives once he approves them.
An Approve of any take settles the window, as it does the walk item.
A window a newer window of the same page replaced (page or take, compared without the port) holds nothing, nor does a walk he dismissed from the list, whatever their marks.
**A window he answered in words, not with an Approve, before the plan was recorded holds nothing**: the plan is the Job's answer to what he said, and the walk item still waits on its own Approve. Shown again after the recording, the window holds the plan again.
Since 20 Oct 2026, after he asked for a bigger change on a walk, the Job planned it, and the walk he had moved past kept the plan out of his sight.
**An Approve stands until the page is shown again.** Any file the Job writes marks every window it showed as changing (`detail.working`, `readiness.rs`) and changed (`detail.changed`), so those marks hold only the newest window, the one on his screen, and a window he has not approved, which waits to be shown again.
The item begins once nothing holds it: its `since` is the latest of the recording, when each window that counts was shown again or made ready, and his Approves, so it calls even if he viewed the Job since the recording.
Since 19 Oct 2026, after the owner approved every walk he was shown and saw no plan: a take shown again inside another window, a window replaced on screen and a page changed after his Approve each held it, and he approved two dead windows by hand.
"This is so painful."
**The plan on the record names what holds it**: `held_by`, each window's `url` and `why` (`unapproved`, `changing`, `changed`, or `walk` for the walk it names).
`record_plan` takes an optional `walk`, the url given to `show_window`; while a plan names one, the item waits for a message from him that approves that url, or a take of the window that showed it, at or after the newest window row showing it.
A page never shown keeps it held.
Recording again without `walk` clears it.
The receipt says `held` when the recording is not put to him yet, so the agent does not tell him it waits on his approval, and names each window in `held_by` with what settles it.
Sleep mode leaves it for the morning, and `answer_waiting` refuses it: a plan is approved, or sent changes, from its own card.
**`approve_plan`** (`session_id`, `version`) is a Bridge route.
It sets `approved` and sends the agent one message, before it settles, so a failed send leaves the plan to approve again.
A version that is not the current one is refused (409, `fleet.plan_version_stale`), a Job with no plan too (`fleet.plan_none`).
A second press tells the agent nothing.

**His changes are proposals.** **`send_plan_changes`** (`session_id`, `version`, `marks`, `approve`) takes marks against the version he read: rename, remove, add task, add group, move task, change subagent, note.
Fleet checks each against that version (a mark may name a group or task another mark of the same batch adds), words them as **one message** to the agent, with the ids so it can answer by them, and writes one `plan_changes` row to the thread in place of a Fleet message beside it.
**The plan does not change**, whether it is pending or approved; it changes when the agent records it again.
`approve: true` approves the version with them.

**How far it got is observed.** A task is `open`, `working`, `handed_in`, `done` or `failed`.
The agent's task list (the `tasks` fact, from the mod and from the hosted gate, through the one funnel `build_heard`) is matched to plan tasks by text.
**The same words first; otherwise reworded close enough** (the task's words in a row inside the item's or the item's inside the task's, filler such as `the` and `to` left out, or at least 0.6 of the longer line's words shared, two words at least), **and an item says at most one task and a task is said by at most one item**, the best pairs taken first and an item no task wants ignored.
**The agent is told its list is the plan**: the `record_plan` receipt (`task_list`), and the message for an approval (with or without marks), which lists the tasks verbatim, say `TASK_LIST_IS_THE_PLAN`, and the Planning prompt says it before the plan is recorded.
A task the list calls in progress is `working`; one it calls completed is `done` if an edit was seen under its group's paths while it was worked, else `handed_in`: done by the agent's word with no edit seen.
**`handed_in` reads as done** (owner, 17 Oct 2026).
No Checks are involved and nothing moves it on; usually the task had no file edits (reading, running tests, a subagent's edits, or edits outside the group's paths).
Bridge draws it with the done check and a faint mark for no edit seen, tooltip `Done, no edit seen`, and counts it as done in Progress, in a group's state and on the Field, where it is hollow.
`failed` is not set by anything Fleet observes yet.
An edit under a group's paths marks the group worked (`worked_at`); a group that names no paths takes every edit.

**With no plan, the agent's own task list is what Progress shows.** The list is kept whether or not a plan exists (*The build record*; `session_build_tasks`), so a Job that never records a plan still has one.
Bridge lists it in the order the agent made the tasks, each with a state mark: `done` for a completed task, `working` for the one the tool has in progress, `open` for the rest.
The running subagents are listed under it.
The row is published when the list changes, so Progress follows live.
**A recorded plan takes over**: with one, Progress shows the plan's groups and tasks and none of the agent's list, and the list goes on moving the plan's tasks by text as above.
A Job with neither a list nor a subagent at work, and no workflow to follow, stands folded to its band.

**Who touched which file.** `SessionRecord.touches` is a bounded window of the Job's recent file touches for the Field: the subagent (type and the call that started it, absent for the Job's own agent), the `path` from the repository's root, the `act` (`write`, `edit` or `remove`) and `at`.
The window holds 60; the same file by the same hand as the latest touch moves its instant and is not a new one.
A hosted Job's touches are read off its process stream: the `Agent` call (once `Task`) names the type, and every line a subagent makes carries that call as `parent_tool_use_id`.
A call whose start Fleet never heard (a Fleet that restarted) is named `subagent`.
A terminal Job's come from the mod's `touched` fact, which carries the subagent's `agentId` (`by`) and the `act`; the type is read from the `subagent` attachment.
A hosted Job loads the mod too, so only the stream is recorded for it.
A removal is read from a plain `rm`, `git rm` or `unlink`; a command that removes by a pattern or a variable names nothing.

# The plan

**Part of:** [Job](../session.md).

Since 16 Oct 2026. The owner, 15 Oct 2026: a Job plans with him first, and what he approves is what gets built.
**Nothing in a plan is the agent's word about progress**: the plan is its statement of the work, and how far it got is Fleet's observation, as for the build record.

**`record_plan`** is a tool on the agent's door.
It carries a `summary` and `groups` in order.
The `summary` is a few plain lines in the person's own words saying what he gets, with no file names or types; it is trimmed, needed, and at most 600 characters, and a plan without one is refused like any other that breaks a rule.
Each group has a `title`, an `agent` (the subagent type that takes it, one word, free text such as `rust-engineer`), `paths` (from the repository's root; an edit under one marks the group worked), an optional `gives` (one line, at most 160 characters, of what the group gives him), an optional `lands_in` (a short label, at most 24 characters, for the pull request the group lands in: groups that ship together share one, a group handed to its own Job lands in that Job's pull request, and absent is `PR 1`; see below), an optional `issue` (the number of the issue the group works, which Fleet holds as an `issue` row on the Session once the plan is recorded, so a plan of tickets shows them before any pull request exists; see [what a Session keeps](what-is-kept.md). **Left blank, Fleet fills it** where the group's title or one of its tasks writes exactly one ticket the Job holds (`#42`, or a key such as `armada-3fz`, whole), and never over an `issue` the agent passed; a group naming none, or two, stays blank) and `tasks` (titles), and an optional `sketch` (`title` and `scene`), checked by the code `show_sketch` uses, so a scene that would draw as an empty frame is refused with the part at fault.
A group or task may send back the `id` Fleet gave it.
Fleet refuses a plan with every reason at once and says to call `record_plan` again; nothing is kept from a refused plan.
The agent gets back the version and each id Fleet gave.

**A group says which pull request it lands in, so the person can see whether approving means one or two.** Fleet binds a pull request to a `lands_in` label from the Job's ledger and every group that shares the label reads that pull request's number, title and state.
A group handed to its own Job takes that Job's pull request.
A pull request opened while a task was in progress takes that task's group's label.
A pull request that followed no task takes the first label, in plan order, that nothing has bound.
Until one is bound a group reads only its label.

**Recording again replaces the plan as a new version.** Fleet keeps the current version and the diff from the one before: a group or task is matched to the old one by its id, then by its text with case and spacing ignored, and what differs is `added`, `renamed`, `moved` (to another group) or `removed`.
An id the old version never held is not an id.
The sketch's scene is a ledger artifact `sketch:<title>`, as `show_sketch` keeps it, and the plan names it.

**A recorded plan is pending until the owner approves it, and pending is one of the Job's calls.** It is a waiting item, id `plan:<version>`, source `plan`, where `ask:` and `perm:` items go.
Nothing blocks the agent's tools meanwhile; the wait is the agent's to keep, and the Job rules say to keep it (`docs/contracts/agent-prompt.md`, 5f).
**The item is raised when it can be answered: only a window he can see or still owes holds it** (`plan_holds`, `planning.rs`).
**A pending plan waits while any of the Job's walks waits for his Approve**, closed window or not, and arrives once he approves them.
An Approve of any take settles the window, as it does the walk item.
A window a newer window of the same page replaced (page or take, compared without the port) holds nothing, nor does a walk he dismissed from the list, or the Job took back with `withdraw_window`, whatever their marks.
Showing the page again lifts the dismissal, so the window waits and holds the plan again.
**A window he answered in words, not with an Approve, before the plan was recorded holds nothing**: the plan is the Job's answer to what he said, and the walk item still waits on its own Approve.
Shown again after the recording, the window holds the plan again.
Since 20 Oct 2026, after he asked for a bigger change on a walk, the Job planned it, and the walk he had moved past kept the plan out of his sight.
**His answer to one of the Job's questions is not words on the walk**: Fleet writes it in the thread as his message, with an id naming the question's row (`answers_a_question`, `asking.rs`), and the walk still holds the plan.
Since 26 Oct 2026, after he answered a question on a Job whose walk he had not opened, and the plan came to him ahead of the walk.
**A document put to him holds the plan too, and his approval of the page approves the plan with it.** A Job whose evidence is a page, as a design plan's is, asks one approval: the plan is not raised while a page of the Job stands unanswered (`plan_holds`, hold `unapproved`, its `url` the page's path), and `answer_document` approving the page approves the pending plan version (`document_answered`).
A page sent back leaves the plan held.
Since 30 Oct 2026, after a design plan asked for its plan and its page at once, and the page's name in the thread opened nothing.
**An Approve stands until the page is shown again.** Any file the Job writes under its folder marks every window it showed as changing (`detail.working`, `readiness.rs`) and changed (`detail.changed`); a window he approved since it was last shown holds nothing whatever those marks say, and one he has not approved that was changed waits to be shown again.
A file outside the Job's folder, such as a memory note, marks nothing: the mod tells Fleet `wrote` only for a path under the session's directory.
The item begins once nothing holds it: its `since` is the latest of the recording, when each window that counts was shown again or made ready, and his Approves, so it calls even if he viewed the Job since the recording.
Since 19 Oct 2026, after the owner approved every walk he was shown and saw no plan: a take shown again inside another window, a window replaced on screen and a page changed after his Approve each held it, and he approved two dead windows by hand.
"This is so painful."
Since 27 Oct 2026, after a Job edited its design document six seconds after his Approve and the plan card went back on hold; the owner chose that his Approve stands.
**The plan on the record names what holds it**: `held_by`, each window's `url` and `why` (`unapproved`, `changing`, `changed`, or `walk` for the walk it names).
`record_plan` takes an optional `walk`, the url given to `show_window`; while a plan names one, the item waits for a message from him that approves that url, or a take of the window that showed it, at or after the newest window row showing it.
A page never shown keeps it held.
Recording again without `walk` clears it.
The receipt says `held` when the recording is not put to him yet, so the agent does not tell him it waits on his approval, and names each window in `held_by` with what settles it.
Sleep mode leaves it for the morning, and `answer_waiting` refuses it: a plan is approved, or sent changes, from its own card.

**"Not yet" puts the plan away.**
Dismissing the plan's call (`plan:<version>`) sets `set_aside_at` on that version, which is kept with the plan, and drops the call as it drops any derived item.
The Session is sent one message before the mark is set, so a send that fails leaves the call standing to press again: `Not ready for a plan yet. Keep talking, and record no plan until I ask for one.`
**A set-aside version holds nothing and calls for nothing**: no window is listed in its `held_by`, and no item is raised for it.
Recording again makes a new version without the mark, and its `plan:<version>` call waits as any first recording does.
Approving a set-aside version is not refused.
Bridge reads the plan first as its summary and a line for each group's `gives`, with the groups under Details, and draws a set-aside plan as a chip (*In Bridge*, *The plan card*).

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
Bridge draws it with the done check and a faint mark for no edit seen, tooltip `Done, no edit seen`, and counts it as done in the Blueprint, in a group's state and on the Field, where it is hollow.
`failed` is not set by anything Fleet observes yet.
An edit under a group's paths marks the group worked (`worked_at`); a group that names no paths takes every edit.

**Fleet marks a task itself where it has proof, and tells the agent once** (owner, 27 Oct 2026; `.claude/decisions/2026-10-27-fleet-keeps-the-task-list-honest.md`).
An audit of 70 Jobs found the agents' marks could not be trusted: half the finished tasks were never in progress, and a median of 44 calls went by with no task update.
The proof is a commit whose first line says a task not yet completed (matched as a list item is), the task's group's pull request merged (the group of the task in progress when the pull request was first kept), or an edit under a group's own paths while nothing is in progress, which puts that group's first open task in progress.
A mark Fleet set carries its proof as `marked` (`commit 1a2b3c4 names it`, `#12 merged`, `edits in <path>`), and Bridge draws the same mark with the proof in its tooltip.
**Fleet's mark stands until the agent's list changes that task**: a list that still says what it said before leaves it, and one that moves it takes its place.
A task in progress or completed carries `changed_at`, when it was put there by either.

**A task in progress can be `stale`**, with `why` and `since`, the last time its mark moved or an edit landed under its own group's paths.
In order: a newer task in progress leads and the older one is stale, unless a subagent edited the older one's group since it started, as two at work side by side do; edits landed in another group's files that has nothing in progress, where the task's own group names paths; or 40 of the agent's own calls went by without a touch.
Stale is never done, and the state stays `working`.
The calls are counted per plan and kept with it: a hosted Job's from its process, a terminal Job's from its mod (the `calls` fact, ten at a time, since mod 0.3.46), and a subagent's not at all.

**Each task is told once**, marked or stale, in one `task_marked` notice listing every task not told yet, asking the agent to bring its list into line or change the task if Fleet has it wrong.
A hosted Job hears it at its next tool call; a terminal Job's mod is handed it.
A merge's tasks go in the merge's own message instead.

**An adversarial review rides on the plan as `adversarial`.** The owner presses it on a plan waiting for approval, and it comes back as findings pinned to the group or task each is about, with the plan's own task matcher doing the pinning.
It advises and gates nothing, and recording the plan again marks it out of date rather than dropping it.
[Adversarial review](../adversarial-review.md) owns the rest.

**In a repository whose tracker is beads, an approved plan is also mirrored into the Job's issue.** Each group is filed as a child of the issue it holds, each task as a child of its group, and each group blocks the next in plan order ([tracker](../tracker.md), *Beads*).
The plan in Fleet stays the source: nothing is read back from the backlog, and a task's bead follows `state_of`, not a word the agent said.
A Job that holds no beads issue mirrors nothing.

**With no plan, the agent's own task list is what the Blueprint shows.** The list is kept whether or not a plan exists (*The build record*; `session_build_tasks`), so a Job that never records a plan still has one.
Bridge lists it in the order the agent made the tasks, each with a state mark: `done` for a completed task, `working` for the one the tool has in progress, `open` for the rest.
The running subagents are listed under it.
The row is published when the list changes, so the Blueprint follows live.
**A recorded plan takes over**: with one, the Blueprint shows the plan's groups and tasks and none of the agent's list, and the list goes on moving the plan's tasks by text as above.
A Job with neither a list nor a subagent at work, and no workflow to follow, stands folded to its band.
The Blueprint draws the plan open and editable while it awaits approval, and once approved as a Now view or the whole plan (`docs/concepts/session/in-bridge.md`).
The group being worked is the one `activity.account.group` names (*Working on*), else one with a task `working` or `failed`.

**Who touched which file.** `SessionRecord.touches` is a bounded window of the Job's recent file touches for the Field: the subagent (type and the call that started it, absent for the Job's own agent), the `path` from the repository's root, the `act` (`write`, `edit` or `remove`) and `at`.
The window holds 60; the same file by the same hand as the latest touch moves its instant and is not a new one.
A hosted Job's touches are read off its process stream: the `Agent` call (once `Task`) names the type, and every line a subagent makes carries that call as `parent_tool_use_id`.
A call whose start Fleet never heard (a Fleet that restarted) is named `subagent`.
A terminal Job's come from the mod's `touched` fact, which carries the subagent's `agentId` (`by`) and the `act`; the type is read from the `subagent` attachment.
A hosted Job loads the mod too, so only the stream is recorded for it.
A removal is read from a plain `rm`, `git rm` or `unlink`; a command that removes by a pattern or a variable names nothing.

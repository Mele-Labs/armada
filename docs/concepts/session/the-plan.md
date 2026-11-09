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

**A group names the gate that ends it: a fact Fleet sees for itself, never the agent's word** (owner, 8 Nov 2026).
`gate` is one of `plan_approved`, `walk_approved`, `sketch_approved`, `document_approved` (a page under the Job's documents he approved), `checks_green` (Checks Fleet runs at the pushed commit), `merged`, `jobs_ended` (every Job this Job started has merged or ended), or `none`.
Absent is `none`, and groups with no gate between them are one stretch.
A word not on the list is refused with the list, rather than dropped.
Each group carries its gate on the wire and in `session_plan_groups.gate`; recording the plan again replaces it.
**A gate closes once, on a fact Fleet saw, and stays closed** (`crates/fleet/src/gate_keeping.rs`).
Fleet keeps a `session_gates` row when it sees the fact, and never works it out again: the rail placed a Job afresh on every draw, so a fact that dropped out moved it backwards.
His approval of a plan closes every `plan_approved` gate for the version that asked for it, and a version recorded again stands on that approval: it draws no card, raises no item and reopens no gate, unless the agent passes `ask_again: true` on `record_plan`, which puts that version to him as the first was (owner, 8 Nov 2026).
Any other fact closes the first open gate of its kind in plan order, and no other: the merge of a pull request the Job holds, his approval of a page's newest revision or of a sketch, and his Approve once every walk shown with no `group` stands approved.
**A walk shown for a group is that group's part** (owner, 8 Nov 2026).
`show_window` takes an optional `group`, a plan group's id, and the `window` row keeps it.
A group gated on `walk_approved` closes when every walk shown for it since the plan was first recorded stands approved, so a feature Job has one stop for each part he would see.
A replaced or taken-back walk waits on nothing, as it does for the walks of no group.
`group` is refused with the ids the plan has when the current plan has no such group, or says the Job has no plan.
**A new walk for a group opens its gate again** (owner, 8 Nov 2026).
The close row is not edited or deleted: the gate reads its newest close against when a walk was last shown for the group (`session_gate_walks`), and a walk shown after the close opens it.
Approving that walk writes a new close row, numbered by its `version`, and the group's `gate_closed` is the newest.
An open gate holds `all_done` and raises its Needs you item again, as it did the first time.
A reopened gate's close tells the rules `gate_closes` again and does not end or start steps a second time.
A skip is his word, and a walk shown after it reopens nothing.
Only facts from after the plan was first recorded count.
Each gate is watched on its own, so a merge closes its gate while an earlier one is open.
A group keeps its close through a recording that keeps its id and its gate.
The group's `gate_closed` carries the close: when, the fact in one line, and the commit where there is one.
Bridge works out the live stretch from it: the first group whose gate is open, with the groups before it that name none.

**Fleet closes a `checks_green` gate by running Checks itself** (`crates/fleet/src/gate_checks.rs`), never on the agent's word that they passed.
A push of the Job's branch queues a run while a group gated on `checks_green` is open, and nothing else does.
The run is the manifest's test Check over the tests the push touched, read as an agent's `armada check test --changed` reads them, in one detached worktree per repository at `.armada/gate-check` that keeps its `target/` between runs.
It waits for a place behind every Check a Drone waits on, runs one at a time per repository, and a newer push replaces the Job's queued older one.
A run that fails names its failing tests, and Fleet runs only those at the commit the branch met main at.
A test red there too is `already_red` and does not hold the gate; a base result is kept per repository, base commit and test, so a base runs a test once.
Green closes the gate with the fact `Checks passed at <sha>`, with `, <n> already red on main` where tests were, or `: no test touched` where the push touched none.
While the gate is open the first such group carries `gate_checks`: the commit, `running` or `red`, the tests that failed and the ones also red on main.

**The agent is told when a gate closes, and `all_done` waits for every gate** (`crates/fleet/src/gate_holding.rs`, owner, 8 Nov 2026).
Each close sends one line, `Gate closed on "<group title>": <fact>.`, from one funnel, and only for a row written then.
A red run of the Checks sends one line too, `Checks failed at <sha>: <tests>.`, with `Already red on main: <tests>.` after it where any were.
`all_done` is held, not refused, while a group of the plan has a gate open.
Fleet keeps the call and its summary in `session_gate_holds`, stamps no `done_since`, and answers that the call was held and on which group.
Open follow-ups still refuse it, and the in-flight reason is skipped, since the gate covers it.
When the last gate closes Fleet runs the call itself, and tells the agent if it is refused.
A call held stops standing as `done_since` does: when he writes, or a turn of the agent's own begins.
A held call reaches the person in Needs you only for a gate nothing in the Job can close, and only while nothing in the Job runs.
That is an approval of any kind, a merge no open pull request is on the way to, or Checks no run has begun.
The item offers `Approve` and `Skip this group`, and goes when the gate closes.
`Approve` on a `plan_approved` gate approves the plan, and on any other closes the gate on his word.
`Skip this group` closes it as skipped.

**A Job's lifecycle rules hear its step moments from the gate rows, never from the rail** (`crates/fleet/src/lifecycles/ruling.rs`, owner, 8 Nov 2026).
The rail's derived step slid backwards after a merge and moved a merged Job's ticket back, so `step_starts` and `step_ends` no longer follow it.
A stretch is the groups after the previous gated group, up to and including the next one, and its steps are the groups' ids.
A close that writes a row ends every group of its stretch, then closes its kind, then starts every group of the next stretch.
The first stretch starts when the first version of the plan is recorded, and a later version starts nothing.
`when: { gate_closes: <gate> }` fires on a gate's kind, `plan_approved` to `merged`, so a rule works for a plan the Job wrote itself.
Each moment is stamped from the row it came from, so a rule runs once for a close however often Fleet hears it.
A pull request opening is still the Job's own event, `when: { job: pull_request_opened }`.

**A group says which pull request it lands in, so the person can see whether approving means one or two.** Fleet binds a pull request to a `lands_in` label from the Job's ledger and every group that shares the label reads that pull request's number, title and state.
A group handed to its own Job takes that Job's pull request.
A pull request opened while a task was in progress takes that task's group's label.
A pull request that followed no task takes the first label, in plan order, that nothing has bound.
Until one is bound a group reads only its label.

**Recording again replaces the plan as a new version.** Fleet keeps the current version and the diff from the one before: a group or task is matched to the old one by its id, then by its text with case and spacing ignored, and what differs is `added`, `renamed`, `moved` (to another group) or `removed`.
An id the old version never held is not an id.
The sketch's scene is a ledger artifact `sketch:<title>`, as `show_sketch` keeps it, and the plan names it.

**A recorded plan is pending until the owner approves it, and pending is one of the Job's calls.** Once he has approved a plan, a version recorded after it is not pending unless the agent passed `ask_again: true`: it keeps the earlier `approved_at`, the receipt says it `stands`, and the agent goes on.
A pending plan is a waiting item, id `plan:<version>`, source `plan`, where `ask:` and `perm:` items go.
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
A reply of his in the thread read as approving the page approves the plan the same way ([Document](../document.md)).
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
`answer_waiting` refuses it: a plan is approved, or sent changes, from its own card. A plan is a kind [Autopilot](../autopilot.md) may approve.

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

**Fleet marks a task itself on one proof, and says nothing to the agent** (owner, 27 Oct 2026; narrowed 5 Nov 2026).
The proof is a commit whose first line says a task not yet completed (matched as a list item is): that task is completed.
The mark carries its proof as `marked` (`commit 1a2b3c4 names it`), and Bridge draws it with the proof in its tooltip.
**Fleet's mark stands until the agent's list changes that task**: a list that still says what it said before leaves it, and one that moves it takes its place.
A task in progress or completed carries `changed_at`, when it was put there by either.
Fleet marks no task from edits, from calls, or from the order tasks start, and flags none as stale: those were guesses.
A merge marks no task either: which tasks a pull request finished is a guess.

**A Judge's reading of the plan rides with it.** The owner presses the Judge on a plan waiting for approval, and it comes back as notes pinned to the group or task each is about, with the plan's own task matcher doing the pinning.
It advises and gates nothing, and recording the plan again marks it out of date rather than dropping it.
[Judge](../judge.md) owns the rest.

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

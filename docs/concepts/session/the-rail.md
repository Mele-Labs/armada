# The rail

**Part of:** [Job](../session.md).

Since 16 Oct 2026. A Job that follows a workflow has a **current step**, and it is **read off what Fleet observed, never off the agent's word**.
`SessionRecord.workflow_step` is its index in the workflow's steps; it is derived each time the row is drawn (`rail.rs`, `Fleet::rail_drawn`), so nothing stores it beside the facts it comes from.
It is absent for a custom Job, once the delivering step's pull request has merged with no step after it, and while `features.workflows` is off or `workflows.off` names its workflow for its repository.

**Three anchors are exact.** A `records_plan` step is current until a plan is approved, which a plan recorded after one of its listed ends, or on a `plan_unasked` step, is at once ([Workflow](../workflow.md#walks-and-replies)).
A `follows_plan` step is current from approval, and stays until a later step is entered.
A `delivers` step is current from the first push until its pull request merges.

**Steps before the plan, and the walk** (27 Oct 2026, `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`).
A step before the recording step is current until a plan is recorded, or until any end its `ends_on` lists holds (28 Oct 2026).
Each end is read off the thread: he replies, approves a walk, approves a sketch, approves a draft, or picks an option the agent's question gave a preview.
The reply is a message of his after the agent's first turn: Fleet's own notices and other Jobs' words are not one.
A walk he approves before the plan's approval ends such a step and never passes the later `walks` step.
The recording step, when something precedes it, is entered when a plan is recorded and is current until he approves it.
A `walks` step is current from the plan's approval until he has approved every window the Job showed since (`show_window`), leaving out one a newer window of its page replaced and one taken back (2 Nov 2026).
A walk is expected as [Workflow](../workflow.md#walks-and-replies) says.
It is passed over only when the plan was approved with no walk expected and the step's own work is done: for a `walks` step that also follows the plan, such as the bug workflow's Fix, once the plan's tasks are finished, and for one that does not, at the plan's approval.
It never waits for the push, which ends the step after it: a bug Job with nothing to show finished its tasks, waited to be moved on to Tests, and Fleet waited for a push (30 Oct 2026).
The step after it is entered once it is passed, whatever fact would have entered it: a window shown again voids his approval, and a Job left on the step before the walk would show it again (30 Oct 2026).
Fleet reads these from the thread's window rows and his Approve messages, and from the plan; the agent's word is never an input.

**A step that ends on a lifecycle state** (29 Oct 2026, "Workspaces, lifecycles and rules") is entered once the Job holds an instance of that lifecycle and left when the state is reached.
`pr: approved` needs every pull request the Job holds, `any pr: approved` the first; `job: landed` needs every Job it started.
Fleet reads the moves it already keeps for the Job's instances, so nothing polls: a kept move redraws the row of a Job whose step waits, the rail moves it, and `rail_acted_on` sends the agent one message saying what got there.
**Reached stays reached**: a reviewer taking an approval back does not drag the Job to the step it left, and an instance that is over has reached every state of its lifecycle.
The step is neither the Job's nor the owner's to end, so it raises no Needs-you item.
A Job holding none of the lifecycle is not on the step, and nothing after it is entered, the delivering step included; the owner writing on the step or skipping it moves the Job on, and no line says a state was reached.
**On a delivering step the state gates the merge and does not end the step** ("How Land behaves"): Land is entered on the push as always, and Fleet wakes the agent where it stands, with one line, once the state comes, unless the thread already holds that line or the owner wrote on the step.
Only landing passes it.
The step after one is entered when it ends, whatever fact would have entered it, and the rail ends when the last step does; orchestrate's Oversee is one, and Summarise is entered the moment the last Job has landed or ended.

**A live instance holds its Job open** (30 Oct 2026, "Workspaces, lifecycles and rules"), as an open pull request does.
A ticket, pull request, deploy or running Job the Job holds that stands in a state its lifecycle knows and does not end on refuses the agent's `all_done` and `finish_up` with `Deploy is Rolling out`, one line for each.
**An unread instance, and one in a status no state lists, holds nothing**: a tracker that declares no words (Jira, a tracker mod) leaves the built-in Ticket lifecycle listing none but `closed`, so a Job holding a ticket under it would be refused with nothing the agent could do to clear it.
A person can teach the status from the ticket's row, and the Job's ticket then stands in the state they chose ([the tracker](../tracker.md)).
**A tracker that declares them does not**: a beads ticket reading `open` or `in_progress` stands in To do or In progress, and holds its Job open until it closes.
**A Job it started that has not begun holds nothing** (3 Nov 2026, `.claude/decisions/2026-11-03-a-job-that-has-not-begun-holds-nothing.md`), on its start card or waiting on another, whatever its instance reads, so a Job can finish and give back its slot while what it started waits.
Ending the Job neither ends the one that has not begun nor stops it starting when its condition is met, a wait on the Job's own end included.
Once it runs it holds the Job again, if that Job is still live; `hosting.ran` says which, and is kept when its process first spawns.
End lists them all the same.
Fleet keeps that the agent was told to wait, and when the last instance holding it is over sends it one `lifecycles_clear` notice, which names any Job that has not begun and says it may finish; an agent never refused is not told.
Fleet asks again at its start, since an agent refused for a Job that had not begun, before 3 Nov, has no move coming to tell it.
**The pull reading reads a pull request a live Job holds until it reaches an end**, however its ledger row was settled: the agent's own `gh pr merge` or a press in Fleet may settle the row first, and neither moves the instance.
**A ticket the Job hands to the Job it started for the ticket's plan group ends the same way, at once**, and the Job it went to holds it ([linked Jobs](linked-jobs.md)): the rail neither waits on it nor counts it reached.
**Ending a Job ends its instances.** Each one no end state has taken, the unplaced too, is kept as ended, standing where it was, no reader moves it again, and the watches the Job's rules started are stopped with the Job's others.
Before it ends, End lists what is in flight as `Deploy: Rolling out` and `ARM-42: In review`; End job goes on, and Keep going leaves the Job as it is.
**Every End asks the same way**: the Job's menu and done band, the Cockpit's End job on a done call and the Deck row's Close all list first, and a Job holding nothing ends at once from each.
A Job started before lifecycles holds none, and is finished and ended as it was.
The code is `crates/fleet/src/lifecycles/ending.rs`.

**The steps between are read loosely**, each entered on the fact that fits it, only after every anchor before it has been passed:

| Step declares | Entered on |
|---|---|
| evidence `diff` | the first edit |
| evidence `test_suite_run` or `failing_test`, or a Manifest Check | a Check run the Job asked for, or the pull request's checks |
| the `delivery` phase | a commit |
| evidence `review` | a pull request opened |
| anything else | never by itself |

**Where a step cannot be told apart from the one before it, the earlier is kept**: a step that fits the same fact as its neighbour is not entered.
**The rail keeps its steps by id** (`session_rail`, `by_id`): the step the Job stood on, the ones it began, skipped, stalled on and was judged at.
A workflow saved under a running Job, with a step added, dropped or moved, changes the indexes and not the steps: each kept id is read back as its place in the steps the Job has now, and a step the workflow no longer has is left out.
**A step added before where the Job stood is passed over** where no step is entered, since the Job never began it; a rail kept before ids is read as indexes until Fleet's start rewrites it by id.

**Every move is entered on the timeline** as `session.step`, `says` `step: <old> → <new>`, a step by its label, `start` before the first and `done` past the last.

**Fleet tells the agent nothing of the step it is on** (5 Nov 2026, `.claude/decisions/2026-11-05-fleet-stops-guessing.md`).
Entering a step sends no `instructions`, a step skipped draws no nudge, and a turn that ends on a step draws no word about it.
The rail only reads the step and enters each move on the timeline.
The one thing a person's press sends is a fact: a step he skipped, undid or moved the Job past is told once, and a Job that has not run carries the skips with its first turn.
The delivery question a `asks_delivery` step asks, and the choice that answers it, are sent as they were.
`instructions` and `about` stay on the definition and the workflow's page, and are not sent.
A Job that has not run is sent nothing, since a word sent now would start it ahead of the message it waits for.

**With `features.workflows` off, or a Job's workflow in `workflows.off`** (Settings, live; the owner's decision of 16 Oct 2026) there is no step and no step judge, even for a Job that already has a workflow, 
The Job keeps the workflow it has.
The proposer is shown none and never picks one, and `set_session_workflow` refuses to set one (`fleet.session_workflows_off`) but still clears one.

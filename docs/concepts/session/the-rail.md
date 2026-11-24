# The rail

**Part of:** [Job](../session.md).

Since 16 Oct 2026. A Job that follows a workflow has a **current step**, and it is **read off what Fleet observed, never off the agent's word**.
`SessionRecord.workflow_step` is its index in the workflow's steps; it is derived each time the row is drawn (`rail.rs`, `Fleet::rail_drawn`), so nothing stores it beside the facts it comes from.
It is absent for a custom Job, once the delivering step's pull request has merged with no step after it, and while `features.workflows` is off or `workflows.off` names its workflow for its repository.

**Three anchors are exact.** A `records_plan` step is current until a plan is approved, which a plan recorded after one of its listed ends, or on a `plan_unasked` step, is at once ([Workflow](../workflow.md#walks-and-replies)).
A `follows_plan` step is current from approval, and stays until a later step is entered.
A `delivers` step is current from the first push until its pull request merges.
A step gated `plan_approved` records the plan, and one gated `walk_approved` walks, as if it said so (20 Nov 2026).
The workflows shipped on 9 Nov state every step by its gate alone, and until then the rail read no anchor on them: 33 feature and bug Jobs stood on their first step from the plan's approval to the merge.

**Steps before the plan, and the walk** (27 Oct 2026, `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`).
A step before the recording step is current until a plan is recorded, or until any end its `ends_on` lists holds (28 Oct 2026).
Each end is read off the thread: he replies, approves a walk, approves a sketch, approves a draft, or picks an option the agent's question gave a preview.
The reply is a message of his after the agent's first turn: Fleet's own notices and other Jobs' words are not one.
A walk he approves before the plan's approval ends such a step and never passes the later `walks` step.
The recording step, when something precedes it, is entered when a plan is recorded and is current until he approves it.
A `walks` step is current from the plan's approval until he has approved every window the Job showed since (`show_window`), leaving out one a newer window of its page replaced and one taken back (2 Nov 2026).
A walk is expected as [Workflow](../workflow.md#walks-and-replies) says.
It is passed over only when the plan was approved with no walk expected and the step's own work is done: for a `walks` step that also follows the plan, once the plan's tasks are finished, and for one that does not, such as the feature workflow's Walk and the bug workflow's Fix, at the plan's approval.
It never waits for the push, which ends the step after it: a bug Job with nothing to show finished its tasks, waited to be moved on to Tests, and Fleet waited for a push (30 Oct 2026).
The step after it is entered once it is passed, whatever fact would have entered it: a window shown again voids his approval, and a Job left on the step before the walk would show it again (30 Oct 2026).
A delivering step right after a walk is entered then too, with no push.
Fleet reads these from the thread's window rows and his Approve messages, and from the plan; the agent's word is never an input.

**A `previews` step** (20 Nov 2026) is an anchor as a walk is, with his Looks good on the Ready to try card in place of a window's Approve: [Workflow](../workflow.md#walks-and-replies) says when it is passed over.
It is entered once the plan is approved and the step before it is passed, or once the Job offers a branch, and ended only by the `you_approved` gate his Looks good closes.
Fleet keeps a note for each offer, so a withdrawn offer, or a branch that moved, leaves the step held.
The delivering step after it is entered once it ends, push or not.

**A step that ends on a lifecycle state** (29 Oct 2026, "Workspaces, lifecycles and rules") is entered once the Job holds an instance of that lifecycle and left when the state is reached.
`pr: approved` needs every pull request the Job holds, `any pr: approved` the first; `job: landed` needs every Job it started.
Fleet reads the moves it already keeps for the Job's instances, so nothing polls: a kept move redraws the row of a Job whose step waits, the rail moves it, and `rail_acted_on` sends the agent one message saying what got there.
**Reached stays reached**: a reviewer taking an approval back does not drag the Job to the step it left, and an instance that is over has reached every state of its lifecycle.
The step is neither the Job's nor the owner's to end, so it raises no Needs-you item.
A Job holding none of the lifecycle is not on the step, and nothing after it is entered, the delivering step included; the owner writing on the step or skipping it moves the Job on, and no line says a state was reached.
**On a delivering step the state gates the merge and does not end the step** ("How Land behaves"): Land is entered on the push as always, and Fleet wakes the agent where it stands, with one line, once the state comes, unless the thread already holds that line or the owner wrote on the step.
Only landing passes it.
The step after one is entered when it ends, whatever fact would have entered it, and the rail ends when the last step does; orchestrate's Oversee is one, and Summarise is entered the moment the last Job has landed or ended.

**A step gated `document_approved`, `sketch_approved`, `jobs_ended` or `checks_green`, with no anchor, holds the Job until that fact** (20 Nov 2026), as a step listing an end does.
Before this, such a step was never entered, and orchestrate and refactor Jobs stood on Plan throughout.

**Work ahead of a step that waits on his Approve moves the Job on and flags the step** (20 Nov 2026, the owner's answer on *The Job's right column, by workflow step*).
A push, a pull request or a merge is work for a later step, so where a later step delivers, a step that waits on his Approve of a plan, a page, a sketch, a draft or an item he was never given is passed, and the Job stands where the work is.
The passed step is listed in `workflow_strip.passed_unapproved`, and the column draws it `passed without your Approve` until he gives the Approve.
**A walk is not passed this way**, nor a step that `previews`: a push never passes a walk, and it is how a preview is made, so a Job whose plan expects one stands on it with the plan flagged.
A workflow with no delivering step passes nothing, since its push belongs to no step.
Before this, Propose stood from the plan to the merge on 41 Jobs that held a pull request.

**The last step that lists nothing ends when the Job does** (20 Nov 2026).
It is entered once every step before it is passed, and its end reads `the Job ends`.
A step in the middle that nothing Fleet observes ends passes at once, so the Job never stands on it: the workflows page warns of it (`StepReading.warning`), and the load still accepts the definition.

**The column follows the step** (20 Nov 2026).
Each step lists the tabs it opens in `tabs`, the front one first, as ids of the native Bridge's zones and `now` and `walks` beside them (`core_model::STEP_TABS`); a step lists none until a workflow says.
`workflow_strip` on the Job's row carries them with the step's usual time (`usual_secs`), what ends it (`ends_when`) and whether it is stalled; the time on the step is the last of `workflow_visits` for `workflow_step`.
**The usual time is measured**: the 90th percentile of how long Jobs on the workflow stayed on the step before the next began, from the visits the rail kept, and absent below five (`crate::step_stalls`).

**A step past its usual time with no move is stalled, and the Job is told once** (20 Nov 2026).
The row carries `workflow_strip.stalled`, when it went past and the cause, which says what the step waits on and whose move it is now.
**An end that is his is his move only once there is something to press** (24 Nov 2026): while the Job is still working, or a plan he would approve was never recorded, the cause says the Job.
**A step he wrote on stays plain until he has been quiet on it for half an hour** (`QUIET_SECS`, 24 Nov 2026): a step he is working through with the Job is not stalled.
Fleet tells the Job once for each visit to the step, as a `rail_nudge`, at a quiet moment: its turn is over and nothing waits on him.
**This is the one word Fleet sends about the step**, which the owner asked for beside the rule below, and it repeats nothing: a step the Job leaves and enters again draws a new one.

**His press is the fact, and the agent's word is not** (20 Nov 2026).
A step that ends on his Approve of an item (`gate: you_approved`) is told to raise one `waiting_for` item with `approves: true` when it thinks the step is done, and not to say so in the thread.
The step ends when he presses Approve on it.
A Looks good on a Ready to try card closes the same gate.

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

**Fleet tells the agent nothing of the step it is on, but for one word** (5 Nov 2026, `.claude/decisions/2026-11-05-fleet-stops-guessing.md`; the word is the stalled-step nudge above, 20 Nov 2026).
Entering a step sends no `instructions`, a step skipped draws no nudge, and a turn that ends on a step draws no word about it.
The rail only reads the step and enters each move on the timeline.
The one thing a person's press sends is a fact: a step he skipped, undid or moved the Job past is told once, and a Job that has not run carries the skips with its first turn.
The delivery question a `asks_delivery` step asks, and the choice that answers it, are sent as they were.
`instructions` and `about` stay on the definition and the workflow's page, and are not sent.
A Job that has not run is sent nothing, since a word sent now would start it ahead of the message it waits for.

**With `features.workflows` off, or a Job's workflow in `workflows.off`** (Settings, live; the owner's decision of 16 Oct 2026) there is no step and no step judge, even for a Job that already has a workflow, 
The Job keeps the workflow it has.
The proposer is shown none and never picks one, and `set_session_workflow` refuses to set one (`fleet.session_workflows_off`) but still clears one.

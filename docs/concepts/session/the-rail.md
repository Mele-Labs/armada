# The rail

**Part of:** [Job](../session.md).

Since 16 Oct 2026. A Job that follows a workflow has a **current step**, and it is **read off what Fleet observed, never off the agent's word**.
`SessionRecord.workflow_step` is its index in the workflow's steps; it is derived each time the row is drawn (`rail.rs`, `Fleet::rail_drawn`), so nothing stores it beside the facts it comes from.
It is absent for a custom Job, once the delivering step's pull request has merged with no step after it, and while `features.workflows` is off.

**Three anchors are exact.** A `records_plan` step is current until a plan is approved.
A `follows_plan` step is current from approval, and stays until a later step is entered.
A `delivers` step is current from the first push until its pull request merges.

**Steps before the plan, and the walk** (27 Oct 2026, `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`).
A step before the recording step is current until a plan is recorded, or, with `awaits_reply`, until the owner replies.
The reply is a message of his after the agent's first turn: Fleet's own notices and other Jobs' words are not one.
The recording step, when something precedes it, is entered when a plan is recorded and is current until he approves it.
A `walks` step is current from the plan's approval until he approves a window the Job showed (`show_window`).
A walk is expected when the plan said `seen: true` in `record_plan` (the person would see the change; it holds nothing), named a `walk` (a page shown before the plan, which holds it), or the Job showed a window.
It is passed over only when the plan was approved with no walk expected and the Job has pushed.
Until it pushes, the step stays current, because the mock is still being built.
A `follows_plan` step after it, or any step that fits a fact, is entered only once it is passed.
Fleet reads these from the thread's window rows and his Approve messages, and from the plan; the agent's word is never an input.

**The steps between are read loosely**, each entered on the fact that fits it, only after every anchor before it has been passed:

| Step declares | Entered on |
|---|---|
| evidence `diff` | the first edit |
| evidence `test_suite_run` or `failing_test`, or a Manifest Check | a Check run the Job asked for, or the pull request's checks |
| the `delivery` phase | a commit |
| evidence `review` | a pull request opened |
| anything else | never by itself |

**Where a step cannot be told apart from the one before it, the earlier is kept**: a step that fits the same fact as its neighbour is not entered.
A workflow edited under a running Job moves the indexes; the rail does not follow that.

**Every move is entered on the timeline** as `session.step`, `says` `step: <old> → <new>`, a step by its label, `start` before the first and `done` past the last.

**Entering a step sends its `instructions`.** A step may carry the words the agent follows at it, and Fleet sends them as one message from Fleet when the rail enters the step, naming the workflow and the step.
A step with none sends nothing, and a step entered again is sent again.
`about` is the owner's line and is never sent as instructions.
A step with `asks_delivery` holds its instructions until the owner has said how the work is delivered, and the answer sends them: *The delivery question*.
**A Job that has not run is sent nothing**, since a word sent now would start it ahead of the message it waits for.
The rail keeps its step and enters it on the timeline, and the Job's first turn carries the workflow line, the step's instructions and the message together, so no second message follows.
That holds for a Job an agent started, which waits for the owner's send, and for one that waits on another, which gets no step prompt before its condition is met.
A step with `asks_delivery` is read from that first message: one that names a delivery goes with the step's instructions, and one that does not goes with the question, the instructions waiting for the answer.
A Job whose Delivery was settled before Start is asked nothing, and its first turn carries the choice with the instructions.

**A skipped step draws one nudge, and nothing blocks.** Skips are told once for each step in a Job's life: files edited before the `records_plan` step's plan was approved, a push while the plan's tasks are open, and a test file written before the `walks` step is passed, when a walk is expected.
In a workflow with a `walks` step, editing before the plan is approved is no skip, because a rough mock may be built while it is discussed; a push then, or a test file then, is.
A test file is one under `tests/` or `__tests__/`, or named `*.test.*` or `*_test.*` (`test_paths.rs`).
Each nudge sent is also kept against the workflow, where it came from and the step, with a short fact such as *a test before the walk was approved*, so the workflow's page can list the Jobs that drifted there.
A kept nudge marked not a drift stops counting and stays listed (`workflow_nudges`).
Fleet sends the agent one short message from Fleet naming the step and what it asks (the step's own `about`, cut at 200 characters, else a default), and ends it with *Nothing is blocked*.
A nudge that cannot be delivered because nobody is there is asked again later and not counted.

**With `features.workflows` off** (Settings, live; the owner's decision of 16 Oct 2026) there is no step, no nudge and no boundary judge, even for a Job that already has a workflow, and the first turn does not tell the agent the steps.
The Job keeps the workflow it has.
The proposer is shown none and never picks one, and `set_session_workflow` refuses to set one (`fleet.session_workflows_off`) but still clears one.

# The rail

**Part of:** [Job](../session.md).

Since 16 Oct 2026. A Job that follows a workflow has a **current step**, and it is **read off what Fleet observed, never off the agent's word**.
`SessionRecord.workflow_step` is its index in the workflow's steps; it is derived each time the row is drawn (`rail.rs`, `Fleet::rail_drawn`), so nothing stores it beside the facts it comes from.
It is absent for a custom Job, once the delivering step's pull request has merged with no step after it, and while `features.workflows` is off.

**Three anchors are exact.** A `records_plan` step is current until a plan is approved.
A `follows_plan` step is current from approval, and stays until a later step is entered.
A `delivers` step is current from the first push until its pull request merges.

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

**A skipped step draws one nudge, and nothing blocks.** Two skips are told, each once for each step in a Job's life: files edited before the `records_plan` step's plan was approved, and a push while the plan's tasks are open.
Fleet sends the agent one short message from Fleet naming the step and what it asks (the step's own `about`, cut at 200 characters, else a default), and ends it with *Nothing is blocked*.
A nudge that cannot be delivered because nobody is there is asked again later and not counted.

**With `features.workflows` off** (Settings, live; the owner's decision of 16 Oct 2026) there is no step, no nudge and no boundary judge, even for a Job that already has a workflow, and the first turn does not tell the agent the steps.
The Job keeps the workflow it has.
The proposer is shown none and never picks one, and `set_session_workflow` refuses to set one (`fleet.session_workflows_off`) but still clears one.

# Protocol 23.20: a step tuned at the press

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: the approval canvas tunes each step, and lands only once Fleet takes what
it draws.

**One optional field, one DTO and one enum, all additive.** `ApproveDispatch` gains `tuning`, a
`StepTuning` per step a person tuned, `gates`' shape: `step_id`, and `model?`, `effort?`
(`Effort`: `low`, `medium`, `high`), `context?`, `judges?` and `checks_off?`. A field left out is
the step as declared, so a 23.19 Bridge approves as before.

**Frozen into the step, after its gate**, as `gates` is, so the Job keeps it for its life and each
reader finds it where it already reads the step. `model` is the step's model in
`Job::model_spawned_for`'s order, under a task's pick, a tier and `set_model`. `effort` goes to the
harness at the spawn; absent sends nothing, the harness's own default. `context` is the Drone's
FOR THIS PART block (`../contracts/agent-prompt.md`). `judges` sets every judge check's
`panel_size`. `checks_off` drops Manifest Checks by name and refuses Fleet's own looks, as the gate
box does.

**Refused, keeping nothing**, with 422 `fleet.unacceptable_proposal`: a step the workflow lacks,
one tuned twice, a blank model, `judges: 0`, `judges` on a step with no Judge, a Check the step
does not run, and a built-in. A model `list_models` does not offer is refused as `tiers`' is.
**`StepTuning` refuses an unknown field**, so the canvas's `harness` is refused rather than
dropped: one harness runs, and a per-step one is not built.

**No store migration**: the frozen workflow column carries `effort` and `context` beside
`model`. `get_job` reads `checks` and `judge_checks` off the frozen step as before; the effort and
the words are not served back. The event stream is untouched.

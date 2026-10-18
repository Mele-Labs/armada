# Protocol 13.48: where a Check runs

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`DeclaredCheck.runs_at`, `StepDetail.held_for_handoff` and `WorkflowStep.held_for_handoff`, additive (#849).
`runs_at` is `gate` for a Check a Drone's own run never asks and `handoff` for one that runs last, on the step before handoff, once every other Check there passes; absent is everywhere.
`held_for_handoff` names the handoff-only Checks a step's gate leaves to a later step, so a step that passed is not read as having run them.
A handoff-only Check that was not reached records `skipped`, with its own sentence in `produced`.

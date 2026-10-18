# Protocol 23.31: what a step does, in words

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 5 Oct 2026, at the approval gate on a workflow he had not used: *I have no idea what that
means.*

**One optional field, additive.** `WorkflowStep` and `StepDetail` gain `about`: a line or two on what
the step does for the Job and what it hands on. A workflow step declares it as `about:` beside
`label:`. Absent where the step wrote none, and a blank one is read as absent. `StepDetail` reads it
off the frozen workflow (`about` is written beside `label` in the Job's frozen steps), so a Job
frozen before 23.31 shows nothing there.

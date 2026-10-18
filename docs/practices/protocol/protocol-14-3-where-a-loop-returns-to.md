# Protocol 14.3: where a loop returns to

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1149`.
`WorkflowStep.verdict_routing_target` and `WorkflowStep.iteration_cap`, and `StepDetail.verdict_routing_target` beside the existing `StepDetail.pass`, all additive.
A person approving a dispatch could not see that a workflow loops, because `structure: loop` only labels the edge — `verdict_routing`, in `crates/config/src/workflow.rs`, is the only place it is named, and it had never crossed.
`pass.of` already carried the cap on a running Job's step; the target it points at had not, on either DTO.

**On the step that sends the work back, not the step it is sent to** — `pass`'s own rule, and the same edge.
`WorkflowStep` carries both fields together: a target with no cap could not stop, and a cap on a step routing nowhere answers a question a preview never asks.

# Protocol 23.19: a step's phase, and the grants a canvas draws

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: the approval canvas lays its lanes by phase. #1768 waits on it too.

**Optional fields and one enum, all additive.** `WorkflowStep` gains `phase`, `may_dispatch_jobs`
and `drone_per_task`; `StepDetail` gains `phase` and `may_dispatch_jobs`, `drone_per_task` being
there since 23.1. `StepPhase` is `setup`, `work` or `delivery`, a plain closed set Bridge lays a
lane per value of rather than matching on.

**Fleet serves the phase resolved.** A step's own `phase:` where it declares one, else `delivery`
where it `delivers` and `work` otherwise: `core_model::StepPhase::of` spells it once. Design Plan's
`present` and Epic's `roll_up` declare `delivery`, since neither sends anything out. On
`StepDetail` it is absent where Fleet cannot say, as `delivers` is. The two booleans are absent at
false.

**The workflow file gains `phase:`**, a word refused outside the three. **No store migration**:
the frozen workflow column carries `phase` beside `model`, null where the step declared none, and
a row frozen before it reads as none. The event stream is untouched.

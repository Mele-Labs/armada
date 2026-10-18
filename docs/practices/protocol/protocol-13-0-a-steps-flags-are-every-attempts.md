# Protocol 13.0: a step's flags are every attempt's

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#791`. `StepDetail.flagged` held the newest attempt's gaming flags, with no
attempt on them; it now holds every attempt's, each stamped with the run that
raised it — the change `check_runs` and `judged` took at 7.0.

**The field is not new, its meaning is, and that is the major.** A Bridge built
before this reads `flagged` as where the step stands now, and handed every
attempt's flags it would draw a run that is over as the reason the step is held.
The store always kept a flag's attempt; only the detail route dropped it.

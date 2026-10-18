# Protocol 23.23: the harness a step runs under

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: the card shows the one harness Fleet runs, and any other is refused.

**Two optional fields, additive.** `ModelChoices` gains `harnesses`, the harness names a Drone may
run under, one today and absent from a Fleet before 23.23. `StepTuning` gains `harness`. A name
`harnesses` does not list is 422 `fleet.unacceptable_proposal`, and the message names the ones that
do. The one name is spelled in `adapters` (`HeadlessAgent::harness_name`). The tuning keeps no
harness, since there is one to run, so nothing about it is frozen or served back.

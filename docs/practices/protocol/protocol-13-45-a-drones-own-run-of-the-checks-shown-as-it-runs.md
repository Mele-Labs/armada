# Protocol 13.45: a Drone's own run of the Checks, shown as it runs

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`StepDetail.dry_run` and the `job.dry_run` event, additive: the Checks a Drone asked for mid-step, in `ChecksUnderway`'s shape, from their start until the Drone asks again, submits or the step ends (#1062).
`checking` stays the gate's alone, and an event kind of its own keeps a Bridge that does not know it from drawing a Drone's run as the gate's.
`CheckUnderway.stopped_by`, on a Drone's run alone, names the Check whose failure stopped this one before it finished, and its `produced` says so in words.

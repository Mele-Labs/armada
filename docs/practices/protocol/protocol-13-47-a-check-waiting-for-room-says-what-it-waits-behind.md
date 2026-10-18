# Protocol 13.47: a Check waiting for room says what it waits behind

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`CheckUnderway.waiting_behind`, additive: on a Check still waiting, how many Checks from other work hold the machine's places while its run waits for one (#1063). `LimitValues.checks_at_once` keeps its shape and range and now counts across the machine — every Job's gate, every Drone's own run, fix drafts and proofs after a merge share it — so a gate can wait on work that is not its own. Absent is a Check waiting on nothing but its own run.

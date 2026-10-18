# Protocol 11.3: a Judge that is unsure asks, rather than stopping the step

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/judge.md`'s asking design, closing #694. A refusal on a
criterion marked `refuse` still stops the step exactly as every refusal did
before this existed; the rest hold the step open at a question instead, and
`declared_plan_drift` can never be marked `refuse` at all.

`ipc::JudgeQuestion`, additive on `get_job`'s `JobDetail` beside
`command_waiting`: the refused criterion, the plain question it asked, the
Judge's own `expected` / `produced` / `consequence`, and when it was raised.
Absent is the ordinary case, and every Job read from a Fleet older than 11.3.

`answer_judge` is a new route and `Commands` method, taking `ipc::JudgeAnswered`
— `answer` (`agree`, `disagree_once` or `disagree_always`) and an optional
`note` that rides along for the record. `agree` fails the step exactly as it
would have without this design — since 2 Oct 2026 it sends the step back to a
Drone with the Judge's finding instead, by `restart_step`'s road, with no
change to the wire; either disagree advances it, and
`disagree_always` also stands the criterion down for the repository, so no
later Job is asked about it either. Refused with a 409 where the Job is not
holding a question open.

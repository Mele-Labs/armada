# Protocol 21.3: a Job's Drones, the exited ones too

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

One route, `GET /jobs/:job_id/drones` (`list_job_drones`), answering `JobDrones`: every Drone the
Job has had, each with its step, its state (`running`, `done`, `failed`, `killed`), when it was
spawned and when it left, and its turns and cost. Additive: a new DTO on a new route.

**`GET /drones` is the roster and loses a Drone the moment it exits.** This reads the Job's own
history and the per-Drone spend rows instead, so a stopped Drone stays. The state rule, and why
`killed` is told apart from `failed`, is the operation's note in `crates/ipc/operations/`.

**`ended_at`, `turns` and `cost_micros` are left out where there is nothing**, never nought. A
running Drone has no `ended_at`, and one still in its first invocation has no terminating line yet,
so no turns and no cost. A running Drone's figures come off its transcript and trail it; a stopped
one's are the row the Job's spend is summed from.

There is no task on a row. Fleet runs one Drone per step and nothing joins a Drone to a plan task.

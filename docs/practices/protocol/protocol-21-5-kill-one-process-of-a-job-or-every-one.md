# Protocol 21.5: kill one process of a Job, or every one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Two routes, both answering the Job's `JobSummary`: `POST /jobs/:job_id/processes/:pid/kill`
(`kill_process`) and `POST /jobs/:job_id/processes/kill` (`kill_processes`). Additive: new routes,
no new DTO. Bridge sent both before Fleet served them, and `pending.ts` no longer lists them.

**The pid is a name, not a grant** (#1647). Fleet reads the Job's process tree again at the act and
refuses a pid outside it with `fleet.not_the_jobs_process`, a 409 carrying `pid`. A row that went
stale between the reading and the press meets the same refusal, and the answer is to read again.

**A child is not the Drone.** Killing one ends it and what it started and moves nothing on the
record; killing the Drone's own pid is `kill_drone`, and so is killing every process, plus the
descendants that left the Drone's process group. The operations' notes in
`crates/ipc/operations/` carry the whole rule.

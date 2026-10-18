# Protocol 13.52: running a stopped step's Checks again

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`rerun_checks`, additive: a new command, `POST /jobs/:job_id/rerun_checks`, with no body, answering `JobSummary` (#1105). `Stuck.recourse` gains `rerun_checks`, offered on a Job at `awaiting_repair` whose stopped step failed a mechanical Check. Bridge reads `recourse` as strings, so a Bridge that predates the value draws nothing for it. The request waits for the Checks, which Fleet runs on a task of its own.

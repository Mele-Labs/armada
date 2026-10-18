# Protocol 21.2: a Job's log files say what they weigh and whether they are being written

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

One optional field added to `JobResources`, `logs` — every log file the Job has: its own log,
each Drone's transcript, and each kept Judge brief, as a `LogFile` carrying `kind` (`job`,
`transcript`, `brief`), `path` relative to `records_root`, `bytes` and `being_written`. Additive
by 18.4's reading: a Bridge built before this ignores it, and a Fleet built before this sends
none, which reads the same as a Job with no file yet.

**`being_written` is an open file with write access, never an mtime** (#1648). Fleet asks `lsof`
about the Job's process tree and about Fleet's own pid, because Fleet is the writer: it appends a
Drone's transcript and the Job's log from its own process for as long as that Drone's stdout is
open. Asking the tree alone would read `false` on every live Drone.

**Absent is not zero, and not `false`.** `bytes` is left out for a file Fleet listed and could
not `stat`; `being_written` is left out where `lsof` did not answer. There is no owner on a row:
every one is the Job's own until a sub job exists to name.

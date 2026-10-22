# A Job's Checks

**Part of:** [Job](../session.md).

**An `armada check` run in a slot a Job holds is the Job's run.** The CLI keeps no record of its own, so it tells Fleet when the run begins and when it ends with the log (`report_session_check`, `POST /sessions/checks`, 23.75).
Fleet places the Job by the slot the directory is in, never by what the call says, and **the pool's lease answers first**: the live Job the slot's lease names (a Job or its subagent leases more slots than the one its ledger `slot` row names), else the Job whose standing `slot` row the pool does not deny (`slot_holders::session_in_slot`).
A directory no Job holds is a person's own run and is not recorded, and neither is a Fleet that is not running.

| Where it shows | What |
|---|---|
| A hosted Job's thread | One `check` row per run: the Check's key, its run id, `running`, `passed` or `failed`, and what it was narrowed to (`target`: the test filter, or `--changed` and the packages the change reached; absent where it ran whole). The row is replaced by id as the run ends, and the Job is published whole each time |
| The Job's flight controls | The Checks tile pulses while any `check` row is `running` and rests at none. A press opens the Checks layer over the Job, which lists each run with its key, its `target` and how long it has run, and a press on one opens its run's log |
| The Checks page | A row with the `session` requester (`session_id`, `slot`), drawn with the slot's owner chip. Pressing the thread row opens the run's log over the Job |
| The log | `get_session_check_output`, `GET /sessions/checks/:run/output`, answered as `get_check_output` answers a run's. Kept up to 256 KiB, the newest end when cut |

**While it runs, its log is live and a person can stop it.** `armada check` tells Fleet what it printed in `chunk` calls on the same route, and the Check log panel follows it (`observe_session_check`).
The panel's head holds Stop, which sends `SIGTERM` to the Check's process group and `SIGKILL` after a grace period, and Force stop, which sends `SIGKILL`.
Fleet holds the ask and answers it to the next `chunk`; the process ends its own group and tells `end` as for any run.

A terminal Job's thread is its transcript and carries no `check` row; its runs are on the Checks page.
A run whose `armada check` process is killed stays `running` until its Job ends, when Fleet ends it as failed with "the Job ended" as its log.
Until then the agent is not told the run is gone, and the person reads it as still out: #2698.

# A Job waits on another

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A Job that waits on another*.
**Additive throughout**; the protocol ID moves with the wire files, and nothing is bumped.
The mirror is `packages/protocol/src/session-waits.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `StartSession.after`, `StartAnotherSession.after` (`StartAfter`) | Bridge or an agent sends, Fleet takes | `session` (the Job waited on), `condition` (`StartCondition`, default `ends`), `base` (`StartBase`, default `main`), `needs`. With it `request` is the brief and nothing is sent. 422 `fleet.wait_session_unknown`, `fleet.wait_part_unknown`, `fleet.wait_on_itself`; 409 `fleet.session_closed` for one that ended. Checked before the Job is made |
| `StartCondition` | Both | A tagged union on `kind`: `ends`, `group_done` (`group`), `task_done` (`task`), `pr_pushed`, `pr_lands`. A kind Fleet does not have does not decode |
| `StartBase` | Both | `main` or `branch`, the branch of the pull request of the Job waited on |
| `SessionRecord.waits` (`SessionWait`) | Fleet writes, Bridge reads | `on` (`{id, title}`), `condition`, `base`, `needs`, `brief`, `pull?` (`{number, branch}`, once the Job waited on has one; Fleet picks the first and keeps it), `by` (`you` or `agent`). Present while it waits, absent once it started. Left out when empty |
| `read_session_wait` (`POST /sessions/read_wait`) | Bridge sends | `ReadSessionWait`: `session_id`, `request`. Answers `WaitReading`: `condition`, `base`, `needs?`. `agent_access` No. A reading that cannot be had is the defaults, never a refusal |
| `change_waiting` (`POST /sessions/change_waiting`) | Bridge or the agent waited on sends | `ChangeWaiting`: `waiting_session`, `condition?`, `base?`. Answers `SessionWait`. `agent_access` Yes. 422 `fleet.wait_not_yours`, `fleet.wait_unplaced`, `fleet.wait_part_unknown`; 409 `fleet.wait_over` |
| `start_waiting_now` (`POST /sessions/start_waiting`) | Bridge sends | `StartWaitingNow`: `session_id`. Answers `SessionRecord`. `agent_access` No. 409 `fleet.wait_over` |
| `SessionRow.waiter` (`session.row`, `get_session`) | Fleet writes, Bridge reads | `id`, `at`, `session` (`SessionLink`), `event` (`waits`, `changed`, `started`), `side?` (`waiting`; absent is `waited`), `says?`, `from?`, `to?`, `by?`. In the thread of the Job waited on (`session` is the one that waits), and, for `changed`, also in the waiting Job's own thread with `side` `waiting` |

**Not a wait in the store's sense, a fact on the record.** The wait is kept in `session_waits` (an additive migration: one table and one index, no foreign key) and read into `SessionRecord.waits` on every read of a Job that has not started.
The row stays after the start, with `started_at`, so a restart cannot start it twice.

**The event stream is neither better nor worse.** A wait is one `session.changed` for the waiting Job and one `session.row` for the Job waited on, then one of each when it starts.
Fleet reads a wait against a Job only when that Job publishes `session.changed`, so a Job nobody waits on costs nothing new.

**Skew.** The IDs differ, so a Bridge before this is refused regardless. A Bridge that draws a record with `waits` it does not know shows a blank Job.

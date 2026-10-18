# Protocol 23.75: a Job's Checks

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A Session's Checks*. **Additive, with one loosening**: two operations, a requester kind and a thread row, and `ManifestCheckRow`'s Job fields become optional.

| Where | Carries | Notes |
| --- | --- | --- |
| `report_session_check` (`POST /sessions/checks`) | `SessionCheckCall`: `act` (`start` or `end`), `cwd`, `name`; on `end` also `run`, `outcome`, `took_ms`, `log`. Answers `SessionCheckAnswer`: `run?` | `armada check` is the caller. No `run` where no Job holds the slot |
| `get_session_check_output` (`GET /sessions/checks/:run/output`) | Answers `CheckOutput` | Keyed by run id, since a Session's run has no Job |
| `Requester` kind `session` | `session_id`, `slot` | |
| `SessionRow` kind `check` | `name`, `run`, `state`, `target?` | Replaced by id as the run ends. `target` is what the run was narrowed to, absent where it ran whole; optional on `SessionCheckCall` too, where a `--changed` run tells it on `end` |
| `ManifestCheckRow.job_id`, `job_handle`, `job_title`, `step` | Now optional | Absent on a Job's run, which carries `session_id` and its run id in `asked_run_id` |

**Skew.** A Bridge before 23.75 reads a row with no `job_id` as a Job row with a missing id, so it draws that row badly; it never asks for the new routes. A Fleet before it sends neither the rows nor the thread row.

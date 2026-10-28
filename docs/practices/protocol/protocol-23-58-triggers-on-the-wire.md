# Protocol 23.58: Triggers on the wire

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/trigger.md (deleted)`. **Additive only**: four operations, one event, the DTOs of `ipc::triggers` and one optional field on `JobDetail`. 23.52 is a job's name.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_triggers` (`GET /triggers`) | nothing, or `?manifest_id=` | `TriggerList`: `triggers` (`name`, `when`, `workflow?`, `step?`, `runs`, `block`, `repair`, `level`, `file`, `skipped?`, `overrides`) and `left_out` (`level`, `file`, `said`). One object, because it starts as one. `Yes` |
| `get_trigger` (`GET /triggers/definition`) | `?when=&step=&name=&source=` | `TriggerDefinition`: the YAML text beside the identity, `level` and `overridden_by?`. 422 `fleet.no_such_trigger`. `Yes` |
| `save_trigger` (`POST /triggers/save`) | `SaveTrigger`: `scope` (`repository` or `machine`), `definition`, `overwrite?` | `TriggerSaved`, with `runs_from?`, `waits_for_main` and `skipped?`. 422 `fleet.trigger_unfit`, `fleet.trigger_name_not_a_name`, `fleet.trigger_exists`; 500 `fleet.trigger_unwritable`. `Bridge only` |
| `remove_trigger` (`POST /triggers/remove`) | `RemoveTrigger`: `scope`, `when`, `step?`, `name` | `TriggerRemoved`. 422 `fleet.no_such_trigger`. `Bridge only` |
| `job.trigger_changed` (event) | `JobTriggerChanged`: `job_id`, `trigger`, `at` | The row whole, on each state a firing reaches. Bridge re-reads the open Job |
| `JobDetail.triggers` | `JobTrigger`: `name`, `when`, `step`, `level`, `state`, `skipped?`, `exit_code?`, `started_at?`, `ended_at?`, `log_at?` | `pending` is a frozen Trigger no moment has reached. Absent from a Fleet before 23.58 and where empty |

**The event stream: slightly worse, and bounded.** At most two messages a firing, to open and to end, on the one drop-oldest channel.
A Bridge that missed some re-reads `get_job`.
The rate is `[broadcast-capacity]`'s to measure.

**`log_at` is an instant and not a line number.** The Job's log has no numbers, so the log line for a firing is stamped with the firing's own end, and `get_job_log` finds it by that `at` and its `trigger` field.
Bridge's half is `packages/protocol/src/triggers.ts (deleted)`, written by hand like the rest.

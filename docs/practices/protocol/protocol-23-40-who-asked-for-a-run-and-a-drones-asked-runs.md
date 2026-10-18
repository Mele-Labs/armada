# Protocol 23.40: who asked for a run, and a Drone's asked runs

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 6 Oct 2026: a Drone's `run_checks` left no record, and no Check run shown anywhere said who asked for it. `../concepts/manifest.md`, *Who asked for a run*.

**Additive only.** One new DTO set, and optional or defaulted fields; a 23.36 Bridge reads past all of it.

| Field | Lands on | Carries | Absent |
| --- | --- | --- | --- |
| `Requester` | new, `ipc::Requester` | `kind` (an opaque string: `gate`, `drone_task`, `drone_step`, `merge_line`, `outside`) and `job_id`, `step`, `task_id`, `drone_id`, `branch` where the kind needs them | Never on the wire from 23.40; a reader treats none as `outside` |
| `requester` | `CheckRun` (every `StepDetail.check_runs` row, stamped `gate`) | who asked | A Fleet before 23.40 |
| `requester` | `ChecksUnderway`: `gate` on `checking`, the Drone on `dry_run` | who asked | As above |
| `requester` | `RunUnderway`, `RunRecord`, `CheckoutRunUnderway`, `CheckoutRunRecord`, `CheckoutVerify` | `outside`; on `RunUnderway` and `RunRecord`, the Drone (`drone_task` or `drone_step`) where a Drone started the run through the agent's door | A record written before 23.40 reads `outside` |
| `requester` | `MergeLineCheck` | `merge_line`, with the entry's `branch` | As above |
| `started_at` | `MergeLineCheck` | When the runner began the Check, an `Instant`, written by `armada land` into the line's state as the Check goes `running` | A Check still waiting, and a line state from before the field |
| `handle` | `Requester` | What a person calls the Job; a Drone's handle | Where Fleet does not know it |
| `asked_runs` | `StepDetail`, `RunList` | `AskedRun`: `id`, `requester`, `attempt`, `started_at`, `finished_at`, `state` (`running`, `passed`, `failed`, `stopped`, `lost`), `checks`, `narrowed`, `only_check`, `logs` | No run was asked |

**One read across Jobs, `list_manifest_checks`.** `GET /manifest/checks?manifest_id=` answers `ManifestChecks { rows, total, truncated? }`: every gate row and every asked run of the repository's Jobs (every served repository's where none is named), newest first, at most 200. A `ManifestCheckRow` is `source` (`gate` or `asked_run`), `requester`, `job_id`, `job_handle`, `job_title`, `step`, `attempt`, `group?`, `name`, `state`, `started_at?`, `ended_at?`, `took_ms?`, `logs[]` (`{ check, kept }`) and `asked_run_id?`.
A gate row has no start or duration, only the write of its ruling as `ended_at`.
A log opens through `get_check_output` on the row's `job_id` and `kept`, which now also resolves an asked run's logs.
Merge-line Checks and checkout runs are not in it.
`requester.handle` is the Job's handle, which is the Drone's handle: a Drone has no other name than its id.

**No version bump for the Drone stamp.** A `start_run` a Drone makes through the agent's door carries `drone_task` or `drone_step` where it carried `outside`: the same field and the same kinds, so a Bridge that follows a Drone requester already draws it.
Fleet places the caller by its connection, and a caller it cannot place stays `outside`.

**An asked run is never a `CheckRun`**, so a dry result cannot be read as a gate's pass. It is its own row in `asked_runs`; a `running` row begun by a Fleet that is gone reads `lost`.

**One migration, V113**, `asked_runs`, pointing at `jobs`.
**Bridge's half**, mirrored by hand in `packages/protocol`: `requester.ts` holds `Requester`, `AskedRun`, `requesterOf` and the kind spellings; `requester` and `asked_runs` are optional there so a fixture and a Fleet before 23.40 still type.

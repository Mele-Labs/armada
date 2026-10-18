# A Job's plan and its file touches

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Oct 2026. `docs/concepts/session.md`, *The plan*. **Additive.** No protocol number moves; the protocol ID changes with the wire it hashes. Types live in `crates/ipc/src/session_plan.rs` and `packages/protocol/src/session-plan.ts`, apart from `SessionRecord`'s file so a parallel change to the record merges cleanly.

| Where | Side | Change |
| --- | --- | --- |
| `record_plan` (`POST /sessions/plan`, on the agent's door) | The agent calls | `RecordPlan`: `groups[]` (`id?`, `title`, `agent`, `paths[]`, `tasks[]` of `id?` and `title`), `sketch?` (`title`, `scene`), `session_id?`. Answers the receipt `{version, groups[]}` with each id Fleet gave. Refused 422 `fleet.plan_refused` with every reason, `fleet.plan_unnamed` where no Job is placed |
| `approve_plan` (`POST /sessions/plan/approve`) | Bridge calls | `ApprovePlan`: `session_id`, `version`. Answers the `SessionRecord`. 409 `fleet.plan_version_stale`, `fleet.plan_none` |
| `send_plan_changes` (`POST /sessions/plan/changes`) | Bridge calls | `SendPlanChanges`: `session_id`, `version`, `marks[]`, `approve?`. `PlanMark` is tagged by `kind`: `rename`, `remove`, `add_task`, `add_group`, `move_task`, `subagent`, `note`. 422 `fleet.plan_mark_refused` |
| `SessionRecord.plan` | Fleet writes, Bridge draws | `SessionPlan`: `version`, `recorded_at`, `approved`, `approved_at?`, `scope[]`, `groups[]`, `changes?`, `sketch?`. A task is `open`, `working`, `handed_in`, `done` or `failed` and carries `seen?`; a group `worked_at?` |
| `SessionRecord.touches` | Fleet writes, Bridge draws | `FileTouch[]`, oldest first, at most 60: `by?` (`agent`, `id`), `path`, `act` (`write`, `edit`, `remove`), `at`. Left out while empty |
| `WaitingSource` `plan` | Fleet writes, Bridge matches | A recorded plan not yet approved; id `plan:<version>` |
| `SessionRow` `plan`, `plan_changes` | Fleet writes, Bridge draws | `plan`: where the plan was recorded. `plan_changes`: `version`, `approved?`, `lines[]` of `verb`, `place`, `from?`, `text?` |
| `SessionFact` `touched` | The mod writes | Gains `by?` (the subagent's `agentId`) and `act?` (`write` or `edit`) |

**Optional members are left out, never sent as `null`.** A Bridge that predates this ignores the members it does not read, and draws a `plan:` waiting item by its text; a `plan` or `plan_changes` row it does not know is one it skips. A Fleet that predates sends none, and a mod that predates `by` and `act` tells a touch with neither, which reads as the Job's own edit. Migration `20261016T1500Z-session_plan`: five tables, none read by anything else.

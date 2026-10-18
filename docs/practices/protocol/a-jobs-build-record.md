# A Job's build record

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Oct 2026. `docs/concepts/session.md`, *The build record*, and `.claude/decisions/2026-10-14-a-session-builds-a-ring.md`.
**Additive only**: one optional field on `SessionRecord`, new fact kinds, new timeline kinds and the types they carry.
No operation, event kind or error is new.
Bridge's half is `packages/protocol/src/session-build.ts`, `sessions.ts` and `timeline.ts`, written by hand.
There is no number to take: the protocol ID moves with the wire files, and `armada need` has nothing to order.
The migration `20261014T2100Z-session_build.record` has a name and not a number, and is additive.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.build` (`session.changed`, `get_session`, `list_sessions`) | Fleet writes, Bridge draws | `SessionBuild`: `phase`, `pieces[]`, `stops[]`, `forecast[]`, `landed_at?`, `quiet_since?`, `stalled`, `workers[]`. Left out for a Job that has done nothing Fleet observed |
| `SessionBuild.phase` | Fleet writes, Bridge matches | `scope`, `plan`, `implement`, `road`, `landed` |
| `SessionBuild.pieces[]` | Fleet writes, Bridge matches `piece` | `BuildPiece`: `id`, `text`, `piece` (`solid`, `hollow`, `scaffold` or `ghost`), `working?` and `by?`, the writing subagent's label in the form `workers` uses (`Type: what it was asked`), left out for the Job's own tasks |
| `SessionBuild.stops[]`, `forecast[]` | Fleet writes, Bridge matches `id` | `BuildStop`: `id` (`walk`, `pushed`, `pr`, `checks` or `auto_merge`) and `at?`, left out for a stop not yet reached |
| `SessionFact` `tasks`, `plan_left`, `committed`, `pushed` | The mod writes | `tasks` carries `ReportedTask`: `id`, `text`, `status` (`pending`, `in_progress` or `completed`) and `by?`, the `agentId` of the subagent that wrote the task, absent for the Job's own (mod 0.3.24). `committed` and `pushed` carry `branch?` |
| `TimelineKind` `session.walked`, `session.committed`, `session.pushed` | Fleet writes, Bridge draws | Strict. A walk's detail is the page's title, the others' the branch |

**Every closed set here is a plain enum Bridge matches on**, as `SessionStanding` is, because no registry declares them.
**The arrays are always sent**, empty when they hold nothing, so a reader never tests for them.
An unreached stop has no `at`, and none is sent as `null`.

**Subagents' tasks, 15 Oct 2026. Additive:** `ReportedTask.by?` and `BuildPiece.by?`, both optional and left out when absent, and the migration `20261015T2000Z-session_build.task_worker` adding a nullable `made_by` to `session_build_tasks`.
A row kept before it reads as the Job's own tasks.
A Bridge that predates `by` ignores it and draws every piece as the Job's.
A mod before `0.3.24` tells no subagent's task, so the ring lacks them.

**The agent's tasks on Progress, 17 Oct 2026. Additive:** `BuildPiece.working?`, true where the agent's task tool has the task in progress and left out otherwise.
Fleet already kept the status (`session_build_tasks.status`), so there is no migration and no new event: the row is published when the list changes, as before.
Bridge's `SessionWork.tasks` is these pieces for a Job with no recorded plan, in the order made: `solid` and `hollow` are done, `working` is working, the rest are open.
A Bridge that predates `working` lists every unfinished task as open.

**Skew.** A Bridge that predates this ignores a `build` it does not read.
A Fleet that predates it sends none.
A mod before `0.3.23` tells none of the new facts, so its Job's `build` holds only what Fleet heard otherwise, and the Job is marked `mod_out_of_date`.
A Fleet that predates the facts refuses them, and the mod goes on.

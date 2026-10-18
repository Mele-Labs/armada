# Sessions absorb Jobs, group 2: the Job surface leaves the wire

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`.
**One rename, and removal otherwise**: no operation, event kind or route is added.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused.
`crates/ipc` and `packages/protocol` are edited together.

**The rename.** Autopilot's figures count Sessions and now say so. The store keeps the settings key it had; Fleet maps it.

| Where | Side | Change |
| --- | --- | --- |
| `Autopilot.job_cap` | Fleet writes, Bridge draws | `session_cap` |
| `Autopilot.jobs_started` | Fleet writes, Bridge draws | `sessions_started` |
| `AutopilotSettings.job_cap` | Bridge sends, Fleet reads | `session_cap` |
| `AutopilotProposal.job_cap` | Fleet writes, Bridge draws | `session_cap` |
| `AutopilotCap` `jobs` | Fleet writes, Bridge draws | `sessions` |

**Operations removed (44).** Each has no route, no MCP tool and no Bridge caller:

| Kind | Operations |
| --- | --- |
| Queries | `get_job`, `get_job_events`, `get_job_log`, `get_job_retro`, `list_jobs`, `list_job_board`, `get_diff`, `get_usage`, `get_manifest_spend`, `get_activity_feed`, `list_alerts`, `list_branches`, `list_reports`, `list_reviews`, `observe_job_log` |
| Commands | `kill_job`, `forget_job`, `file_report`, `delete_branch`, `reclaim_worktree` |
| Events | `job.created`, `job.state_changed`, `job.step_advanced`, `job.files_changed`, `job.judging`, `job.checking`, `job.dry_run`, `job.asking`, `job.command_waiting`, `job.forgotten`, `job.landed`, `job.remarks_changed`, `job.paused`, `job.resumed`, `job.plan_changed`, `job.trigger_changed`, `job.addition_changed`, `drone.spawned`, `drone.exited`, `evidence.submitted`, `proposal.moved`, `run.finished`, `commander.changed`, `commander.card` |

**Wire types removed.** The DTO files of the Job: `job`, `detail`, `work`, `work_plan`, `drones`, `history`, `journal`, `judged`, `asked`, `asking`, `approving`, `approval_ask`, `proposing`, `waiting`, `turn`, `attempt`, `added_steps`, `walk_notes`, `trigger_holds`, `piloting`, `usage`, `resources`, `report`, `reclaimed`, `breakage`, `alerts`, `capacity`, `commander`, `commander_card`, `commander_repair`, `job_issues`, `manifest_checks`, `overlap`, `raising`, `remarks`, `showing`, `explaining`; the MCP tool modules under `ipc::mcp`; and the Job vocabularies in `ipc::enums` (`JobStatus`, `StepState`, `QueuedReason`, `Resumption`, `Recourse`, `DronePresence`, `BudgetHold`, `Urgency`, `Side`, `Origin`, `TopLevelOrigin`, `TaskTier`, `JudgeVerdict`, `CriterionSource`, `DependencyDirection`, `TaskState`, `GroupState`), `JobId` and `DroneId`.

| Where | Side | Change |
| --- | --- | --- |
| `Resync.jobs` | Fleet removes, Bridge stops reading | A resync carries the ID and the cursor, and each surface re-reads its own state |
| `Event::about` | Fleet writes | `Option<String>`: an event that names no subject says so |
| `BayHolderKind` `job`, `ClashSide` `job`, `SlotHolderKind` `job`, `SlotHolding::Job` | Fleet writes the other arms only, Bridge reads them | A Job holder is not drawn. A lease record written before still reads as a holder id |
| `SessionAttachment` kind `job` | Bridge stops sending, Fleet stops reading | An old attachment is skipped |
| `ServerState.job_id`, `ServerState.for_review`, `StartServer.job_id`, `ServerOpened.job_id`, `StartedBy` `drone` | Fleet removes, Bridge stops reading | A server belongs to a checkout or a Session |
| `Requester` `job_id`, `step`, `task_id`, `drone_id`, `handle` and the `gate` and `drone` kinds | Fleet writes `merge_line`, `session`, `outside` | A record written before still reads as its `kind` |
| `WireError.job_id`, `drone_id`, `step_id` | Fleet removes, Bridge stops reading | A fault names no Job. The three were optional and skipped when empty |
| `TrackedIssue.job_id`, `ReviewPullRequest`, `ReviewDispatched` | Fleet removes, Bridge stops reading | A review is a Session's |
| `FixesMain`, `FixesMainState`, `FixMain` | Fleet removes, Bridge stops reading | A fix is a Session the hub names by `fixing_session` |
| `JobRetro` | Fleet removes, Bridge stops reading | `Retro` (`get_session_retro`) replaces it, with the Job-shaped record rows (`RecordCheck`, `RecordNotMet`, `RecordAsked`, `RecordWaited`) gone |
| `Retro.session`, `Lesson.session` | Fleet writes, Bridge reads | `RetroSession`, no longer `Option`: every retro and lesson belongs to a Session |
| `RunSheet`, `RunRecord`, `RunUnderway` and the Job runs | Fleet removes, Bridge stops reading | `CheckoutRunRecord` and `observe_checkout_run` serve a checkout's runs |
| `ChecksUnderway`, `CheckUnderway` | Fleet removes, Bridge stops reading | `Session` runs have `observe_session_check` |
| `DeclaredJudge.gaming_check`, `gaming_patterns`; `CheckRun.group`, `group_attempt` | Fleet removes, Bridge stops reading | |
| `ManifestProposals.caps` (`StatedCaps`), `WorkflowStep.may_dispatch_jobs`, `WorkflowStep.drone_per_task` | Fleet removes, Bridge stops reading | A step no longer declares what a Drone may do |
| `StudioNodeContent` `job`, `DispatchStudioDraft` | Fleet stops writing, Bridge stops drawing | A Job node on an old Studio is dropped on read. The store's CHECK keeps the word |
| `HelmScreen` `board` | Bridge stops sending, Fleet stops reading | The Board is gone. A context naming it no longer decodes; `overview` is the screen that replaced it |
| `WorktreeHeld`, `HeldReason`, `WorktreesHeld`; `reclaim_worktree`, `delete_branch` | Fleet removes, Bridge stops calling | Slot cleanup is the pool's |
| `AlwaysAllowScope`, `AnswerCommand`, `CommandAnswer`, `CommandInFlight`, `Reach`, `RemoveAllowedCommand`, `SetWhenBlocked`, `WhenBlocked`, `SetModel`, `SetCommanderModel` | Fleet removes, Bridge stops sending | A repository's allowed commands are `get_repository_allowed_commands` and `remove_repository_allowed_command` |
| `ReclaimedSaved` | Fleet writes, Bridge reads | Moves from `reclaimed.rs` to `holding.rs`, shape unchanged; `SlotReleased.saved` names it |
| `WalkTold` | Fleet writes, Bridge reads | Moves from `showing.rs` to `sessions.rs`, shape unchanged; `window_failed` answers with it |

**Left, because a surface other than a Job reads it.** The store's Job tables stay in the schema and nothing reads or writes them; no migration drops them.
A `holder job <id>` already in a lease record still reads, so no slot is stranded by one.
`docs/capabilities/drone-per-step.md (deleted)` describes what is gone and is for the docs pass.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
No reader needs to tolerate the old shape.
The lifeboat's `GET /v0/jobs` and `POST /v0/jobs/:id/kill` rows name Jobs and the lifeboat is not built; this change does not touch them.

**The event stream.** This change makes the open risk below **better in kind**: it removes the producers of `job.*`, `drone.*`, `commander.*`, `evidence.submitted`, `proposal.moved` and `run.finished`, the busiest kinds Fleet published, so the stream carries far fewer events and a minimised Bridge has less to fall behind on.
It adds no bounded broadcast and no "you missed N events" message; `Missed` and the drop-oldest channel are as they were, and the number is still unmeasured.

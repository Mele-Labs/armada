# Sessions absorb Jobs, group 1: the Job arms and piloting a Job leave the wire

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`, the retired Pilot concept and `docs/concepts/session.md`.
**Removal only**: no field, operation or event kind is added, so every change below is a Bridge or Fleet that still sends or reads it.
The protocol ID moves with the hash of `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused.
Where a wire type had a Job arm beside a Session arm, the Job arm is gone and Fleet stops producing the value.

| Where | Side | Change |
| --- | --- | --- |
| `take_over` (`POST /jobs/:job_id/take_over`), `get_handoff` (`GET /jobs/:job_id/handoff`) | Fleet removes, Bridge stops calling | Operations, routes, MCP tools and the DTOs `TakeOver`, `PilotOutcome`, `HandoffBundle`, `HandoffWorktree`, `StoppedOn`, `DroneNarrative` |
| `StartSession.pilot` (`PilotFrom`) | Bridge stops sending, Fleet stops reading | The Session started on a Job's worktree, with the Job's slot and branch handed to it |
| `SessionRow` kind `handoff` (`HandoffStep`, `HandoffPlan`) | Fleet stops writing, Bridge stops drawing | The first row of a piloted Session's thread |
| `TimelineKind` `job.created`, `job.queued`, `job.started`, `job.step`, `job.check`, `job.judged`, `job.files`, `job.read`, `job.ran`, `job.asked`, `job.answered`, `job.landed`, `job.failed` | Fleet stops writing, Bridge stops drawing | Fleet appends no entry for a Job. `slot.leased` and `slot.released` are a Session's alone |
| `TimelineSubject` `job` | Fleet stops writing, Bridge stops drawing | The subject is `session`, `pull_request` or `main` |
| `MergeLineHub.fixing`, `HubPullRequest.job`, `HubMerged.job`, `MainMerge.job`, `HubJob` | Fleet removes, Bridge stops reading | `MergeLineHub.fixing_session` and `MainMerge.session` are the owners |
| `HolderKind` `job` (`FixingElsewhere.holder`, `ClaimHolder`, `Holder`, `Ownership`) | Fleet writes `session` only, Bridge reads `session` only | A Job's claim in the central store is not drawn |
| `ClaimPullRequest.job_id`, `PullRequestClaimed.holder_kind` | Bridge stops sending, Fleet removes | `claim_pull_request` is a Session's; `holder_id` is its id |
| `ThoughtKind` `job`, `ThoughtTarget.kind` `job` (`read_thought`, `record_thought_sent`) | Fleet stops offering, Bridge stops sending | A thought routes to a Session or to none |
| `TagKind` `job`, `Mention` Job arm, `SessionAttachment.job` (`TaggedJob`) | Bridge stops sending, Fleet stops reading | The composer offers no Job after `@` |
| `SessionSummary.jobs` | Fleet removes | The Jobs a Session holds that have not finished |
| `who_owns`, `armada need`'s `mine` and `ahead` | Fleet writes Session holders only | A Job holder is omitted, not renamed |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. Nothing here needs a reader that tolerates the old shape.

**Left for group 2.** What this group left is removed in *Sessions absorb Jobs, group 2*, below.

**The event stream.** This change makes the open risk below no better and no worse in kind: it removes the Job producers of timeline entries, so the stream carries fewer events.
It adds no bounded broadcast and no "you missed N events" message, so a minimised Bridge against several Drones still has nothing pushing back.

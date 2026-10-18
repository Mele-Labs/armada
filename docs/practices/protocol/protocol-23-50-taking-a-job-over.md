# Protocol 23.50: taking a Job over

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

the retired Pilot concept. *`take_over` and `get_handoff` are removed, see "Sessions absorb Jobs, group 1".* **Additive only**: five operations, the DTOs of `ipc::piloting`, one optional field on `JobSummary` and one migration. 23.48 is the pull-request acts and 23.49 is the session host's.

| Operation | Carries | Notes |
| --- | --- | --- |
| `take_over` (`POST /jobs/:job_id/take_over`) | `TakeOver`: `outcome` (`take_over` or `restart_step`), `session_id?`; or no body | Answers the Job's `JobSummary`, `piloted`. `Helm only`, asked as `destructive` |
| `get_handoff` (`GET /jobs/:job_id/handoff`) | nothing | Answers `HandoffBundle`: the Job's detail, its history, its evidence, the declared plans and the worktree's changed files marked against them, the worktree path and branch, where it stopped and the Drone's `narrative?`. `agent_access` `Yes` |
| `submit_for_verification` (`POST /jobs/:job_id/submit_for_verification`) | nothing | Held for the whole run. `Helm only`, `pushes to shared` |
| `attest_complete` (`POST /jobs/:job_id/attest_complete`) | `PilotNote`: `note?`; or no body | `Helm only`, `pushes to shared` |
| `close_as_superseded` (`POST /jobs/:job_id/close_as_superseded`) | `PilotNote`: `note?`; or no body | `Helm only`, `destructive` |

**`JobSummary.piloted`** is `Piloted`: `reason`, `session_id?`, `since`, and `exit?`, `ended_at?`, `note?` once the pilot ended. Absent on a Job nobody piloted, and on every row from a Fleet before 23.49. **`exit` reads `attested` on a Job a person completed**, and a Bridge draws that apart from a Job that passed its gates. Assist is not an `outcome`: a body naming it does not decode.

**Refusal codes**, all 409: `fleet.not_pilotable`, `fleet.already_piloted`, `fleet.not_piloted`, `fleet.pilot_busy`, `fleet.no_step_to_restart`, `fleet.nothing_to_verify`, `fleet.nothing_to_submit` and `fleet.steps_not_advanced`; and `fleet.paused` where the Job's worktree is parked. The migration adds `job_pilots`. Bridge's half is `packages/protocol/src/piloting.ts (deleted)`, written by hand like the rest.

# Autopilot: a repository works its own backlog

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#2092`, `docs/concepts/autopilot.md`. **Additive only**: nine operations, one event kind, the DTOs of `ipc::autopilot`, and `origin` gains `autopilot_picked` (*Picked by Autopilot*), a string on the wire like the rest. Bridge's half is `packages/protocol/src/autopilot.ts`, written by hand.

| Operation | Carries | Notes |
| --- | --- | --- |
| `list_autopilots` (`GET /autopilots`) | nothing | Answers `AutopilotList`, every repository's, stopped ones included |
| `get_autopilot` (`GET /autopilots/:repository`) | nothing | Answers `Autopilot`: the settings, `state`, `capped_by`, the figures `spent_micros`, `jobs_started` and `at_work` read now, and `picks`. 409 `fleet.no_autopilot` |
| `propose_autopilot` (`POST /autopilots/:repository/proposal`) | `ProposeAutopilot`: `sentence`, `local_now?` | Answers `AutopilotSettings`. One model call; **starts and keeps nothing**. 500 `fleet.autopilot_not_read` |
| `start_autopilot` (`POST /autopilots/:repository`) | `AutopilotSettings`: `instruction`, `at_once`, `until?`, `spend_cap_micros?`, `job_cap?` | A 201 with `Autopilot`. 409 `fleet.autopilot_running`, 422 `fleet.autopilot_unsettable` |
| `pause_autopilot`, `resume_autopilot`, `stop_autopilot` (`POST /autopilots/:repository/pause`, `/resume`, `/stop`) | nothing | Answer `Autopilot`. 409 `fleet.autopilot_not_running`, `fleet.autopilot_not_paused`, `fleet.autopilot_stopped`. Stop closes each picked Job that never ran and marks its pick `withdrawn` |
| `dispatch_set_aside`, `dismiss_set_aside` (`POST /autopilots/:repository/picks/:pick_id/dispatch`, `/dismiss`) | nothing | Answer `Autopilot`. Dispatch starts a Job on the issue, as a sure pick does. 409 `fleet.pick_not_set_aside` |
| `autopilot.changed` (event) | `Autopilot` | On each act, each pick, a cap reached, and a pass that finds the figures moved. Whole, so a reconnecting Bridge replaces what it holds |

**A Job a sure pick starts is an ordinary `job.created` and `job.state_changed`**, its last move by `fleet` rather than `human`: no new kind says a Job skipped the gate, since the origin on its row does.

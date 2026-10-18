# Autopilot starts Sessions, not Jobs

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/autopilot.md`, `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`. A pick starts a Session briefed on its issue, and the figures count Sessions. **One field is renamed and none is added.** The protocol ID moves with `crates/ipc/src/autopilot.rs`, and there is no number to take. The migration `20261017T0900Z-autopilot.started_session` has a name and not a number, and is additive.

| Where | Side | Change |
| --- | --- | --- |
| `AutopilotPick.job_id` | Fleet writes, Bridge reads | Replaced by `session_id`: the Session the pick started, opened in the Cockpit. A pick of an earlier build carries none |
| `Autopilot.at_work`, `spent_micros` | Fleet writes, Bridge draws | Count the Sessions it started: live, at work, and not waiting on a person, and their reported cost. Same fields, same types |
| `Autopilot.jobs_started`, `job_cap`, `AutopilotCap` `jobs` | Fleet writes, Bridge draws | Renamed in *Sessions absorb Jobs, group 2*, below |
| `AutopilotPickState` `withdrawn` | Fleet writes | A Session closed at Stop because it had never run, where it was a Job killed |
| `dispatch_set_aside` | Bridge calls | Starts a Session at once, where it drafted a Job to the approval gate. The route and the answer are unchanged |

**Skew.** A Bridge before this reads a pick with no `job_id` and cannot open it; the IDs differ, so the two are refused regardless. **The event stream is untouched** beyond the `autopilot.changed` each Autopilot already publishes. The lifeboat is untouched.

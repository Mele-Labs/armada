# Autopilot's reports box and weekly cap: `on_autopilot.reports`, `weekly_cap_percent` and `weekly`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/autopilot.md`.
The person checks whether Autopilot works the bugs agents reported, and sets a percent of the account's week at which it pauses.
**Additive only**: one field on `AutopilotWork`, one on `Autopilot` and one on `AutopilotSettings`, one word on `AutopilotCap`, and a migration with a name and not a number.
The protocol ID moves with `crates/ipc/src/autopilot.rs`, the operation files and `packages/protocol/src/autopilot.ts`, and there is no number to take.
Bridge's half is `packages/protocol/src/autopilot.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `AutopilotWork.reports` | Bridge sends, Fleet keeps | A boolean. Absent reads as false, so a body from before it, and a record kept before it, read as off |
| `AutopilotSettings.weekly_cap_percent`, `Autopilot.weekly_cap_percent` | Bridge sends, Fleet keeps and writes | A whole percent, optional. Absent is no cap and is left off the wire |
| `AutopilotCap` | Fleet writes in `capped_by`, Bridge reads | A fourth word, `weekly` |
| `update_autopilot` | unchanged route | Takes `weekly_cap_percent` as it takes the other caps, and keeps the boxes as it did |
| `propose_autopilot` | unchanged route | Answers no weekly cap and `reports` false: a sentence sets neither |

**The weekly cap is a reading and never a budget.** Fleet compares the account's `seven_day` window `percent_used` with the cap.
No reading never reaches it: an account with no such window, and a window whose `resets_at` has passed, are not readings.
A start or an update with the reading already at or past the cap is a 422 `fleet.autopilot_unsettable`, so raising the cap above the reading is what lets a Resume through.

**Reports are offered on the armada repository's Autopilot only**, whatever Bridge draws, and only with the box on.
A pick of one carries `report:<id>` as its `issue_url`, which is no web address.
Migration `20261103T1000Z-autopilot.reports` adds `on_reports` and a nullable `weekly_cap_percent` to `autopilots`.

**Skew.** The IDs differ, so a Bridge from before is refused regardless.
The event stream is untouched beyond the `autopilot.changed` each Autopilot already publishes, which now carries the two fields.
The lifeboat is untouched.

# The pull request a plan's group lands in: `lands_in`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 29 Oct 2026. The person approving a plan sees whether it means one pull request or two.
**Additive**: one field in, one field out, three small types; no operation, route or event kind.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/session-plan.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `PlanGroupInput.lands_in` (`record_plan`) | Agent sends | A label, at most 24 characters (`MOST_LANDS_IN_CHARS`), such as `PR 1`. Absent or blank is `PR 1`. A longer one is refused 422 naming `groups[n]` |
| `SessionPlanGroup.lands_in` (`SessionRecord.plan`) | Fleet answers | `PlanLandsIn`: `label`, and `pull_request` (`PlanLandsInPr`: `number`, `title`, `state` of `open`, `draft`, `merged` or `closed`) once one is bound. Sent on every group |

**The label is the agent's; the pull request is Fleet's.** Fleet binds from the Job's ledger in this order, and every group that shares a label reads the one pull request: a group handed to its own Job (`started_to` with `detail.group`) takes that Job's `pr` rows; a `pr` row whose first version has `from_task` takes the label of the group holding that task; a `pr` row with neither takes the first label in plan order that nothing has bound.
A later row replaces an earlier one for the same label within a step.
Title and state are the row's `detail.title` and `detail.state`; a row with neither reads as open, or merged where it is spent.
A row given back binds nothing.
**Recording again** keeps what the new recording sends, as `gives` does: a label left out reads `PR 1`.
**Store**: `session_plan_groups.lands_in`, an additive migration; a group kept before reads `PR 1`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream is neither better nor worse.** `session.changed` already carries the plan whole; it grows by a label and an optional pull request on each group.
The unbounded sink and the missing resync message are untouched.

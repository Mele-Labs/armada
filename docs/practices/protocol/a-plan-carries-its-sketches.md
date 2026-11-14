# A plan carries its sketches: `sketches` on `record_plan` and `SessionPlan`, and `in_plan` on `JobArtifact`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. A plan names the sketches its Job drew with `show_sketch`, and his one Approve of the plan settles them. `docs/concepts/artifact.md`, *A plan's sketches*.
**Additive only**: five fields, each serde default and left out when empty. No operation, route, event kind or error code is new.
Bridge's half is `packages/protocol/src/session-plan.ts` and `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not read them, and native draws the page later.

| Where | Side | Carries |
| --- | --- | --- |
| `RecordPlan.sketches`, `PlanGroupInput.sketches` | The agent sends, Fleet reads | Titles of sketches the Job holds from `show_sketch`: the plan's for the whole, a group's beside its section. A title it holds none for is 422 `fleet.plan_refused`, naming up to twenty it holds |
| `SessionPlan.sketches`, `SessionPlanGroup.sketches` | Fleet writes, Bridge reads | Each named sketch as `PlanSketch` `{ id, title }`, `id` the ledger's `sketch:<title>`, as the inline `sketch` is |
| `JobArtifact.in_plan` | Fleet writes, Bridge reads | The plan version a sketch goes with: the pending one naming it while it asks, then the one whose Approve settled it. `item` is `plan:<version>` while it waits with the plan |
| `detail.in_plan` on an artifact of form `sketch` | Fleet writes, Bridge reads | The version, as text, whose Approve settled it, beside `settled` `approved` and `settled_at`. A new drawing does not carry it |

**Kept as titles, one a line.** `session_plan.sketches` and `session_plan_groups.sketches` are new columns, an additive migration, so a store from before reads every plan as naming none.

**The event stream.** An Approve now rewrites each sketch it settles, and each rewrite publishes `session.changed` as `answer_sketch` does.
That is one event more per named sketch on a press he makes, so the unmeasured sink risk is a little worse and stays unmeasured.

**Skew.** The wire surface changed, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.

# What a workflow lands in: `WorkflowSummary.lands`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 25 Oct 2026. The Job view's right column ends in the Landing, which draws differently by kind.
**Additive**: one field on an existing type and one enum; no operation, route or event kind.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/setup.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `WorkflowSummary.lands` | Fleet answers, on `list_workflows` | `Lands`: `pull_request`, `report`, `spike` or `document`. Sent on every workflow |

**Fleet serves the word resolved.** A definition's own `lands:` where it declares one, else `pull_request` where a step `delivers` and `document` otherwise: `core_model::Lands::of` spells it once.
Prototype declares `spike` and Code Review `report`; every other shipped workflow reads its fallback.
**`mod` is not one of the four.** Bridge reads a Mod Job off `SessionRecord.mod`, and that wins over the workflow's word.

**Bridge reads it where it already reads the Session's workflow**: `workOf` in `packages/screens/src/session-work.ts` finds the `WorkflowSummary` the record's `workflow` names and sets `work.landing`, which `landingOf` in `session-view/story.ts` reads.
The same read carries the resolved `phase` of the step at `workflow_step`, from `WorkflowStep.phase` (protocol 23.19), and `stageOf` opens the Landing on a step in `delivery`.
Neither adds a field to `SessionRecord`.

**The workflow file gains `lands:`**, a word refused outside the four.
**No store migration**: it is held beside the frozen workflow for `for_requests`'s reason, not frozen onto the Job.
The event stream is untouched.

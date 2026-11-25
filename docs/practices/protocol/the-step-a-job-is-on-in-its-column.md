# The step a Job is on, in its column: `SessionRecord.workflow_strip`, `StepReading.warning`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 20 Nov 2026. The Job's right column follows its workflow step: the tabs the step opens, the time on it, what ends it, and whether it has stood past its usual time.
**Additive**: one optional field on `SessionRecord`, two new types, one optional field on `StepReading`; no operation, route or event kind.
The protocol ID moves with `crates/ipc/src` and `classic-final:packages/protocol/src`, and there is no number to take. An older Fleet sends neither and Bridge draws the column as it does now.
Bridge's half is `classic-final:packages/protocol/src/sessions.ts` and `setup.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.workflow_strip` | Fleet answers, on every read of a live hosted Job on a workflow that resolves | `WorkflowStrip`: `tabs`, `usual_secs`, `ends_when`, `stalled` and `passed_unapproved`. Each part is absent where empty |
| `WorkflowStrip.tabs` | Fleet answers | The ids the step lists in its `tabs`, front first: the native Bridge's zone ids, with `now` and `walks`. Empty where the step lists none |
| `WorkflowStrip.usual_secs` | Fleet answers | The step's 9-in-10 time: the 90th percentile of how long Jobs on this workflow stayed on this step before the next began. Absent below five visits |
| `WorkflowStrip.ends_when` | Fleet answers | What ends the step, to the person, each end as `StepReading.ends_any` says it: `you approve the plan`, `the Job ends` |
| `WorkflowStrip.stalled` | Fleet answers | `StepStalled`: `since` (when the step went past `usual_secs`) and `cause` (what the step waits on, and whose it is). Absent while inside the time |
| `WorkflowStrip.passed_unapproved` | Fleet answers | The steps, by the index `workflow_step` uses, that work ahead passed without the Approve they wait on. Drawn `passed without your Approve` |
| `StepReading.warning` | Fleet answers, on `get_workflow_steps` | A sentence for the editor where a step in the middle of the workflow has nothing Fleet observes to end it. A warning only: the definition loads |

**The time on the step is not repeated.** It is the last of `workflow_visits` for `workflow_step`.

**Nothing here is the agent's word.** The tabs are the definition's, the usual time is measured from the visits the rail kept (`session_rail.visits`, read across Jobs by `Store::workflow_step_dwells`), and the rest are facts the rail reads.
`usual_secs` is read again after ten minutes, not on every row.
Fleet publishes a Job's row when its amber appears or goes, and sends the Job one `rail_nudge` for each visit to the step, never repeated (`crate::step_stalls`).

**The tab ids are `core_model::STEP_TABS`**, which a workflow file's `tabs` is refused outside of.
A bridge-native test holds every `Zone` id inside it; `now` and `walks` are the two the zone list lacks. A child Job's tab is `jobs`.

The warning has no place in the workflow check's refusals, which load nothing; `StepReading` is what the editor already reads per step, so it rides there.

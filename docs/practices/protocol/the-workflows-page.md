# The Workflows page: `get_workflow_steps`, `delete_workflow_copy`, `reset_workflow_copy`, `mark_not_a_drift`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 27 Oct 2026. The System › Kit › Workflows page draws a workflow as its steps: the words, what ends each, and the Jobs Fleet nudged there.
**Four operations and no change to an existing type.** The protocol ID moves with `crates/ipc/src`, `crates/ipc/operations/*.toml`, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/setup.ts`, written by hand, field for field.

| Operation | Route | Side | Carries |
| --- | --- | --- | --- |
| `get_workflow_steps` | `GET /workflows/steps?workflow_id=&source=&manifest_id=` | Fleet answers | `WorkflowSteps`: the id, the place and `steps`, each a `StepReading`: `id`, `label`, `instructions?`, `ends_when?`, `passed_over_when?`, `nudges?` (`StepNudge`: `id`, `job_id`, `job_title?`, `fact`, `at`) |
| `delete_workflow_copy` | `POST /workflows/delete?manifest_id=` | Bridge sends `WorkflowCopy` (`workflow_id`, `source`) | `WorkflowCopyDeleted`: the id, the place, the file and `runs_from?` |
| `reset_workflow_copy` | `POST /workflows/reset?manifest_id=` | Bridge sends `WorkflowCopy` | `WorkflowSaved`, since a reset is a save |
| `mark_not_a_drift` | `POST /workflows/not_a_drift` | Bridge sends `MarkNotADrift` (`nudge`) | `NotADriftMarked` |

**A route of its own, and not fields on `get_workflow`.** That read is the file's text and not a tree, so what an editor sends back to `save_workflow` is the string it was given.
The steps are read from the parsed file and resolved against the repository's Checks as a Job would freeze them.
One a more specific place replaced is resolved on the read, since the catalogue never does.
`ends_when` and `passed_over_when` are `rail_ends`' reading of the rules the rail places a Job by, so the page does not restate them.
Nudges are those `workflow_nudges` keeps for that step in that place's definition, less those marked not a drift.

**Refusals**, all 422 but the last:

| Code | Where |
| --- | --- |
| `fleet.no_such_workflow_definition` | `get_workflow_steps`, delete and reset, where that place holds none by that id; a definition that was left out is on `list_left_out_workflows` and is not here |
| `fleet.workflow_unfit` | `get_workflow_steps` of a replaced definition that names a Check this repository does not declare; reset, where Armada's does not fit here (nothing is written) |
| `fleet.workflow_is_armadas` | delete and reset with `source: armada` |
| `fleet.workflow_not_carried` | reset of a copy of an id Armada does not carry |
| `fleet.no_such_nudge` | `mark_not_a_drift` of an id Fleet kept none by |
| `fleet.workflow_unwritable` (500) | delete, where a file cannot be removed |

**Make my copy needs no route.** `get_workflow` with `source=armada` reads Armada's text and `save_workflow` with `scope: kit` and `overwrite: false` writes it under the same id; 422 `fleet.workflow_exists` where Kit already holds one.
It is checked against the picked repository's Checks, as every save is.

**Delete and reset act on copies Fleet holds.** A copy that was left out has no definition to find by id, so neither reaches it.
Fleet reads the three places again before answering, as `save_workflow` does. No store migration, no event.

**Agents.** The read is `Yes`, `get_workflow`'s terms; the three acts are `Bridge only`, `remove_trigger`'s reason: they are the person's press.

**The event stream is untouched**, so this makes the unbounded sink neither better nor worse.

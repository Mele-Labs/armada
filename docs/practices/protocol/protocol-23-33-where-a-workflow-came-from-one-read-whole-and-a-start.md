# Protocol 23.33: where a workflow came from, one read whole, and a start that leaves out

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The Workflow creator needs to draw each definition's place, the ones a more specific place replaced, and the one it is editing. A workflow file that does not fit stops refusing Fleet's start.

**Additive.** `WorkflowSummary` gains `file` and `overrides` (each an `OverriddenWorkflow { source, file }`, one per replaced place, absent where none). A new route, `GET /workflows/definition?workflow_id=&source=&manifest_id=`, `get_workflow`, answers a `WorkflowDefinition { workflow_id, source, file, definition, overridden_by? }`, where `definition` is the file's text in the shape `save_workflow` takes back. `source` is optional and absent means the one that runs; 422 `fleet.no_such_workflow_definition` where Fleet holds none by that id and place. `FleetHealth` gains `workflows_left_out`, a boolean and never a count, true where any served repository left a definition out. A 23.28 Bridge ignores all of it, and a 23.33 Bridge behind it is refused, which is the skew rule's own direction.

| What | How |
|---|---|
| A repository's own file that does not fit | Left out with its reason at start, as Kit's is; `list_left_out_workflows` carries it with `source: repository`. It no longer answers 422 `fleet.repository_refused` |
| Two files in one place with one id | Both left out, named together, at start and on a re-read alike |
| A definition a more specific place replaced | Not in `list_workflows`' rows; named on the winner's `overrides`, and read with `get_workflow` and its `source` |
| A step that routes back and names no `iteration_cap` | Takes five. Nothing else declares a back edge, so the passes cap is what bounds one |

**Not a protocol change, and the same release.** The workflow definition has no `structure` key any more; a file still writing it is left out as an unknown key. `verdict_routing` on any step is the only declaration of a back edge, and must name an earlier step. A Job's frozen copy never held the key, so no frozen Job moves.

**No migration, no store change, and no event.**

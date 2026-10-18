# Protocol 23.28: a workflow definition saved, and the folders read again

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner's ask of 5 Oct 2026: Helm helps author a workflow and ends by creating it, in the repository or in Kit.

**A route and its types, additive.** `POST /workflows/save`, `save_workflow`, takes `SaveWorkflow { scope, definition, overwrite? }` and answers `WorkflowSaved { workflow_id, scope, file, replaced, runs_from? }`, `scope` being `repository` or `kit`. `LeftOutWorkflow.source` may now read `repository`, where it read only `armada` or `kit`: a Fleet that was already running set a repository's own file aside. A 23.27 Bridge has no such route, and a 23.28 Bridge behind it is refused, which is the skew rule's own direction.

| What | How |
|---|---|
| The definition is checked | `config::fit`: parsed against the machine's models, resolved against the Manifest |
| A definition that does not fit | 422 `fleet.workflow_unfit` with the loader's sentence; nothing is written |
| An id that cannot be a file's name | 422 `fleet.workflow_id_not_a_name` |
| An id the scope already holds, no `overwrite` | 422 `fleet.workflow_exists`, naming the file |
| A file that cannot be written | 500 `fleet.workflow_unwritable` |
| Held | Before the answer, and again after any hand edit to either folder, by a watch on both folders, with `armada.yml`'s settle window |

**No migration, no store change, and no event.** What a reader sees move is `list_workflows` and `list_left_out_workflows`, which are asked again. The generated TypeScript moves by the version constant only, since the DTO types are mirrored by hand and no Bridge reads these yet.

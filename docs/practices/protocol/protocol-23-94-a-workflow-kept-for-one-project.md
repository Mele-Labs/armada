# Protocol 23.94: a workflow kept for one project

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/workflow.md`, *Where a definition comes from*. The owner's ruling of 13 Nov 2026: a repository runs, for each workflow id, **yours first, then narrower**.
**Changed**: `WorkflowScope` stops being a pair of words and loses `Copy`, so a caller that held one by value borrows it. Everything else is additive.
There is no number to bump: the protocol ID moves with the files below.

| Where | Side | Carries |
| --- | --- | --- |
| `WorkflowScope` (`SaveWorkflow.scope`, `WorkflowSaved.scope`, `ProposeNewWorkflow.scope`) | Both | `"repository"` and `"kit"` as before, and `{"project":{"id":"…"}}` for one project's copy, kept in Kit's folder under `projects/<id>/` |
| `WorkflowSummary.project`, `WorkflowDefinition.project`, `OverriddenWorkflow.project`, `LeftOutWorkflow.project` | Fleet writes, Bridge reads | `project?: string`, the project whose copy it is, where `source` is `project`. Absent otherwise, and from a Fleet before 23.94 |
| `WorkflowCopy.project`, `WorkflowCopyDeleted.project` (`delete_workflow_copy`, `reset_workflow_copy`) | Bridge sends, Fleet answers | `project?: string`. Required in the body where `source` is `project`, and 422 `fleet.workflow_project_unnamed` without it |
| `source` on `WorkflowSummary`, `WorkflowDefinition`, `LeftOutWorkflow`, `WorkflowSteps`, `WorkflowCopy`, `WorkflowSaved.runs_from` | Both | May now read `project`, beside `armada`, `kit` and `repository` |
| `WorkflowSummary.hides` | Fleet writes | `repository` where a project's copy hides a repository one that says something else, as Kit's does |

**Refused with nothing written.** A save, a check or a reset for a project Fleet holds none of is 404 `fleet.workflow_project_unknown`, and for one the repository named by `manifest_id` is not a member of is 422 `fleet.workflow_project_not_member`.
A change proposal for a project's copy, `propose_new_workflow` with the project scope or `propose_workflow_change` of a `project` source, is 422 `fleet.workflow_change_not_for_project`: a change is kept for the repository's or Kit's.

**What a left-out line says.** Two projects that both hold the repository and both keep one id are both left out, in one `LeftOutWorkflow` whose `source` is `project`, whose `project` is absent and whose `said` names both.
A project's copy that does not fit is left out with `project` set, and the next place down answers.

**The precedence is not on the wire.** Fleet applies it, and a surface reads `source`, `project` and `overrides`, so a Bridge never ranks places itself.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A Fleet that is still before this change sends no `project` and never `project` as a `source`.
Bridge's half is `packages/protocol/src/setup.ts`, written by hand like the rest.

**The event stream.** Untouched in kind: a save or a project's change publishes the `repositories.changed` a re-read already did.
The open risk below is neither better nor worse.

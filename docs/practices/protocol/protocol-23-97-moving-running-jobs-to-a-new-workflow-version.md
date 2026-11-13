# Protocol 23.97: moving running Jobs to a new workflow version

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/workflow.md`, *Drafts and Publish*.
The owner's ruling of 13 Nov 2026: **Publish asks which running Jobs move to the new version.** A publish moves none by itself.
**Added**: two operations, the types they carry, and `pinned` on `WorkflowPublished`.
`save_workflow` and a file edited by hand pin nothing and change what Jobs read at once, as they did.
There is no number to bump: the protocol ID moves with the files below.

Also added here, for the native editor: **`PutWorkflowDraft` gains `by?` and `change?`**, so an agent's change rebased onto a draft that moved is recorded as the agent's (see *An agent's put*).

## Why a publish had to pin

A hosted Job that holds no copy of its own reads its workflow's steps live from what its repository serves (`steps_with` falls back to `rail_steps`), so a publish changed the steps under every Job that followed it.
The exception is a Job holding its own copy in `job_copy_workflows`.
A pin is that same row, made by a publish, holding the text the Job was running.

## The routes

| Operation | Route | Side | Carries |
| --- | --- | --- | --- |
| `get_affected_jobs` | `GET /workflows/affected?workflow_id=&source=&project=&manifest_id=` | Fleet answers | `AffectedJobs` |
| `migrate_jobs` | `POST /workflows/migrate?manifest_id=` | Bridge sends `MigrateJobs` | `MigratedJobs` |
| `publish_workflow_draft` | `POST /workflows/draft/publish?manifest_id=` | as before | `WorkflowPublished`, which gains `pinned` |

`agent_access` is `Bridge only` for the two new operations, and `get_affected_jobs` is a query and `migrate_jobs` a command.
`manifest_id` names the repository only where `source` is `repository`; Kit's and a project's reach every repository they hold.

## The types

| Type | Fields |
| --- | --- |
| `AffectedJob` | `session_id`, `title?`, `manifest_id`, `step_id?` and `step_label?` (where the rail stands now, as the Job reads its own steps), `survives` (that step's id is in the target's steps; true where the Job stands on no step or the target does not read), `pinned`, `edited`, `on_version?` (the version in the place's log whose hash is the Job's copy text; absent where none matches and where the Job holds no copy) |
| `AffectedJobs` | `jobs` |
| `MigrateJobs` | `workflow_id`, `source` (`kit`, `repository` or `project`), `project?` (required for `project`), `session_ids` |
| `MigrateResult` | `session_id`, `moved`, `step_id?` and `step_label?` (where it stands now, where it moved), `code?` and `message?` (why not) |
| `MigratedJobs` | `results`, one for each id asked, in order |
| `WorkflowPublished` | gains `pinned` (the ids of the Jobs left pinned; an empty list where none) |

## What each does

**A publish pins.** Before the save, each live hosted Job that follows `workflow_id`, holds no copy of its own, and is in a repository the written place reaches (the repository's own; every served one for Kit; the repositories a project holds) is kept a pinned copy of the text its repository runs now, found as a Job's own copy is: the winning file for that id, with its path.
The place is the scope the publish writes to, which for a draft of Armada's is the scope the owner picked.
After the save, and after the other repositories it reaches are read again (a save re-reads only its own), each Job whose repository now runs the same text it was pinned to is unpinned.
Precedence can leave a repository running the same text: Kit published while a project copy still wins there.
A publish whose save is refused gives its pins back.
**A Job that already holds a copy is left alone**, whether its own edit or a pin an earlier publish made, so a second publish does not move a pinned Job's text on.
**A pin is not a change the Job made.** `get_job_copy` says `own` false for it, and `keep_job_workflow` refuses it as it does a Job that changed nothing; the Job's rail and lanes read the pinned steps.
**An edit clears the pin.** A step edit of a pinned Job begins from the pinned text and keeps the copy as the Job's own.

**`get_affected_jobs`** lists every live hosted Job that follows the workflow in a repository the place reaches, pinned, edited, or neither.
**The target** is the draft's text where a draft is kept for that place, else the text published there, so before a publish it is the preview of what Publish will pin and after it the list to move.
A Job with an edited copy is listed with `edited` true and cannot be moved.

**`migrate_jobs`** answers 200 with a result for each Job, in order, whatever the results are.
A move drops the pin, so the Job reads the published definition.
The rail is kept by step id and keeps its place; where the step the Job stood on is not in the new steps, the rail goes to the first step after it, in the old order, whose id is in the new steps, and where none is, to the new last step, with a visit at now.
A Job that has run is sent one line, `Fleet: the person moved this Job to the newest <workflow name> workflow; it is on the "<label>" step.`; a Job that has not run is told nothing.
The timeline gets a `session.workflow` entry, `workflow: moved to the newest <name>`, `by you`, and the Job's row and lanes are published again.
**A Job with no copy is not refused**: it already reads the published workflow, so its result is `moved` false with a `message` and no `code`.

## An agent's put

The editor rebases an agent's change onto the draft when the draft moved after the agent read it, and `PUT /workflows/draft` used to record that text as `you`.
Landing marks stay the agent's: `PutWorkflowDraft` gains `by?` (`you` or `agent`; absent is `you`) and `change?` (an `i64`).
**With `by: "agent"`, `change` must name a `WorkflowChange` Fleet holds** for the same `workflow_id`, source and project (and, for the repository's own, the repository), that is ready, carries an `after`, and is not `drafted` yet.
Fleet then adds the draft's change `by` the agent, with `change` that id, the change's ask as its summary and the put text whole, and marks the `WorkflowChange` `drafted`, the mark the landing of a drafted change sets.
**Anything else is a 422, `fleet.workflow_draft_change_not_pending`, with a sentence, and nothing is kept**: no `change`, an unknown id, another workflow, place or piece, a change not ready or without an `after`, one already `drafted`, and a `change` on a put by `you` or by none.
**Pieces are unchanged.** Fleet has no agent changes for pieces, so `PutPieceDraft` takes neither field.

## Refusals

| Code | Where |
| --- | --- |
| `fleet.workflow_migrate_not_live` | a result: the Job is not a live job Fleet hosts |
| `fleet.workflow_migrate_not_following` | a result: the Job follows a different workflow, is in a repository the place does not reach, or its repository no longer holds the workflow |
| `fleet.workflow_migrate_edited` | a result: the Job edited its own copy, and a move would lose the edits |
| `fleet.workflow_project_unnamed` (422), `fleet.workflow_project_unknown` (404) | either route, for a project's place |
| `fleet.workflow_draft_change_not_pending` (422) | `put_workflow_draft`, for a `by` and `change` that do not fit a waiting change |
| `fleet.no_such_workflow_definition` (422) | either route, for a source that is none of `kit`, `repository` and `project` |

## What is kept

One column: `job_copy_workflows.pinned`, 1 for a pin and 0 for a Job's own edit and for every copy kept before this.
Migration `20261113T1400Z-job_copy_pins.pinned.sql`, additive.

## Skew and the stream

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Bridge's half is `packages/protocol/src/workflow-migrating.ts` and `workflow-drafts.ts` (which also carries `by` and `change` on `PutWorkflowDraft`), written by hand like the rest.
**The event stream.** A move publishes the moved Job's row again, as any step change does, one row for each Job moved, and a publish raises the `repositories.changed` it already did.
The open risk below is neither better nor worse.

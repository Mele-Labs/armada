# Protocol 23.95: a workflow edited as a draft and published

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/workflow.md`, *Drafts and Publish*.
The owner's ruling of 13 Nov 2026: edits to a workflow are **a draft, then Publish**, with a history he can step back through, and **an agent's edits land in his draft, marked as the agent's**.
Lifecycles, rules and autonomy defaults get drafts too.
**Added**: eight operations, the types they carry, and `project` and `drafted` on `WorkflowChange`.
**Changed**: `ask_workflow_change` accepts `armada` and `project`, and `list_workflow_changes` takes `project`.
`save_workflow` is untouched and still writes at once, for the Classic Bridge, and so is `approve_workflow_change`, apart from refusing a change to one of Armada's own.
There is no number to bump: the protocol ID moves with the files below.

## The routes

A draft is read, kept and thrown away at one path, by three methods.
Every workflow route takes `?manifest_id=` as the other `/workflows` routes do, and a draft is kept by it only where the place is `repository`: Kit's, a project's and Armada's are the same in every repository.

| Operation | Route | Side | Carries |
| --- | --- | --- | --- |
| `get_workflow_draft` | `GET /workflows/draft?workflow_id=&source=&project=&manifest_id=` | Fleet answers | `WorkflowDraft` |
| `put_workflow_draft` | `PUT /workflows/draft?manifest_id=` | Bridge sends `PutWorkflowDraft` | `WorkflowDraft` |
| `discard_workflow_draft` | `DELETE /workflows/draft?workflow_id=&source=&project=&manifest_id=` | Bridge asks | `WorkflowDraftDiscarded` |
| `publish_workflow_draft` | `POST /workflows/draft/publish?manifest_id=` | Bridge sends `PublishWorkflowDraft` | `WorkflowPublished` |
| `get_piece_draft` | `GET /lifecycles/pieces/draft?kind=&id=&within=` | Fleet answers | `PieceDraft` |
| `put_piece_draft` | `PUT /lifecycles/pieces/draft` | Bridge sends `PutPieceDraft` | `PieceDraft` |
| `discard_piece_draft` | `DELETE /lifecycles/pieces/draft?kind=&id=&within=` | Bridge asks | `PieceDraftDiscarded` |
| `publish_piece_draft` | `POST /lifecycles/pieces/draft/publish` | Bridge sends `PublishPieceDraft` | `PiecePublished` |

These are the first routes served on `PUT` and `DELETE`; the gate's rule reads a command on `POST`, `PUT` or `DELETE`, and a query on `GET` alone. `agent_access` is `Bridge only` for all eight.

## The types

| Type | Fields |
| --- | --- |
| `DraftChange` | `id` (Fleet's number, rising across every draft), `by` (`you` or `agent`), `at`, `text` (the whole draft after this change), `summary?`, `change?` (the `WorkflowChange` an agent's change came from) |
| `WorkflowDraft` | `workflow_id`, `source` (`armada`, `kit`, `repository` or `project`), `project?`, `text` (the draft, whole: the last change's), `base` (the published text it began from, and for `armada` the carried text, empty where the place held none), `unfit?` (the loader's sentence where `text` would not load), `changes` (oldest first, never empty), `started_at`, `updated_at` |
| `PutWorkflowDraft` | `workflow_id`, `source`, `project?`, `text`, `summary?` |
| `WorkflowDraftDiscarded` | `workflow_id`, `source`, `project?` |
| `PublishWorkflowDraft` | `workflow_id`, `source`, `project?`, `scope?` (a `WorkflowScope`), `overwrite?` (false where absent; read only for `armada`) |
| `WorkflowPublished` | `saved` (the `WorkflowSaved`). A struct so a later field has room |
| `PieceDraft` | `kind` (`lifecycle`, `rule` or `autonomy`), `id`, `scope` (a `PieceScope`), `text` (the piece as the JSON `SaveLifecyclePiece.piece` takes), `base` (the piece Kit held, as the same JSON, empty where Kit held none), `unfit?`, `changes`, `started_at`, `updated_at` |
| `PutPieceDraft` | `kind`, `id`, `scope` (everywhere where absent), `text`, `summary?` |
| `PieceDraftDiscarded` | `kind`, `id`, `scope` |
| `PublishPieceDraft` | `kind`, `id`, `scope` (everywhere where absent) |
| `PiecePublished` | `saved` (the `LifecyclePieceSaved`) |
| `WorkflowChange` | gains `project?` (where `source` is `project`) and `drafted?` (absent where false), meaning `after` landed in the draft |
| `AskWorkflowChange` | gains `project?`; `source` may now read `armada` or `project` |

On a piece route the scope is `within` in a query (the scope's id; absent is everywhere), since a `PieceScope` is an object, and `scope` in a body.

## What each does

**A put begins the draft where there is none**, from what is published in that place now (Armada's carried text for `armada`, empty where the place holds no file of the id), and adds a change by `you`.
Where one is kept it adds a change by `you` and `base` stays.
A put of the text the draft already ends on adds nothing, so putting an earlier text again is how the editor undoes and a repeat does not grow the list.
**A put accepts any text**, since a draft may be half-edited.
`unfit` on the answer is the parse and resolve `save_workflow` runs, and also the sentence that the text declares another `workflow_id` than the draft's, since publishing it would write another workflow.
For a piece it is that the text is not a piece, is another kind, id or scope than the draft's, or that the piece's parser refuses it.

**A publish saves through the existing save.** A workflow draft is written in its own place, `repository`, `kit` or the project's, and always replaces what that place holds, since that is the copy it was drafted from and the stale check guards it.
The draft is deleted only once the file is.
A piece draft is decoded and run through `save_lifecycle_piece` with `overwrite`.
The loader's and the parser's refusals pass through unchanged, and nothing is written on one.
**The scope of a workflow publish.** For a draft of an `armada` workflow, `scope` is required and may be any of the three. For any other source it is absent, or names that same place.
**A draft of Armada's is held against the carried text**, not against a copy already at the scope picked, and its copy is a new file.
Where the scope already holds a definition of the id, a publish without `overwrite` is refused with `fleet.workflow_exists`, which names the file: nothing is written and the draft stays.
With `overwrite: true` it replaces that copy, as a save with `overwrite` does.

**Where an agent's edit goes.** `ask_workflow_change` reads `before` from the draft's text where there is a draft, else the published or carried text.
When the drafting finishes and the change is `ready`, Fleet writes `after` into the draft as a change by `agent`, with `change` the id of the `WorkflowChange` and `summary` its words, and sets `drafted`.
It does so only where the draft's text still equals `before`, or, with no draft, where the published text still equals `before`, in which case it begins the draft.
Anywhere else the change stays `ready`, as it was.
It happens under the lock that moves the change to `ready`, so a reader never sees a ready change that has not yet landed.
A change sent back is drafting again and `drafted` clears.
`propose_workflow_change` and `propose_new_workflow` are untouched, and a project's is still refused there with `fleet.workflow_change_not_for_project`.

## Refusals

| Code | Where |
| --- | --- |
| `fleet.workflow_draft_none` (404) | get, discard, publish of a workflow draft none is kept for |
| `fleet.workflow_draft_scope_needed` (422) | publish of an `armada` draft without `scope` |
| `fleet.workflow_draft_scope_differs` (422) | publish of any other draft with a `scope` that names another place |
| `fleet.workflow_exists` (422) | publish of an `armada` draft, without `overwrite`, to a scope that already holds the id. Nothing is written and the draft stays |
| `fleet.workflow_draft_stale` (409) | publish, where what is published in the place is no longer `base`. Nothing is written and the draft stays |
| `fleet.workflow_draft_not_kept` (500) | any, where the store would not answer |
| `fleet.workflow_unfit` (422), `fleet.workflow_id_not_a_name` (422), `fleet.workflow_unwritable` (500) | publish, as `save_workflow` refuses |
| `fleet.workflow_project_unnamed` (422), `fleet.workflow_project_unknown` (404), `fleet.workflow_project_not_member` (422) | any workflow route with `source: project`, as `save_workflow` refuses |
| `fleet.no_such_workflow_definition` (422) | put, a source that is none of the four, or `armada` for an id Armada does not carry |
| `fleet.piece_draft_none` (404), `fleet.piece_draft_stale` (409), `fleet.piece_draft_not_kept` (500) | the piece routes, as the workflow codes above |
| `fleet.piece_unfit` (422), `fleet.piece_unwritable` (500) | publish of a piece, as `save_lifecycle_piece` refuses |
| `fleet.workflow_is_armadas` (422) | `approve_workflow_change` of a change to one of Armada's own: there is no file to write, and the draft is published instead |

## What is kept

`workflow_drafts` holds one row for each draft, keyed by kind (`workflow`, or a piece's kind), place (`source` and `project`), the repository where the place is `repository` and empty otherwise, a piece's scope and the id; `workflow_draft_changes` holds its changes under it.
`workflow_changes` gains `project` and `drafted`.
Migrations `20261113T1200Z-workflow_drafts.table.sql` and `20261113T1200Z-workflow_changes.drafted.sql`, both additive.

## Skew and the stream

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Bridge's half is `packages/protocol/src/workflow-drafts.ts` and `setup.ts`, written by hand like the rest.
**The event stream.** Untouched: a draft raises no event of its own, and a publish raises the `repositories.changed` a save already did.
A Bridge learns an agent's change landed from `WorkflowChange.drafted` on the list it already polls while a change drafts, and reads the draft then.
The open risk below is neither better nor worse.

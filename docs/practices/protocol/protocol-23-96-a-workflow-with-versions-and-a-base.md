# Protocol 23.96: a workflow with versions and a base

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/workflow.md`, *Drafts and Publish*.
The owner's ruling of 13 Nov 2026: edits are a draft, then Publish, **with a history he can step back through**; and **when Armada ships a new version of a workflow he copied, he sees what changed and chooses what to take into his draft**.
The native editor does the diff and the choosing. Fleet keeps what the copy was based on, and marks when Armada's has moved.
**Added**: two operations, the types they carry, `version` on `WorkflowPublished`, `based_on` and `armada_changed` on `WorkflowSummary`, `based_on` on `WorkflowDefinition`, and `source=base` on `get_workflow`.
`save_workflow` and `approve_workflow_change` write no version.
There is no number to bump: the protocol ID moves with the files below.

## The routes

| Operation | Route | Side | Carries |
| --- | --- | --- | --- |
| `get_workflow_versions` | `GET /workflows/versions?workflow_id=&source=&project=&manifest_id=` | Fleet answers | `WorkflowVersions` |
| `revert_workflow` | `POST /workflows/revert?manifest_id=` | Bridge sends `RevertWorkflow` | `WorkflowDraft` |
| `get_workflow` | `GET /workflows/definition?workflow_id=&source=base&of=&project=&manifest_id=` | Fleet answers | `WorkflowDefinition` with `source: "base"` |

`agent_access` is `Bridge only` for the two new operations.
`get_workflow` stays `Yes` and the agent's tool names `workflow_id` and `source` only, so it reads no base: `source=base` without `of` is the 422 below.
**`of` names the copy's place** (`kit`, `repository` or `project`, with `project` naming the project), since `source` already says `base`; `source=armada` stays Armada's current text, so a client holds both to diff.

## The types

| Type | Fields |
| --- | --- |
| `VersionOrigin` | `published` (a publish wrote it) or `found` (the text in the place when the first publish came) |
| `WorkflowVersion` | `version` (from 1), `text` (the definition whole), `hash` (the sha256 of `text`, in hex), `at`, `origin` |
| `WorkflowVersions` | `versions` (newest first; empty where nothing was published) |
| `RevertWorkflow` | `workflow_id`, `source` (`kit`, `repository` or `project`), `project?` (required for `project`), `version` |
| `BasedOn` | `version` (the `version` Armada's carried definition declared when the copy was made), `hash` (the sha256 of that carried text, in hex) |
| `WorkflowPublished` | gains `version` (the number this publish took) |
| `WorkflowSummary` | gains `based_on?` and `armada_changed` (absent where false) |
| `WorkflowDefinition` | gains `based_on?`; for `source: "base"` it carries the base's own `version` and `hash` |

## What each does

**A publish writes the log.** The text it wrote is the next version in the place it wrote to: the draft's own, or for a draft of Armada's the scope the owner picked.
**The first publish into a place also keeps the text that was there as version 1 (`found`)**, written before the publish's own row and only where a definition was there; the publish is then version 2.
For a draft of Armada's, whose copy is a new file, the place held nothing and its first publish is version 1; where it replaced a copy with `overwrite`, that copy's text is kept as `found`.
Nothing is written for a hand edit between publishes: the stale check refuses a draft that began from other text, and a later draft begins from what is there.
**A revert is a put.** The version's text goes into the draft by the path `put_workflow_draft` takes: a change by `you` with the summary `Back to version N`, beginning the draft where there is none, and adding nothing where the draft already ends on that text.
It publishes nothing.
**A base is kept only where it is known.** It is recorded by a publish of a draft of Armada's (the carried text the draft began from), by a `save_workflow` into a place with no base whose text is Armada's carried text of that id byte for byte (which is how a copy is made from the editor and from Classic's *Make a copy*), and by `reset_workflow_copy` (the carried text it wrote).
**Otherwise none is recorded**: a copy made before this, or made by hand, has no `based_on`, and nothing is guessed for it.
`delete_workflow_copy` drops the place's versions and base; `reset_workflow_copy` drops the versions and records the carried text as the new base.
**`armada_changed`** is true on a summary whose copy has a base and where Armada's carried text of the id now hashes to something else.
The summary is the one that runs; a copy a more specific place replaced (`overrides`) carries neither field.

## Refusals

| Code | Where |
| --- | --- |
| `fleet.workflow_version_none` (404) | `revert_workflow`, where the place has no such version |
| `fleet.workflow_base_none` (404) | `get_workflow` with `source=base`, where the copy has no base |
| `fleet.workflow_base_of_needed` (422) | `get_workflow` with `source=base` and no `of` |
| `fleet.workflow_project_unnamed` (422), `fleet.workflow_project_unknown` (404), `fleet.workflow_project_not_member` (422) | the two routes and `source=base`, for a project's place, as the draft routes refuse them |
| `fleet.workflow_draft_not_kept` (500) | any, where the store would not answer; a publish whose file was written but whose versions were not keeps its draft |

## What is kept

`workflow_versions` holds one row for each version, keyed by place (`source`, `project`, the repository only where the place is `repository`), the workflow's id and the number.
`workflow_bases` holds one row for each copy whose base is known: its `version`, `hash` and `text`.
Migration `20261113T1300Z-workflow_versions.table.sql`, additive. The hash is computed by Fleet, which already depends on `sha2`; the store takes it as given.

## Skew and the stream

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Bridge's half is `packages/protocol/src/workflow-versions.ts`, `workflow-drafts.ts` and `setup.ts`, written by hand like the rest.
**The event stream.** Untouched: a publish raises the `repositories.changed` it already did, and a revert raises none, since it changes a draft.
The open risk below is neither better nor worse.

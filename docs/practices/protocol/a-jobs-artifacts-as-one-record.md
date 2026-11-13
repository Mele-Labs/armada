# A Job's artifacts as one record: `get_job_artifacts`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. A Job's sketches, drafts and pages as one list, each with its revisions and where its newest stands, so native can draw one Artifacts tab. `docs/concepts/artifact.md`.
**Additive only**: one operation, one route and the types it carries. No field of `SessionRecord`, no event kind and no error is new.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not read it.

| Where | Side | Carries |
| --- | --- | --- |
| `get_job_artifacts` (`GET /sessions/artifacts?session_id=`, `agent_access = "No"`) | Bridge reads | A list of `JobArtifact`, newest first by its newest revision's `at`: `id`, `title`, `form`, `revisions[]`, `state`, `words?`, `item?`. `ArtifactRevision`: `number`, `at`, `by`, `form`. 422 `fleet.session_unnamed` for a job Fleet does not know |
| `ArtifactForm` | Fleet | `sketch`, `draft`, `page` |
| `ArtifactBy` | Fleet | `agent`, `owner`; always `agent` today |
| `ArtifactState` | Fleet | `reading`, `asking`, `approved`, `sent_back`, `withdrawn`, `dismissed` |

**A draft is now a ledger attachment too.** `propose_draft` upserts an `artifact` of form `draft` at target `draft:<title>`, its `detail` holding `form`, `by`, `title`, `revisions` (the kept `r<n>.md` paths, oldest first) and `row`, the newest row's id.
It rides in `SessionRecord.attachments` beside sketches and pages; Classic and native both pass over a form they do not know.
The `draft` thread row is kept as it was, so Classic and native answer it as before.
A draft proposed under the title of one still open settles that row `withdrawn` with no message, so `waiting_for` holds one item for the title.

**A revision's `at` is read off the attachment's history**, the table `get_attachment_history` serves: the first revision is the row's `since`, and a later one the first version whose `revisions` list names it, else the row's `changed_at`.
Nothing new is written for it.

**Not on `SessionRecord`.** It is read on demand, as the attachment history is, and a `session.changed` says when to read it again.
It adds no event and no queue, so it leaves the unmeasured sink risk where it was.
`propose_draft` still publishes one `session.changed` per call; settling a replaced row publishes only its `session.row`.

**Skew.** The route is new, so the protocol ID moves with it and a Bridge and a Fleet that differ on it never meet.

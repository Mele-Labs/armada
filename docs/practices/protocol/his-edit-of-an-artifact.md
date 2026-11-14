# His Edit of an artifact: `save_artifact_edit`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. He edits a page's or a draft's words in place, and Save keeps them as the artifact's next revision, his. `docs/concepts/artifact.md`, *His Edit*.
**Additive only**: one operation, one route and one body type. No field of `SessionRecord` or `JobArtifact`, no event kind and no error kind is new.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not read it, and native draws the toggle later.

| Where | Side | Carries |
| --- | --- | --- |
| `save_artifact_edit` (`POST /sessions/artifacts/edit`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 | Body `SaveArtifactEdit`: `session_id`, `artifact` (the page's path or `draft:<title>`), `revision` (the number he edited) and `text` (a page's whole HTML, or a draft's markdown). Answers the `JobArtifact` |
| `ArtifactRevision.by` | Fleet writes, Bridge reads | Now `owner` on a revision he saved with his Edit; every other revision stays `agent` |
| Refusals | Fleet | 409 `fleet.edit_not_newest`, naming the newest. 422 `fleet.edit_not_words` for a sketch, or a page whose elements differ or whose words changed outside text, headings, lists and table cells, naming the first element that differs; `fleet.edit_settled`; `fleet.edit_archived`; `fleet.artifact_not_held`; `fleet.session_unnamed`. 500 `fleet.edit_unkept` |

**The Job is told first.** It is sent his message, `He edited "<title>": revision <n> is his, from revision <n-1>.` and each change, before anything is kept, so a send that fails keeps nothing, as his other answers do.

**Who wrote a revision is a store row, not attachment detail.** `artifact_owner_revisions` keeps the Job, the artifact's id, the revision's number and when, and `get_job_artifacts` reads it.
A page's and a draft's attachment is rewritten whole on each revision, so a key there would need carrying forward at every write.

**The page's file holds his bytes.** Fleet writes them to the page in the worktree and keeps the same bytes as the revision, so the turn's end reads nothing new and keeps no revision of the agent's for them.

**The event stream.** A save publishes `session.changed`, as an archive does, and Bridge reads `get_job_artifacts` again on it.
No event kind is new, so the unmeasured sink risk is no worse.

**Skew.** The route is new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.

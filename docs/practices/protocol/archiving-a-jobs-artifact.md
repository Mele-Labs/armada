# Archiving a Job's artifact: `archive_artifact` and `restore_artifact`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. The agent or he folds a stale sketch, draft or page away, and brings it back. `docs/concepts/artifact.md`, *Archive*.
**Additive only**: two operations, two routes, one optional field and one body type. No field of `SessionRecord`, no event kind and no error kind is new.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not read it.

| Where | Side | Carries |
| --- | --- | --- |
| `archive_artifact` (`POST /sessions/artifacts/archive`, `agent_access = "Yes"`) | Agent and Bridge call | Body `ArtifactRef`: `artifact`, the `JobArtifact` id, and `session_id?`. Answers the `JobArtifact` with `archived` set |
| `restore_artifact` (`POST /sessions/artifacts/restore`, `agent_access = "Yes"`) | Agent and Bridge call | Body `ArtifactRef`. Answers the `JobArtifact` with `archived` absent |
| `JobArtifact.archived?` | Fleet writes, Bridge reads | `ArtifactArchived`: `by` (`agent` or `owner`) and `at`. Absent unless archived |
| `JobArtifact.item?` | Fleet writes | Now also absent while an archive keeps the artifact off his list |
| `JobArtifact.state` | Fleet writes | No new value. `withdrawn` now reads for a sketch or page the Job withdrew, and an archived one reads as it stood |
| Refusals, all 422 | Fleet | `fleet.artifact_not_held`, naming at most 20 ids the Job holds; `fleet.artifact_archived_already`; `fleet.artifact_not_archived`; `fleet.artifact_unnamed`; `fleet.session_unnamed` |

**`by` is read off the door the call came through.** A person's own press, from Bridge, the phone or another of his Macs, is `owner`; the agent's door and any caller that names none is `agent`.
A hosted Job is placed by its connection, as `withdraw_sketch` is, and `session_id` is read otherwise.

**The archive is a store row, not attachment detail.** `artifact_archives` keeps the Job, the id, `archived_by`, `archived_at` and the Needs you id the archive dismissed.
Sketches, pages and drafts each rewrite their attachment's detail whole on a new revision, so a key there would need carrying forward at every write.
The row and the dismissal are written in one transaction, and a restore lifts both in one.

**Withdrawn and dismissed told apart.** `withdraw_sketch` and `withdraw_document` add `withdrawn_by` = `agent` to the attachment's `detail`, which rides in `SessionRecord.attachments` as any detail key does.
His Dismiss does not write it.

**The event stream.** An archive and a restore publish one `session.changed` each, as a withdrawal does, and Bridge reads `get_job_artifacts` again on it.
No event kind is new, so the unmeasured sink risk is no worse.

**Skew.** The routes are new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.

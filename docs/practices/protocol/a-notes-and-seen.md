# Notes on words and the revision he last saw: `ItemNote` and `mark_artifact_seen`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. A note of a verdict that points at words, and the newest revision of an artifact he opened. `docs/concepts/artifact.md`, *Seen and anchored notes*.
One field changes type and one field is new, so this is not additive: `ItemAnswer.notes` was strings and is now `ItemNote`s, and no Bridge sent `answer_items` before this change.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `ItemAnswer.notes` (`answer_items`) | Bridge writes | `ItemNote[]`, each `{ text, on? }`. Was `string[]` |
| `ItemNote.on` | Bridge writes | `NoteOn` `{ quote, block }`: the words he marked, and the page block's id or the sketch part's id they sit in. Skipped where the note has none, as a card's note box sends it |
| `mark_artifact_seen` (`POST /sessions/artifacts/seen`, `agent_access = "No"`) | Bridge writes | `MarkArtifactSeen` `{ session_id, artifact, revision }`: the artifact's `id` and the revision he opened |
| Its answer | Fleet writes, Bridge reads | 200 `ArtifactSeen` `{ seen }`: the newest revision he has opened, after the mark |
| `JobArtifact.seen` (`get_job_artifacts`) | Fleet writes, Bridge reads | The newest revision he opened. Skipped while he has opened none |
| Refusals | Fleet | 422 `fleet.artifact_revision_unknown` for a revision never kept, `fleet.artifact_not_held` or `fleet.session_unnamed` |

**A note with `on` reads to the Job as `On "<quote>": <text>`**, numbered as every note is.
A note with no `on`, or a `quote` blank after trimming, reads as its `text`, and blank `text` is dropped as before.
The `block` is not told to the Job.

**`seen` only moves forward.**
A mark of an older revision, or of the one already kept, is answered with the current `seen` and is not refused.
The refusal `fleet.artifact_revision_unknown` is the one `get_artifact_page` uses, declared once in Fleet.
Nothing is sent to the Job.

**The event stream.** `session.changed` is published on the artifact's Job where a mark moved `seen`, and not where it held, so a Bridge that marks what it already marked is not told to read again.
One mark is one event at most, so the unmeasured sink risk is neither better nor worse.

**Skew.** The route is new and `ItemAnswer.notes` changed shape, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A `JobArtifact` with `seen` reads on a Bridge that does not know it as the artifact without it.

# The owner's notes on a pull request's lines: `list_pull_request_notes` and four more

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/pull-review.md`.
**Additive only**: five operations, five routes, two error codes and the types they carry; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `apps/desktop/src/shared/api/pull-request-notes.ts`, written by hand, field for field.

`:repository` is the Manifest id, and `repository` in a body or a note is `owner/name`, as the forge names it.

| Where | Side | Carries |
| --- | --- | --- |
| `list_pull_request_notes` (`GET /pull_request_notes/:repository/:number`, `agent_access = "Bridge only"`) | Bridge reads | `PullRequestNotes` `{ notes }`, oldest first. 422 for a repository Fleet does not serve |
| `save_pull_request_note` (`POST /pull_request_notes/:repository/:number`, `agent_access = "Bridge only"`) | Bridge writes | `SavePullRequestNote` `{ id?, path?, line?, side?, body }`; answers the `PullRequestNote` as kept. Without `id` it makes a note and `path`, `line` and `side` are required; with `id` it changes that note's `body` only, while it is open and unsent. `body` is kept trimmed |
| `remove_pull_request_note` (`POST /pull_request_notes/:repository/:number/:id/remove`, `agent_access = "Bridge only"`) | Bridge writes | No body; answers `PullRequestNotes`. A note already gone is not an error |
| `send_pull_request_notes` (`POST /pull_request_notes/:repository/:number/sent`, `agent_access = "Bridge only"`) | Bridge writes | `SendPullRequestNotes` `{ note_ids, session_id, title }`; sets `sent_to`, `sent_title` and `sent_at` on each and answers `PullRequestNotes`. It records and sends nothing |
| `resolve_pull_request_note` (`POST /sessions/pull_request_note_resolved`, `agent_access = "Yes"`) | An agent writes | `ResolvePullRequestNote` `{ note_id, session_id? }`, `session_id` read only where the connection places no hosted session; answers the `PullRequestNote` with `status` `done` |
| `PullRequestNote` | Fleet writes, Bridge reads | `{ id, repository, number, path, line, side, body, status, sent_to?, sent_title?, sent_at?, created_at }`; `side` is `left` or `right`, `status` is `open` or `done` |

**Refusals.**

| Code | When |
| --- | --- |
| 422 `fleet.pull_request_note_invalid` | `save_pull_request_note` with a blank body, a new note missing `path`, `line` or `side`, a note that is not there, or one already sent or done; `send_pull_request_notes` with an id that names no note on that pull request |
| 422 `fleet.pull_request_note_refused` | `resolve_pull_request_note` for a note not sent to this job (a never-sent one included), an unknown id, or one already resolved. The note is left as it was |

**The notes stay in Fleet's store on his Mac.** They are never posted to the forge, and they are not the review's drafts.

**One message, then a record.** Bridge sends the unsent notes to a Job as one numbered message through `send_session_message`, each note named by `path:line` and its id, with the line `call resolve_pull_request_note with its id`.
Bridge then calls `send_pull_request_notes`, which is why the Job a note was sent to is the only one that can resolve it.
The Job is the one whose process holds the connection, an ended Job is refused, and only a Job Fleet hosts is served.

**Bridge's call** reads the notes again when the Files tab is entered, and folds the resolved ones away behind Show resolved.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404, which Bridge words as a read or a save that failed.

**The event stream.** Untouched: resolving a note pushes no event, so Bridge sees it only on its next read.
The open risk below is no worse.

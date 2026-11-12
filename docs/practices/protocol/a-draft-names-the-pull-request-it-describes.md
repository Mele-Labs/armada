# A draft names the pull request it describes: `SessionRecord.pull_request_drafts`, `PullRequestPage.draft`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 12 Nov 2026. A Job that proposes a new description for a pull request it holds puts a draft to the person, and he had to match each draft to its pull request himself.
Fleet now links the two, so a client can show the waiting draft on the pull request's card and page, and the pull request on the draft.
**Additive only**: two optional fields out, one optional field in, and one type they carry; no operation, route or event kind, and one error code.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
The Rust half is `PullRequestDraft` in `crates/ipc/src/pull_requests.rs`; the link is kept and given back in `crates/fleet/src/session_host/drafting.rs`.
Bridge's half is `packages/protocol/src/pull-requests.ts` and `sessions.ts`, written by hand, field for field. Electron Bridge reads neither field.

| Where | Side | Carries |
| --- | --- | --- |
| `ProposeDraft.pull_request` (`propose_draft`) | Agent sends | The number of the pull request whose description the draft is. Refused 422 `fleet.draft_pull_request_not_held` where the Job holds no such pull request, with nothing kept |
| `SessionRecord.pull_request_drafts` | Fleet answers | `PullRequestDraft[]`, each draft of the Job that waits on the person and is linked. Absent where none is |
| `PullRequestPage.draft` (`get_pull_request_detail`) | Fleet answers | The `PullRequestDraft` waiting on that pull request, the newest where Jobs proposed several. Absent where none waits |
| `PullRequestDraft` | Fleet writes | `manifest_id`, `number`, `draft` (the row id its `draft:<id>` item names), `text`, `before` (absent where the Job gave none) and `revision` |

**Detected, then named.** Where the Job names no pull request, Fleet compares the draft's `before` with the body it last read of each pull request the Job holds and has not given back, taking every run of whitespace as one space.
The newest that matches is the link; none matching links nothing.
A named pull request wins over `before`.
**`revision`** is 1 for the Job's first draft for that pull request, and one more for each it proposed since, settled ones too: a draft sent back and proposed again is revision 2.

**Kept on the Job's ledger.** The link is a `pr_draft` row: its target the draft's row id, its repository the pull request's, and `detail` holding `pull_request`, `text`, `before` and `revision`, so a record is drawn without reading the thread.
The row is not served in `attachments`; `pull_request_drafts` is its wire form, and `get_attachment_history` keeps every version.
**Settling the draft gives the row back**, whichever way it is settled: approved, approved with fixes, sent back, dismissed or withdrawn.

**Approve, Edit and Note stay the draft's own acts.** A client answers the `draft:<id>` item with `answer_waiting` as before, and the Job is sent what it was sent before.
Fleet never writes the pull request's body: the Job edits it once the person approves, as with any draft.

**Why not a field on the `draft` row.** The native Bridge destructures `SessionRow::Draft` field by field, so a new field there breaks its build.
The Job's record carries the link instead, keyed both ways.
`SessionRecord` takes no `Default`: that would need one on `SessionId`, `SessionState` and `Instant`, and no client builds a record by hand.
`PullRequestPage`, `PullRequestDraft` and `ProposeDraft` derive it, with `ManifestId`'s empty id, `PullRequestMergeable::Unknown` and `PullRequestReviewState::None` as their defaults.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream is neither better nor worse.** `session.changed` already carries the record whole; it grows by the waiting drafts' text.

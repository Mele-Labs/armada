# Protocol 23.48: acts on a pull request

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Acts on a pull request*.
**Additive only**: five operations, the DTOs of `ipc::pull_requests`, and one `origin` value.
A pull request is named by its repository (a manifest id) and its number, and nothing here is a Job's. 23.49 is the session host's.

| Operation | Carries | Notes |
| --- | --- | --- |
| `get_pull_request` (`GET /pull_requests/:repository/:number`) | `PullRequestState`: `state` (`draft`, `open`, `merged` or `closed`), `auto_merge`, `queued` (in the base branch's merge queue now), `checks` (`pending`, `passed`, or `failed` with `failing` names), `title`, `branch`, `address`, `body` (the description as Markdown, left out where there is none) | `agent_access` `Yes`. Refreshes every Job's `pr` row for it, and the pull watch does each interval for pull requests nobody opened; a merged one is settled `spent`, a closed one `given_back`, in the same write as its detail |
| `ready_pull_request` (`POST /pull_requests/:repository/:number/ready`) | nothing | Answers `PullRequestState`. `agent_access` `Helm only`, asked as `pushes to shared` |
| `merge_pull_request_by_number` (`POST /pull_requests/:repository/:number/merge`) | nothing | Answers `PullRequestState`. Refused with `fleet.merge_checks_not_passed` unless the checks have passed. `Helm only`, `pushes to shared` |
| `enable_auto_merge` (`POST /pull_requests/:repository/:number/auto_merge`) | nothing | Answers `PullRequestState`. While the checks run, and once they have passed, when the forge queues it. `Helm only`, `pushes to shared` |
| `review_pull_request` (`POST /pull_request_reviews/:repository`) | `ReviewPullRequest`: `pull_request` (a number or an address), `session_id?` | Answers `ReviewDispatched {job_id, address, session_id?}`, a 201. `Helm only` |

**`merge_pull_request` was taken**, and is a Job's press at its gate, so the by-number sibling is `merge_pull_request_by_number` and the two are never one route.
**`origin` gains `session_dispatched`** (*From a Session, by you*); a Bridge before 23.48 shows the raw spelling on such a row, since `origin` is a string on the wire.
Which Session is the ledger's: the Session holds a `job` row with `detail.origin` reading *dispatched from Session <id>*.

**Refusal codes**, all 409 unless noted: `fleet.merge_checks_not_passed`, `fleet.merge_not_open`, `fleet.pull_request_is_a_draft`, `fleet.pull_request_not_a_draft`, `fleet.ready_refused` and `fleet.auto_merge_refused` (the forge's own sentence), and the forge's merge kinds `fleet.merge_branch_protected`, `fleet.merge_conflicted`; 500 for `fleet.pull_request_unreadable`, `fleet.merge_no_tool` and `fleet.merge_refused`; 422 for `fleet.pull_request_unnamed` and `fleet.session_unknown`.
No migration.
Bridge's half is `packages/protocol/src/pull-requests.ts`, written by hand like the rest.

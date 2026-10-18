# Protocol 23.83: a pull request's page and the writes on it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Acts on a pull request*.
**Additive only**: two operations and the DTOs of `ipc::pull_requests` that go with them.
The type is `PullRequestPage` and not `PullRequestDetail`, which a Job's live pull request snapshot already is.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_pull_request_detail` (`GET /pull_request_details/:repository/:number`) | `PullRequestPage`: `base`, `author?`, `created_at?`, `additions`, `deletions`, `mergeable`, `review`, `approvals`, and `conversation[]`, `checks[]`, `files[]`, `commits[]` | One forge read. Comments and reviews merge in time order. Capped at the first 300 files, the first 250 commits and the latest 200 things said, with the count left out kept in the reader and not yet shown anywhere. `agent_access` `Yes` |
| `write_pull_request` (`POST /pull_request_writes/:repository`) | `WritePullRequest`: `pull_request`, `kind` (`comment`, `approve` or `request_changes`), `body?`, `session_id?` | Answers `PullRequestPage`, read again after the write, and every Job's `pr` row follows the forge. A comment or a request for changes with no body is a 422 `fleet.pull_request_write_blank`; a `session_id` the ledger does not know is a 422 `fleet.session_unknown`; the forge's refusal is a 409 `fleet.pull_request_write_refused`. `Helm only`, and Helm asks first (`pushes to shared`) |

**Skew.** A Fleet before 23.83 answers 404 to both, so a Bridge that opens the panel draws its tabs empty and cannot write.

The page's `head`, `body` and `threads`, the write's `comments` and `commit`, and the two operations the Reviews tab adds are in *The Reviews tab* below.

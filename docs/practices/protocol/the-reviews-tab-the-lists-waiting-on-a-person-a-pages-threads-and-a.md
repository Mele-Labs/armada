# The Reviews tab: the lists waiting on a person, a page's threads and a review's comments

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/pull-review.md`.
Bridge reads the pull requests waiting on a person from the forge through Fleet, opens any pull request of a served repository, and posts a review as one call with its line comments.
**Additive**: two operations, new DTOs in `ipc::pull_reviews`, and optional fields on the page and the write of *Protocol 23.83* above.
Bridge's half is `packages/protocol/src/pull-reviews.ts` and `pull-requests.ts`, written by hand like the rest.

| Where | Side | Change |
| --- | --- | --- |
| `get_review_lists` (`GET /review_lists`) | Bridge reads | `PullReviewLists`: `me`, `fetched_at`, and `requested[]`, `team[]`, `assigned[]`, `mine[]` of `PullReviewRow`, each capped at `MOST_PER_LIST`. `Bridge only` |
| `PullReviewLists.silent` | Fleet writes, Bridge reads | `Vec<PullReviewList>`, left out when empty: a list the forge would not answer, so Bridge keeps what it held. All four silent is a 500 `fleet.review_lists_unreadable` |
| `PullReviewRow.team` | Fleet writes, Bridge reads | The team asked, left out off the Team list |
| `PullRequestPage.head`, `.body` | Fleet writes, Bridge reads | The whole head commit and the description, left out where the forge gave none |
| `PullRequestPage.threads` | Fleet writes, Bridge reads | `Vec<PullReviewThread>`, left out when empty: `id`, `path`, `line?`, `side`, `resolved`, `comments[]`. A page whose threads the forge would not give is a 500 `fleet.pull_request_unreadable`, not a page |
| `WritePullRequest.comments` | Bridge writes, Fleet reads | `Vec<PullReviewComment>`, left out when empty: `kind` `line` (path, line, side), `file` (path) or `reply` (thread), each with `body`. Set, the write is one review with `kind` as the verdict, and `body` may be empty |
| `WritePullRequest.commit` | Bridge writes, Fleet reads | The commit the person read, left out for the branch as it stands |
| Fleet's check (`pull_request_panel.rs`, `write_of`) | Fleet refuses | 422 `fleet.pull_request_write_blank` for a comment with a blank `body`, as for a plain comment, nothing sent to the forge |
| `resolve_review_thread` (`POST /review_threads/:repository`) | Bridge writes | `ResolvePullThread` (`thread`, `resolved`), answers `PullThreadResolved`. A 409 `fleet.review_thread_refused` carries the forge's sentence. `Bridge only` |

**Additive.** Every new field is optional and left out when empty. The IDs differ, so a Bridge and a Fleet that differ are refused regardless.

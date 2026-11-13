# What Fleet remembers of a pull request: seen heads, viewed files, his review, the inbox, the compare and layouts

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/pull-review.md`, *Since you looked* and *Your review*.
**Additive**: eight operations, eight routes, three error codes, new types, and optional fields on six existing types; no event kind.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/pull-review-memory.ts`, written by hand, field for field.

`:repository` is the Manifest id. Fleet keys what it keeps by Manifest id, the repository as the forge names it, lowercase, and the pull request's number.

| Where | Side | Carries |
| --- | --- | --- |
| `mark_pull_request_seen` (`POST /pull_request_seen/:repository`) | Bridge writes | `MarkPullSeen` `{ number, head }`; answers `PullSeen` `{ head, at }`. Sent when he leaves the pull request or presses Mark as seen, never on open |
| `mark_file_viewed` (`POST /pull_request_viewed/:repository`) | Bridge writes | `MarkFileViewed` `{ number, path, commit, blob? }`; answers `PullViewedFile` with `still` true. `blob` is the file's `PullRequestFilePatch.blob` as he read it |
| `unmark_file_viewed` (`POST /pull_request_viewed/:repository/unmark`) | Bridge writes | `UnmarkFileViewed` `{ number, path }`, answered as sent. A file not marked is not an error |
| `get_pull_request_compare` (`GET /pull_request_compares/:repository/:number?since=&head=`) | Bridge reads | `PullRequestCompare`, tagged `outcome`: `patches` `{ since, head, files }`, each file a `PullRequestFilePatch`, or `base_gone` `{ since, head }` |
| `list_review_layouts` (`GET /review_layouts`) | Bridge reads | `ReviewLayouts` `{ layouts, picked }`; each `ReviewLayout` `{ name, arrangement, shipped? }`, Fleet's `Default` first |
| `save_review_layout` (`POST /review_layouts`) | Bridge writes | `SaveReviewLayout` `{ name, arrangement }`; answers `ReviewLayouts`. A kept name is replaced |
| `delete_review_layout` (`POST /review_layouts/delete`) | Bridge writes | `NamedReviewLayout` `{ name }`; answers `ReviewLayouts`. A deleted pick falls back to `Default` |
| `pick_review_layout` (`POST /review_layouts/pick`) | Bridge writes | `NamedReviewLayout` `{ name }`; answers `ReviewLayouts`. One pick for every pull request |
| `PullReviewRow.seen?`, `.my_review?`, `.inbox` | Fleet writes, Bridge reads | `get_review_lists`' rows: his seen head, his own latest submitted review `PullMyReview` `{ said, commit?, submitted_at? }` with `said` `approved`, `changes_requested` or `commented`, and `inbox`, one of `changed_since_you_looked`, `done`, `waiting_on_author`, `needs_first_look`, the first that holds. A row from an older Fleet reads as `needs_first_look` |
| `PullRequestPage.seen?`, `.viewed?` | Fleet writes, Bridge reads | `get_pull_request_detail`: his seen head, and each `PullViewedFile` `{ path, commit, viewed_at, still }` |
| `PullRequestPage.my_review?`, `.since_my_review?` | Fleet writes, Bridge reads | `get_pull_request_detail` and `write_pull_request`'s answer: his latest submitted review as a `PullMyReview`, and `PullSinceMyReview` `{ commits?, replies? }`, present exactly when `my_review` is. `commits` counts those after the review's commit, absent where that commit is not among the pull request's; `replies` counts comments by anyone else after `submitted_at` on the review threads he wrote on. Both absent where Fleet does not know who he is |
| `PullRequestFilePatch.blob?` | Fleet writes, Bridge reads | The forge's blob for the file's content at the head, on `get_pull_request_patches` and the compare. `get_pull_request_file` leaves it out |
| `StartReviewRun.since?`, `ReviewRun.since?` | Bridge writes, Fleet answers | A run of only the change from `since` to the head, kept for the range |
| `ReviewFinding.comment?` | Fleet writes, Bridge reads | One or two plain sentences he can post as they are, at most `COMMENT_AT_MOST` (280) characters. Absent on a finding kept before |

**Refusals.**

| Code | When |
| --- | --- |
| 422 `fleet.pull_review_memory_invalid` | A `head`, `commit`, `since` that is not a commit name; a blank path; a blank layout name, one past 60 characters, or `Default` saved or deleted; an `arrangement` that is not a JSON object with a `root`, or past 64 KiB; a pick of a name no layout has |
| 500 `fleet.pull_request_compare_unreadable` | `get_pull_request_compare` where the forge would not answer |
| 422 `fleet.review_range_invalid` | `start_review_run` with `since` on a fix, `since` not a commit name, or `since` the head itself |
| 409 `fleet.review_range_base_gone` | `start_review_run` with a `since` the forge no longer has. Nothing is kept or spent |

**The compare is the forge's, from the merge base.** After a rebase its files can hold changes from the target branch and not the author's.
**A lost base is an outcome, not an error**: `base_gone` is a 200, so Bridge offers the whole pull request rather than drawing a failure.

**Still viewed.** A file marked at the page's head is still viewed.
Marked at an older head, it is only while the forge's blob for it now is the one marked, which costs one forge read of the files, made only when such a mark exists.
With no blob the mark goes stale once the head moves.

**A ranged run's key.** The store keeps it at `<since>..<head>` in the `head` column of `review_runs`, so it is a key of its own beside the whole run at the same head and no column is added.
A kept result answers a press only with the same `since`.

**The finding comment rule** is [the review-run brief](../../contracts/agent-prompt/5j-the-review-run-brief.md)'s; a finding whose comment breaks it fails the run.

**A review Fleet wrote is served until the forge shows a newer one.** After `write_pull_request` makes a review, Fleet keeps it in `pull_request_reviewed`, stamped with its own clock after the forge took it, at the request's `commit` or else the head it reads next.
The page and the row serve whichever of that and the forge's latest has the later `submitted_at`, so a forge read that lags the write is never what he sees.
A plain comment is not a review and is not kept.

**Layouts.** `arrangement` is the pane tree as Bridge writes it, `{ root, focus, next, threadWas? }`, kept as text Fleet does not read into.
`Default` uses the tab kinds `pr-inbox`, `pr-changed`, `pr-files`, `pr-conversation`, `pr-checks`, `pr-findings` and `pr-comments`.
Classic reads none of these operations.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404, and every new field is optional on both sides.

**The event stream.** Untouched: no event carries any of it, and `review_run.changed` carries `since` only as a field of the run it already sends.

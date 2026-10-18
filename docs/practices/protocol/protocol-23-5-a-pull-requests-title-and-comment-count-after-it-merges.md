# Protocol 23.5: a pull request's title and comment count, after it merges

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Two optional fields on `JobDelivery`, additive.** `pull_request_title` and `pull_request_comments`
sit beside `pull_request` and are served only where it is. Unlike `pull_request_detail`, which is the
sweep's live reading and goes away when the pull request settles, both come off the Job's record, so
a merged Job still has them.

**`pull_request_title`** is written when Fleet opens the pull request, from the title it opened it
with, and again on every read of the forge, the settling read included, so a title edited on the
forge replaces it. Absent means no read has named it: a pull request opened before 23.5 and not read
since.

**`pull_request_comments`** is the count the sweep reads while the pull request is open, matching
what the forge shows: conversation comments, reviews that say something, and comments on lines of
the diff. The line comments cost one more forge call on a sweep turn that finds the pull request open,
the owner's call on 2 Oct 2026; the wire shape did not change, so there was no version bump. A turn
where either read goes unanswered keeps the last count. **Absent is unknown, never 0**: the pull
request settled before the rotation reached it open. Opening a new pull request clears it.

**No forge call on a Job read.** Both are written by the sweep and by opening the pull request. **Store V96** keeps
them, in two `jobs` columns.

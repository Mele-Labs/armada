---
name: land
description: Merge the pull request this session opened, once `ci` is green, without asking. Load when the owner types /land, or tells you to merge it, land it, or merge when green. `/land 1850` names the pull request.
---

# /land

**The owner typing `/land` is the go-ahead to merge. Do not ask again, and do
not wait for him to look at it.** He has already seen the preview, and `ci` is
the guard. `work-issue` step 6 has the order that led here.

**Which pull request.** The numbers he gives. With none, every pull request
this session opened and pushed, named from what you did here. If you cannot name
one, say so and stop. **Never merge a pull request this session did not open**
unless he names its number.

**Put it in the merge queue.** `main` has one (#1920): GitHub tests each pull
request on top of the ones ahead of it and merges it once `ci` passes there.

```
gh pr merge <n> --merge --auto
```

It prints `! The merge strategy for main is set by the merge queue`, and
`autoMergeRequest` can read null afterwards. Neither means it failed. Confirm
with `gh api graphql -f query='{repository(owner:"Mele-Labs",name:"armada"){pullRequest(number:<n>){isInMergeQueue mergeQueueEntry{state position}}}}'`.
A pull request whose own `ci` is still running gets auto-merge instead, and
joins the queue when it passes. Confirmed 7 Oct 2026: #1905 looked unqueued
and merged 40 minutes later from position 2.

`--squash`, `--rebase` and `--admin` are refused by `.claude/hooks/guard_merge.py`,
as is a push to `main`.

**A queued pull request is tested as it stands**, so nothing more may be pushed to
it. Before sending an agent back to its branch, take it out of the queue with the
GraphQL `dequeuePullRequest` mutation (or `disablePullRequestAutoMerge`, if it
is not queued yet), since the hook refuses `--disable-auto`. Or have the agent
open a new pull request. Confirmed 7 Oct 2026: #1867 merged while its agent was
still pushing the fix the owner was waiting for, and that fix needed #1883.

**A stacked pull request waits.** One whose base is another pull request's
branch merges into that branch. Queue it only after GitHub has retargeted it to
`main`.

**A change to `.github/` is green only when every job it touches is.** `ci`
on a pull request can pass while a job beside it fails, and a filtered job may
not run at all. Before queueing one, read `gh pr checks <n>` whole and require
each job that runs the changed step, `desktop_unit` for `node-setup`, to pass.
Confirmed 8 Oct 2026: #1963's `desktop_unit` failed in setup, `ci` passed, it
merged, and every Linux browser job on `main` failed until #1964.

**Then watch it until `gh pr view <n> --json state` says `MERGED`.** A queue run
can take well over 30 minutes. Leaving the queue unmerged (`OPEN`,
`isInMergeQueue` false) is the failure. Read the queue run with
`gh run list -e merge_group` and `gh run view --log-failed`, fix on the same
branch, push and queue it again. A conflict with what is ahead of it shows as
`UNMERGEABLE`: merge `origin/main` in once that has landed, never rebase, push and
requeue. Stop and tell him only where the failure is not yours to fix: the same
failure on `main`, a decision that is his, or one that survives two fixes.

**A queue run can die without failing a job.** Confirmed 7 Oct 2026, in a GitHub
Actions outage: #1918's run ended `failure` with every job passed and no `ci`, so
the queue waited on it. `gh run rerun <id>` of that run released it.

**When it has merged, give back what this session cut:** `armada worktree
release <slot path>` for a slot you leased, `git worktree remove` for a scratch
worktree. Report one line: the pull request, the merge commit, and what you
released. A merge does not restart Fleet or Bridge; `restart-app` and
`preview-app` do that, and only when he asks.

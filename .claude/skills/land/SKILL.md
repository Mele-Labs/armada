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

**Merge it with a merge commit, and let GitHub wait for `ci`:**

```
gh pr merge <n> --merge --auto
```

The repository allows merge commits and auto-merge only. `--squash`, `--rebase`
and `--admin` are refused by `.claude/hooks/guard_merge.py`, as is a push to
`main`. GitHub can refuse `--auto` for a pull request whose checks have already
passed; then run `gh pr merge <n> --merge` itself, after the check below.

**Then watch it until `gh pr view <n> --json state` says `MERGED`**, the way
`work-issue` says to watch `ci`. A red run comes back to you: read
`gh run view --log-failed`, fix on the same branch and push, and it merges when
the new run passes. A conflict: merge `origin/main` in, never rebase, and push.
Stop and tell him only where the failure is not yours to fix: the same failure on
`main`, a decision that is his, or one that survives two fixes.

**Before a hand merge (no `--auto`), the green must be for the pull request's
current head.** Check `gh pr view <n> --json headRefOid` against the latest run's
`headSha` for the branch, and that `mergeable` is `MERGEABLE`; GitHub says
`UNKNOWN` for a few seconds, so wait. Confirmed 7 Oct 2026: a watcher reported
green for #1844 from a run on the commit before a conflict, and the pull request
was still `CONFLICTING` with 15 commits to catch up.

**When it has merged, give back what this session cut:** `armada worktree
release <slot path>` for a slot you leased, `git worktree remove` for a scratch
worktree. Report one line: the pull request, the merge commit, and what you
released. A merge does not restart Fleet or Bridge; `restart-app` and
`preview-app` do that, and only when he asks.

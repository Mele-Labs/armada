---
name: agent-worktrees
description: Leasing an agent a warm worktree slot and releasing it, or cutting a worktree and giving it back — the cleanup that has to happen when a branch merges, and why a build directory is not the thing to delete. Load before dispatching agents at issues, and before merging their work.
---

# Agent worktrees

**A worktree outlives the agent that asked for it, and nothing reclaims it.**
`armada clean` gives back a *Job's* worktrees under `.armada/worktrees/`. Nothing
gives back an *agent's* under `.claude/worktrees/`, so they accumulate silently
until a disk fills mid-run.

That has happened once, and it cost the session: **74 worktrees, 220 GB, three
agents dead at zero bytes free** with uncommitted work in each. This skill is
what that taught.

## Lease a slot rather than cut a tree

**Every cut tree builds the workspace cold.** On 2 Oct 2026 there were 26 agent
worktrees, each building from nothing, on a machine loaded past 20 on 18 cores.
The repository keeps a pool of warm slots instead — `setup.worktrees` in
`armada.yml`, eight by default — and the pool is the cap.

```
slot=$(armada worktree lease <branch>)   # waits while every slot is held; never `path=`, zsh ties it to PATH
armada worktree --status                 # who holds each, and since when
armada worktree release <path>           # after the pull request merges
```

**A lease belongs to the process that took it, and a restart orphans it.** The
slot then reads free or stranded, and the next lease takes a clean one back for
another branch. Confirmed 6 Oct 2026: after a cmux crash a session ran
`git merge origin/main` in what it still thought was its slot, which another
session had been given, and put a merge commit on that session's branch (undone
with `git reset --keep`). After any restart run `git -C <slot> branch
--show-current` before another git command there, and re-attach a branch with
`armada worktree lease --existing <branch>`. A lease started from a subshell
(`( ... &)`) fails with "the process this was run from could not be read"; run
it in the foreground or as the shell tool's own background task.
**A lease taken in one tool call can read free when the call ends**, because the
holder is the shell that ran it. Confirmed 6 Oct 2026, twice in one session:
`--status` said `holder is gone after 0m` on slots just leased, and another
session was given each, with an agent's uncommitted work landing on its branch.
Take the lease and do the work that needs the slot in one background task, and
commit before it ends; a branch with its commits is safe whoever has the slot.

**The owner sizes the pool, not the dispatcher.** He adds, removes and closes
slots from Cleanup, and that machine's pool stands in for `setup.worktrees`. A
closed slot reads `closed` in `--status` and is never leased; a lease that
waits on closed slots is waiting on him, so say so rather than opening one.

**The dispatcher leases, and the path goes in the brief.** Dispatch without
`isolation: "worktree"`, and tell the agent to work only at that path, with
absolute paths and `git -C <path>` — the rest of this skill still applies to it.
The lease is held for the session that ran the command, so lease from the
session that will release it.

**Release at the merge, never remove.** A release refuses while anything is
uncommitted, because an untracked or modified file does not survive the detach.
Bridge's Clear and Release commit the files first, and so does Fleet for a Job that is killed or fails, so that refusal is the pool's alone. A commit on the slot's branch is enough: the branch keeps it, so a push is
optional and an unmerged branch releases. It detaches the slot and leaves its
`target/`; the branch stays until you delete it, so do not `git branch -D` one
whose commits are not landed. A slot whose session ended without releasing is
taken back by the next lease when it is clean and its commits are on its branch
— otherwise it reads `stranded` in `--status` and stays held.

**To free a slot with work half done**, commit it on the branch and release. The
pool can also do the commit: a parked slot's work is committed with a `WIP:`
message and the slot freed, with no push. `armada worktree lease --existing
<branch>` puts a slot back on that branch at its tip.

**Only where `armada` does not know the verb**, because the installed binary
predates it, fall back to `isolation: "worktree"` and everything below.
`scripts/restart` installs the current binary.

**The pool does not reach `.claude/worktrees/`.** Those already cut are left as
they are; give each back by the rules below.

## Give it back when the branch merges

**The merge is the moment.** Not "later", not "when disk is low" — a branch that
is in `main` has a worktree that holds nothing that is not also in `main`.

```
git worktree remove --force <path>
git branch -D <branch>
```

Do both. A branch left behind with no worktree is cheap; a worktree left behind
is not.

**The cleanup happens after the pull request merges**, never when you
open it. Check it merged with `gh pr view <branch> --json state` (it reads
`MERGED`) before removing a worktree, and run the three checks below first,
every time: nothing here can tell your worktree from one another agent is still
writing in. Delete the remote branch with `git push origin --delete <branch>` or
GitHub's delete-branch button. A branch still queued on the merge line is
cleaned up by `scripts/land`, which prints these two commands when it lands it.

**Removing the worktree is the fix. Deleting its `target/` is not.** A build
directory rebuilds; a worktree that nobody removes stays forever and takes a new
build directory with it the next time anyone touches it. Clearing `target/` and
leaving the worktree is the move that has to be made twice.

**A worktree the harness locked refuses `remove --force`; unlock it first.**
`git worktree unlock <path>`, then remove. Confirmed 1 Oct 2026: a cleanup fell
back to `rm -rf` when the remove refused, which deleted a still-registered tree
and needed an unlock and a `prune` afterwards to clear the listing. Both trees
were clean and pushed, so nothing was lost — but that was the three checks
below, not the fallback.

**Deleting a merged branch closes every pull request based on it.**
Confirmed 1 Oct 2026: #1729 was stacked on #1721's branch, #1721 was merged and
its branch deleted, and #1729 closed unmerged with its base gone. The work
survived on its own branch and came back as #1731 after a rebase. Stack a
branch only if it will be rebased onto `main` before it opens, or open it
against `main` from the start.

**`Directory not empty` means something is still writing there, and it is
usually you.** After an `isolation: "worktree"` agent finishes, the dispatching
session's working directory moves into its worktree and the session's own
rust-analyzer starts building there. `git worktree remove --force` then
deregisters the tree and leaves a gigabyte behind. Once the three checks below
pass, `rm -rf` what is left. Confirmed 2026-09-11.

**zsh does not split an unquoted variable, so a cleanup loop can report work it
never did.** Confirmed 14 Sep 2026: `for pair in "<dir> <branch>"; do set --
$pair` kept both words as one argument, every `git -C` pointed at a path that did
not exist, and the loop printed `removed` for two worktrees still registered.
Loop over one name, quote every path, and read `git worktree list` afterwards
rather than the loop's own output.

## Three things that must survive

Check all three before removing anything. Merged-ness alone is not enough.

| Condition | How to tell | Why |
|---|---|---|
| **An agent is still working in it** | it is running, or its last report said "not committed" | Its edits are on disk and nowhere else |
| **The tree is dirty** | `git status --porcelain` is non-empty | Uncommitted work looks identical to no work from outside |
| **The branch is not in `main`** | `git merge-base --is-ancestor <branch> main`, and where that says no, `gh pr list --head <branch> --state all` | Commits exist only there |

**A PR that merged rebased leaves a branch that fails the ancestor check.**
Confirmed 2026-09-11: two days after its PR merged, `fleet/the-check-record-on-the-wire`
still read as unmerged. An agent reported it as work that would conflict with its
own, and that reached the owner as a rebase still owed. `gh pr list --head`
settled it in one line.

**The dirty check is the one that catches the dangerous case.** An agent that has
written files but not committed is on a branch with *no commits ahead of `main`* —
so it reads as fully merged, and removing it destroys work that was never
anywhere else.

**Never `git stash` in a worktree; set work aside with a WIP commit.** The stash
is one ref shared by every worktree and every session. Confirmed 13 Sep 2026: an
agent and another session pushed to `refs/stash` at the same moment, the
agent's pop applied the other session's changes into its tree, and its own work
came back only through `git fsck --unreachable`.

## An agent that cuts no worktree takes somebody else's

**A brief that says to cut one is not evidence that one was cut.** Confirmed
2026-09-09, three times in one session: an agent told to
`git worktree add .claude/worktrees/<name> -b <branch>` ran `git checkout -b`
instead, inside the worktree the dispatching session was already working in. The
dispatching session's `HEAD` moved under it mid-command, and it found out when
`git log` printed somebody else's commits.

**The first time it cost an agent everything it had not committed.** Two actors
in one tree, and the second one's checkout reset the first one's files. That
agent had been told to commit after every step and had not — which is why that
instruction is in every brief, and why it is not sufficient on its own.

Two rules, and the second is the one nobody thinks of:

- **Verify before the first edit, not after.** `git worktree list`, and confirm
  the path you were given is in it and is yours. One command, before anything is
  written.
- **A dispatching session must not assume its own worktree is still its own.**
  Check `git rev-parse --abbrev-ref HEAD` before any command whose meaning
  depends on which branch is checked out — a rebase, a stash, a `checkout --`.
  The session that lost work here had been in that worktree for hours.

**A `cd` does not survive to the next tool call.** This is how a correctly cut
worktree still ends up unused. The agent runs `cd <worktree>`, writes with
relative paths, and every call after the first resolves against the main
checkout instead. Confirmed twice on 12 Sep 2026, in one session: one agent left
13 files on `main` this way and another 22, and both had been told in their brief
to use absolute paths. Neither noticed; the dispatching session found it.

**An agent may be refused a worktree outright, and then it lands in yours.**
Confirmed 12 Sep 2026: two agents dispatched in parallel were each refused
`git worktree add` and `git -C` by the sandbox — one was refused its branch
twice as a "shared-resource change" — so both committed onto the branch the
dispatching session's own worktree held. Nothing was lost, because their scopes
were disjoint and each staged by explicit path, but the dispatching session
learned it from the first agent's report rather than from the brief. **Plan for
it rather than forbidding it**: while agents run, the dispatching session's tree
is not its own, so it cannot commit — a `git add` sweeps their half-written
files into somebody else's commit — and it cannot run the Checks, because the
run captures a mid-write tree. Tell each agent to stage by explicit path, expect
one branch rather than two, and treat your own tree as read-only from dispatch
until the last one reports.

Three habits, and they are cheap:

- **Prove the worktree, with output you read.** `git -C <repo root> worktree list`
  says the path exists, and `git -C <worktree> rev-parse --abbrev-ref HEAD` says
  it is your branch. Before the first edit, not after.
- **Every path written starts with the worktree's own prefix**, and every git
  call is `git -C <absolute worktree path>`. Where a `cd` is unavoidable, keep it
  inside a subshell in the same call: `(cd <worktree> && …)`.
- **Check `git -C <repo root> status --porcelain` prints nothing before you
  report.** It is the one line that catches this, and it catches it while the
  work is still yours to move.

**The hook refuses it now.** `.claude/hooks/guard_write.py` denies an `Edit` or
`Write` whose path lands in a checkout that has `main` checked out, and says how
to cut a worktree instead. It has no gate half and cannot have one: writing in
the wrong checkout leaves no trace in a diff, so there is nothing to catch
afterwards — which is why it was written into prose three times before it was
written into a hook.

**A harness that pins a shell to a worktree makes this worse, not better.** When
the dispatching session moved itself out, every agent still pinned to the old
path had its next command refused, mid-task. Moving is safe only once nothing
else is working there.

## Whose worktree is it

**A worktree named `agent-<id>` names the session that spawned it, if you look
in the transcripts rather than ask around.** `grep -l "agentId: <id>"
~/.claude/projects/<this project>/*.jsonl` finds the one parent transcript, and
its last `"customTitle"` is the session's name; message that session. Confirmed
1 Oct 2026: a branch blocking the merge line (#1709) was traced by messaging
three sessions in turn, none of them its owner, for twenty minutes, until the
owner asked how we could not tell who made it. One grep answered it.

**An agent's `git merge origin/main` or `git push` can be refused by the
permission check**, as can removing its own worktree. Brief it to stop and
report rather than retry; the dispatching session asks the owner, then runs it.
That is the owner's standing answer (1 Oct 2026). Confirmed the same day: a
merge-line agent stopped with two commits unpushed, and its branch landed only
after the dispatching session merged and pushed on his say-so.

## Sweeping when it has already got away

Audit before deleting, and print what will be kept rather than what will go — the
keep list is short and readable, and a mistake in it is visible.

**When the disk is already full, clear `target/` first and audit worktrees
second.** Confirmed 6 Oct 2026: a `du` of every worktree was still running five
minutes in while the owner waited at 7 GiB free, and was abandoned. Removing
every `target/` took the disk to 535 GiB free and lost no work. Skip any
checkout where a build is running. That run also removed slot-9's `target/`
while an agent was compiling there, and two of its builds failed.

`sed`, `cut` and `sort` have been unavailable in this environment's non-interactive
shell. Prefer a `python3` heredoc over a pipeline for anything that has to parse
`git worktree list`.

## Build size is a separate problem with a separate fix

`[profile.dev]` sets `debug = "line-tables-only"`, which took `target/debug` from
**32 GB to 1.8 GB** on this workspace with every test still passing. Rust links
statically, so without it each of a thousand-odd test binaries carries its own
copy of the full debug info for the whole dependency graph.

**Stale build output is trimmed, and a slot stays warm.** Cargo never deletes an
old dependency build, incremental session or test binary, and on 6 Oct 2026 24
checkouts reached 50 GB each. A `release` (and `armada clean` giving back a
Job's slot) drops every file under the slot's `target/` that no build has
written in 14 days, and removes the whole `target/` if it is still over 20 GiB.
Fleet also sweeps every checkout's `target/` once an hour: the main checkout,
slots held or free, bases, land trees, preview, and the Job and agent worktrees.
A `target/` with a cargo build running in it is skipped. Settings:
`slot-build-trim-after-days`, `slot-build-ceiling-gib`,
`build-sweep-interval-minutes` (constants until #1821). Outside a full disk, do
not delete `target/` by hand to save space.

**Do not share one `CARGO_TARGET_DIR` between worktrees to save space.** It was
tried: two manifest directories against one target poisoned the incremental cache
and produced phantom link failures on `main` that took a `cargo clean` to clear.
Cargo also locks the directory, so parallel agents would serialise their builds.

## What to say when you cannot clean up

**A blocked agent should stop and report, not free space on its own judgement.**
Two did exactly that when the disk filled, and both were right to: one asked
before deleting a 37 GB `target/`, which was not the problem and would have cost
a cold rebuild for nothing.

Deleting a build directory is recoverable. Deleting a worktree with uncommitted
work is not, and it looks the same from outside.

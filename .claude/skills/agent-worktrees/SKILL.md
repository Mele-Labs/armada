---
name: agent-worktrees
description: Leasing an agent a warm worktree slot and releasing it, or cutting a worktree and giving it back — the cleanup that has to happen when a branch merges, and why a build directory is not the thing to delete. Load before dispatching agents at issues, and before merging their work.
---

# Agent worktrees

**A worktree outlives the agent that asked for it, and nothing reclaims it.**
`armada clean` gives back Fleet's worktrees under `.armada/worktrees/`. Nothing
gives back an *agent's* under `.claude/worktrees/`, so they accumulate silently
until a disk fills mid-run.

That has happened once (74 worktrees, 220 GB, three agents dead at zero bytes
free with uncommitted work in each). This skill is what that taught.

## Lease a slot rather than cut a tree

**Every cut tree builds the workspace cold.** On 2 Oct 2026 there were 26 agent
worktrees, each building from nothing, on a machine loaded past 20 on 18 cores.
The repository keeps a pool of warm slots instead — `setup.worktrees` in
`armada.yml`, eight by default — and the pool is the cap.

```
slot=$(armada worktree lease <branch>)   # waits while every slot is held; never `path=`, zsh ties it to PATH
armada worktree --status                 # who holds each, the branch checked out, and since when
armada worktree release <path>           # after the pull request merges; Fleet tells you the slot is free, and gives it back when your turn ends with nothing new in it
```

**A lease belongs to the process that took it, and a restart orphans it.** The
slot then reads free or stranded, and the next lease takes a clean one back for
another branch (6 Oct 2026: after a cmux crash a job ran `git merge origin/main`
in a slot another job now held). After any restart run `git -C <slot> branch
--show-current` before another git command there, and re-attach with `armada
worktree lease <branch>`: an existing branch, here or on `origin`, is checked
out at its own tip and never reset to the base (13 Oct 2026). `--existing` is
the same but refuses a name nothing has instead of cutting it. A slot whose
holder is gone and which holds uncommitted files is refused rather than taken
back, so commit there first. A lease started from a subshell (`( ... &)`) fails
with "the process this was run from could not be read": run it in the foreground
or as the shell tool's own background task.

**A lease taken in one tool call could read free when the call ended**, because
the holder was a shell that ran it (`$(armada worktree lease x | tail -1)`;
6 Oct 2026, twice: another job was given each slot, and an agent's uncommitted
work landed on its branch). The holder is now the first process above the
command that is not a shell, so an older binary still has the old holder:
commit before a call ends, because a branch with its commits is safe whoever has
the slot. The same lapse happens to a lease made in a background shell, or
before the job restarted (7 Oct 2026: a second job's `git commit -a` swept an
agent's half-built files into #1845). So lease in the foreground, check `git
branch --show-current` before every commit, commit early and push after the
first commit.

**A job Bridge hosts holds its leases by its job id, not a process**
(`ARMADA_SESSION`, 13 Oct 2026): Fleet ends and resumes its process when it moves
the job into a slot. Such a lease shows as `job <session id>` in `--status`, is
never taken back, and is parked when the job closes.

**The owner sizes the pool, not the dispatcher.** He adds, removes and closes
slots from Cleanup, and that machine's pool stands in for `setup.worktrees`. A
closed slot reads `closed` in `--status` and is never leased; a lease that
waits on closed slots is waiting on him, so say so rather than opening one.

**The dispatcher leases, and the path goes in the brief.** Dispatch without
`isolation: "worktree"`, and tell the agent to work only at that path, with
absolute paths and `git -C <path>` — the rest of this skill still applies to it.
The lease is held for the job that ran the command, so lease from the
job that will release it.

**Release at the merge, never remove.** A release refuses while anything is
uncommitted, because an untracked or modified file does not survive the detach.
Bridge's Clear and Release commit the files first, and so does Fleet for a Job that is stopped, so that refusal is the pool's alone. A commit on the slot's branch is enough: the branch keeps it, so a push is
optional and an unmerged branch releases. It detaches the slot and leaves its
`target/`; the branch stays until you delete it, so do not `git branch -D` one
whose commits are not landed. A slot whose job ended without releasing is
taken back by the next lease when it is clean and its commits are on its branch
— otherwise it reads `stranded` in `--status` and stays held.

**In a slot, stage named paths, never `git commit -a` or `git add -A`.** A slot
can hold another job's uncommitted tracked edits, and `-a` commits them as
yours. (7 Oct 2026: a `commit -a` swept three half-built `crates/api` files into a
pull request, `api` did not compile, and `ci` went red.) Read `git status`
first and `git add` each path you changed.

**To free a slot with work half done**, commit it on the branch and release. The
pool can also do the commit: a parked slot's work is committed with a `WIP:`
message and the slot freed, with no push. `armada worktree lease --existing
<branch>` puts a slot back on that branch at its tip.

**Only where `armada` does not know the verb**, because the installed binary
predates it, cut a tree by hand (`git worktree add .claude/worktrees/<name> -b
<branch>`), dispatch without `isolation`, and follow everything below.
`scripts/restart` installs the current binary.

**`.claude/hooks/guard_agent.py` refuses an Agent call with `isolation:
"worktree"`.** The preview merges only slots and the merge line, so a tree the
harness cut is invisible to it. (7 Oct 2026: the owner's preview lacked three
branches.) There is no escape hatch: a lease waits while the pool is full, and a change to
`scripts/preview` is built in a slot like any other. `isolation: "remote"` and
no isolation pass.

**The pool does not reach `.claude/worktrees/`.** Those already cut are left as
they are; give each back by the rules below.

## Give it back when the branch merges

**The merge is the moment.** Not "later", not "when disk is low" — a branch that
is in `main` has a worktree that holds nothing that is not also in `main`.

**A job Fleet hosts calls `finish_up`**, never `git branch -D` and never the
owner. It releases every slot the job holds and deletes each merged branch
that is its own: the leased one, any a slot of its is on, and every pull
request's it holds. Fleet's merge notice says so. Ending the job parks its
slot and keeps the branch, so leaving it to the end leaves the branch behind.
(15 Oct 2026, #2343: a job was refused `git branch -D` and asked the owner to
delete the branch.) The commands below are for a job in a terminal, and for a
tree cut by hand.

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
GitHub's delete-branch button.

**Removing the worktree is the fix. Deleting its `target/` is not.** A build
directory rebuilds; a worktree that nobody removes stays forever and takes a new
build directory with it the next time anyone touches it. Clearing `target/` and
leaving the worktree is the move that has to be made twice.

**A worktree the harness locked refuses `remove --force`; unlock it first.**
`git worktree unlock <path>`, then remove. (1 Oct 2026: a fallback to `rm -rf` deleted a still-registered tree and needed
an unlock and a `prune`.) Never fall back to `rm -rf` on a refusal.

**Deleting a merged branch closes every pull request based on it.**
(1 Oct 2026: #1729, stacked on #1721's branch, closed unmerged when #1721's
branch was deleted.) Stack a branch only if it will be rebased onto `main` before it opens, or open it
against `main` from the start.

**`Directory not empty` means something is still writing there, and it is
usually you.** After an `isolation: "worktree"` agent finishes, the job's own
rust-analyzer starts building in the tree, so `git worktree remove --force`
leaves a gigabyte behind (2026-09-11). Once the three checks below pass, `rm -rf`
what is left.

**zsh does not split an unquoted variable, so a cleanup loop can report work it
never did** (14 Sep 2026: it printed `removed` for two worktrees still
registered). Loop over one name, quote every path, and read `git worktree list`
afterwards.

## Three things that must survive

Check all three before removing anything. Merged-ness alone is not enough.

| Condition | How to tell | Why |
|---|---|---|
| **An agent is still working in it** | it is running, or its last report said "not committed" | Its edits are on disk and nowhere else |
| **The tree is dirty** | `git status --porcelain` is non-empty | Uncommitted work looks identical to no work from outside |
| **The branch is not in `main`** | `git merge-base --is-ancestor <branch> main`, and where that says no, `gh pr list --head <branch> --state all` | Commits exist only there |

**A PR that merged rebased leaves a branch that fails the ancestor check.**
`gh pr list --head`
settles it in one line (2026-09-11: a merged branch read as unmerged and reached
the owner as a rebase still owed).

**The dirty check is the one that catches the dangerous case.** An agent that has
written files but not committed is on a branch with *no commits ahead of `main`* —
so it reads as fully merged, and removing it destroys work that was never
anywhere else.

**Never `git stash` in a worktree; set work aside with a WIP commit.** The stash
is one ref shared by every worktree and every job. (13 Sep 2026: an agent's pop applied
another job's changes, and its own work came back only through `git fsck
--unreachable`.)

## An agent that cuts no worktree takes somebody else's

**A brief that says to cut one is not evidence that one was cut** (2026-09-09,
three times: an agent ran `git checkout -b` inside the dispatching job's worktree
and moved its `HEAD` mid-command, and the first lost everything it had not
committed). That is why *commit after every step* is in every brief, and why it
is not sufficient on its own.

Two rules, and the second is the one nobody thinks of:

- **Verify before the first edit, not after.** `git worktree list`, and confirm
  the path you were given is in it and is yours. One command, before anything is
  written.
- **A dispatching job must not assume its own worktree is still its own.**
  Check `git rev-parse --abbrev-ref HEAD` before any command whose meaning
  depends on which branch is checked out — a rebase, a stash, a `checkout --`.
  The job that lost work here had been in that worktree for hours.

**A `cd` does not survive to the next tool call.** An agent that runs `cd
<worktree>` and writes with relative paths resolves every call after the first
against the main checkout (12 Sep 2026: one left 13 files on `main`, another 22,
both told to use absolute paths).

**An agent may be refused a worktree outright, and then it lands in yours**
(12 Sep 2026: two agents refused `git worktree add` committed onto the
dispatching job's branch). **Plan for it rather than forbidding it**: while
agents run, the dispatching job's tree is not its own, so it cannot commit (a
`git add` sweeps their files into its commit) or run the Checks (they capture a
mid-write tree). Tell each agent to stage by explicit path, expect one branch,
and treat your own tree as read-only from dispatch until the last one reports.

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
afterwards — which is why it is a hook and not prose.

**A harness that pins a shell to a worktree makes this worse, not better.** When
the dispatching job moved itself out, every agent still pinned to the old
path had its next command refused, mid-task. Moving is safe only once nothing
else is working there.

## Whose worktree is it

**A worktree named `agent-<id>` names the job that spawned it, if you look
in the transcripts rather than ask around.** `grep -l "agentId: <id>"
~/.claude/projects/<this project>/*.jsonl` finds the one parent transcript, and
its last `"customTitle"` is the job's name; message that job. One grep
answers it (1 Oct 2026: #1709 was traced by messaging three jobs in turn).

**An agent's `git merge origin/main` or `git push` can be refused by the
permission check**, as can removing its own worktree. Brief it to stop and
report rather than retry; the dispatching job asks the owner, then runs it.
That is the owner's standing answer (1 Oct 2026).

## Sweeping when it has already got away

Audit before deleting, and print what will be kept rather than what will go — the
keep list is short and readable, and a mistake in it is visible.

**When the disk is already full, clear `target/` first and audit worktrees
second.** (6 Oct 2026: a `du` of every worktree was
still running five minutes in at 7 GiB free; removing every `target/` freed 535
GiB and lost no work.) Skip any checkout where a build is running (that run
broke two builds in slot-9).

`sed`, `cut` and `sort` were unavailable in one environment's non-interactive
shell, and ran in an interactive job on 6 Oct 2026, so check before relying on
either. Prefer a `python3` heredoc over a pipeline for anything that has to parse
`git worktree list`. It parses; it is not for editing files, which is `Edit`.

## Build size is a separate problem with a separate fix

`[profile.dev]` sets `debug = "line-tables-only"`, which took `target/debug` from
**32 GB to 1.8 GB** on this workspace with every test still passing. Rust links
statically, so without it each of a thousand-odd test binaries carries its own
copy of the full debug info for the whole dependency graph.

**Stale build output is trimmed, and a slot stays warm.** Cargo never deletes an
old dependency build, incremental job or test binary, and on 6 Oct 2026 24
checkouts reached 50 GB each. A `release` (and `armada clean` giving back a
slot) drops every file under the slot's `target/` that no build has
written in 14 days (never a build script's `out/`, which cargo would not rebuild), and removes the whole `target/` if it is still over 20 GiB.
Fleet also sweeps every checkout's `target/` once an hour: the main checkout,
slots held or free, bases, land trees, preview, and the agent worktrees.
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

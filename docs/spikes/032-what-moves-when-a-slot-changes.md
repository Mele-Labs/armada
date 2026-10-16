# Spike 32 — What can the slot list be driven by, instead of a timer?

**Answer: FSEvents on each slot's git directory and on the work tree of slots nobody holds, and
nothing on a held slot's work tree.** `git status` is not made cheaper by libgit2 or by git's
untracked cache enough to matter, so it stays what it was and is asked for less.

Measured 15 Oct 2026 on macOS 27, 12 cores, load average 16 to 24, git 2.55.0, this repository
(6,407 tracked files, ten slots). Everything was read-only against the owner's slots, except the
builds, which were this branch's own in its own slot.

## A recursive watch on a slot, during a real build

A watch on a slot's root takes in `target/`. Counted by category through one real
`cargo build -p armada` (101 seconds, 107 with the watch's own tail), with every stream below
watching the same slot at once. Driver: a Swift program over `FSEventStreamCreate`, so the flags
are the ones named, not a library's.

| Stream | Events | Peak per second | Of them in `target/` |
|---|---|---|---|
| Slot root, per-file events, no latency (what `notify` asks for) | 8,600 | 811 | 8,600 |
| Slot root, per-directory events, 1s latency | 6,646 | 701 | 6,646 |
| Slot root, per-file events, 1s latency | 6,701 | 704 | 6,701 |
| Slot root, exclusion paths set (`target`, `node_modules`, six more) | **0** | 0 | 0 |
| The slot's own git directory | 18 | 4 | 0 |
| The repository's `.git`, `objects/` not excluded | 329 | 201 | 0 |

A `cargo check -p fleet` is a tenth of it (2,084 events, peak 310). So the recorded reason
`crates/armada/src/watching.rs` gives holds: tens of thousands of events is what a larger build
or several builds make, and **latency coalescing does not help**, since the events are on
different files and directories.

**What it costs the watcher.** All seven streams together, 107 seconds: 0.48 CPU-seconds. A
callback is about 15 microseconds, so a flood is not the Fleet's CPU. A slot building flat out
is about 80 events a second, and ten of them 800.

**Exclusion paths remove it at the source, and are not usable.** `FSEventStreamSetExclusionPaths`
takes at most 8 directories; a ninth was refused (`exclusion refused`). This repository has at least ten
`node_modules` directories under its slots' roots, before `target`. `notify` does not expose it
and neither does `fsevent-sys`, so it would be an FFI call and an `unsafe` block in a crate
that forbids them.

## Reading a slot's status

`git status --porcelain` in a slot of this repository, `GIT_OPTIONAL_LOCKS=0`, load 20:

| How | Wall (median) | CPU per run | Processes |
|---|---|---|---|
| `git status --porcelain` | 63 to 75 ms | 87 ms | 1 |
| libgit2 `statuses()`, untracked included | 82 to 90 ms | 74 ms, process start included | 0 |
| `git status -uno` (tracked files only) | 28 to 32 ms | 52 ms | 1 |
| `git status` with `core.untrackedCache` | 29 ms | 63 ms | 1 |
| `git status` with the built-in fsmonitor and the untracked cache | 54 ms | 17 ms | 1, and a daemon per slot |

**In-process status is slower, not cheaper.** The untracked walk is two thirds of the time, and
libgit2 does it without the cache. The untracked cache halves the wall time and is a setting in
the repository's config, shared by every worktree, and an index write the read-only reads here
(`GIT_OPTIONAL_LOCKS=0`, so an agent's `git commit` is not refused) never make. The daemon is a
process per slot. None is taken.

## How quiet a free slot is

`.armada/slots` watched for 180 seconds, per-file events, with four slots free: **0 events in any
free slot**, none in a `.lease` record, 96 events in one held slot's `node_modules`. A sample of
one, at one moment.

## What Fleet spawned

On the owner's Fleet (before #2367), `ps` sampled every ~90 ms for 90 seconds with Bridge open:
**656 children, 437 a minute** (110 `git status`, 111 `git rev-parse`, 84 `git rev-list`,
32 `gh`). A lower bound, since a process that lives under a poll is missed.

On a scratch Fleet of this branch's two binaries, `git` counted by a shim on `PATH`, four free
slots, `GET /worktrees` every two seconds as Bridge does, 120 seconds each:

| | Before | After |
|---|---|---|
| Nothing changes | 47 a minute (96 `status`) | **26 a minute** (52 `status`) |
| A file in a free slot written or removed every 15s | 92 a minute | 76 a minute |

Before is main with #2367; the saving here is the interval and reading the slots touched. The
second row is dominated by the one slot shown holding work, whose reading is `branch`,
`rev-parse`, `rev-list` and `log` each time it is read. **From the edit to the list showing
it: 0.5 to 2.4 seconds** (six runs: 1.57, 1.75, 1.57, 0.49, 2.44, 0.5), against up to 5 before.

## What was chosen

- Watch the repository's `.git` once, recursively, and act on `refs/` for the base branch,
  `packed-refs` and `worktrees/<slot>/`. A branch or a tag moving is not the base moving, so it
  reads nothing: a commit anywhere would otherwise read every slot.
- Watch the work tree of a slot nobody holds, and stop when it is leased. Events under `target/`,
  `.git`, `.gitnexus` or any `node_modules` are dropped.
- **`FETCH_HEAD` is not watched**, though the task named it. A fetch that changes the base moves
  `refs/remotes/origin/<base>`, and one that does not changes nothing a slot shows.
- Read the slots that moved. Every `STALE_AFTER` (10s) read every slot, measured from the last
  such read.

## What no event covers

- **A holder that dies.** Its record stays and the process is gone; nothing is written.
- **A change to the pool's size by another process** (`armada worktree add`).
- **A deleted branch** that leaves a free slot's commits unnamed.
- **Events lost across a stream restart**: `notify` makes a new stream each time a path is added
  to it, and an edit in the moment between is not delivered.

Each is shown by the read at the deadline, at most 10 seconds after.

## Left open

The deadline's read still asks `git status` of every free slot: 26 spawns a minute with four.
Free slots are watched, so trusting the watch and checking only HEAD and the refs on the
deadline would take that to a few; the owner's bound is what a missed event may wait, and that
is his to move.

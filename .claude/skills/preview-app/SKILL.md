---
name: preview-app
description: Merge every in-flight branch into a preview and move the owner's Fleet and Bridge onto it, or onto a merged fix with `scripts/restart` — including past a working Drone with --adopt. Load before running `scripts/preview`, `scripts/preview --restart` or `scripts/restart`.
---

# Updating the preview, and moving the owner onto it

`docs/practices/running-locally.md`, *A preview of unlanded work*, is what the
scripts do and refuse. This is when to reach for which form, and what to tell
the owner.

## When to restart

- After merging a fix to Fleet (`crates/`) or Bridge (`apps/desktop/`,
  `packages/`) that the owner is about to rely on. A Job was left undelivered on
  12 Sep because Fleet kept serving a binary from before the fix.
- **After merging a change to `.armada/workflows/`.** Fleet reads the catalogue
  once at boot and freezes a copy onto every Job at creation. Confirmed 18 Sep
  2026: a Judge criterion merged at 11:32 and a Job dispatched at 16:37 ran the
  three criteria the step had before it, on a Fleet up since 11:24.
- Not after every merge. A change nobody is watching for waits for his own
  restart, and each one is a prompt he has to read.

## The forms

| Wanted | Run | Moves the owner's Fleet? |
|---|---|---|
| The merge and its table | `scripts/preview` | No |
| Keep it current as branches land | `scripts/preview --watch` | No, never |
| See what a restart onto it would do | `scripts/preview --restart --dry-run` | No |
| Run it in his Fleet and Bridge | `scripts/preview --restart` | Yes |
| The same with a Drone working | `scripts/preview --restart --adopt` | Yes, and the Drone is adopted |

**`--restart` and `scripts/restart` are never on an allow list.** They stop the
owner's real Fleet and reopen Bridge in front of him, so Claude Code's own
permission prompt is his confirmation, the same reason an agent does not run
`scripts/dev` (`armada-local`). Dry-run first and put what it printed in front
of him. Say what it will do, not "restarting Fleet": whether a Drone is working
now, and whether Bridge reopens (only if `apps/` or `packages/` changed since
its last build).

**A plain `scripts/restart` builds the latest `main`.** It fast-forwards to
`origin/main` first and refuses if it cannot; on 17 Sep 2026 the checkout was
four merges behind, so the restart rebuilt old code. **While the preview runs it
becomes `scripts/preview --restart`**, so the owner stays on the preview.
`scripts/restart --main` is the way back to `main` alone, only when he asks.
`--from <worktree>` is the preview's own build; it refuses a tree whose
migrations are behind the database.

**A preview can hold a branch that never lands**, and runs no Checks. Say which
branches are in it, and which were skipped or conflicted, before he runs it.

**`--adopt` only with his say-so.** Without it the restart refuses while a Drone
is working. With it, Fleet adopts each working Drone at boot, and the script
prints the Jobs and these costs once:

- its pipes die, so it cannot be redirected, poked or handed a verdict
- its recorded spend is an undercount
- its Job shows as `unheard`
- a Job's servers stop when Fleet stops
- a Check running mid-gate most likely dies and the gate re-runs from scratch
- a Drone that cannot be adopted is ended

Read the dry run's Job list to him in the question, not the machinery. A roster
that does not answer refuses even with `--adopt`. `--watch` refuses `--adopt`:
a watch never restarts Fleet. A Job whose Drone is gone resumes by itself:
Fleet restarts its step at boot, once.

**What refuses a restart.** A Job at `running` refuses it, naming the Job; an
escalated Job's idle Drone, a queued Job, a human gate and a piloted Job do not.
A restart already in progress refuses a second at once, naming its pid: wait.
`fleet.json names a live pid but the API on port 40000 did not answer` is Fleet
busy, not dead: poll `/health` for a few minutes and run it again, never kill
it (2 Oct 2026, #1424). A restart that died holding the lock names the pid and
the lock file; tell the owner, never remove it yourself.

**What runs after `--restart` is the preview's.** Bridge is launched from
`.armada/preview/apps/desktop` with its own `electron-vite`, and the run fails
if launchd loaded any other tree or Fleet reports a protocol ID other than the
one the preview's wire files hash to
(`node apps/desktop/codegen/protocol-id.mjs`). Read the `Bridge runs ...` line.

**A `libsqlite3-sys` compile error in the preview is a stale build script, not
the code.** Run `cargo clean -p libsqlite3-sys` in `.armada/preview` and run the
restart again. The owner's Fleet stays up, because the build fails before Fleet
is stopped. Confirmed twice on 6 and 7 Oct 2026. A slot freshly cloned from
the seed hits the same error in `armada check test`; clean it there too (8 Oct).

**The preview merges slot branches only.** A branch in a plain worktree under
`.claude/worktrees/` is not in it, and nothing says so. With every slot held,
`git worktree remove` it, `armada worktree add`, then
`armada worktree lease --existing <branch>`. Confirmed 8 Oct 2026: one dry run
and a read of `scripts/preview` to find why a branch was missing.

**A branch the restart cannot take is dropped by the refresh itself.**
When `--restart` fails, the branches behind it are left out and it goes again
until it builds: a branch with a breaking migration `origin/main` lacks
(`skipped (breaking migration <name> is not on main)`), and one that changed a
file the compiler errors name (`skipped (does not build: <files>)`). Fleet tells
each left-out branch's Job why. The work is not lost, it is not in this preview.
With no merged branch behind a failure it fails as before. A failure Bridge
shows starts with its cause: the compiler's first error and its file, or the
migration and its branch.

**A branch that conflicts with another in-flight branch is skipped.** To preview
both, merge the other into a preview-only copy of yours, and open the pull
request from the clean commit instead. Confirmed 8 Oct 2026 (#2025).

**The owner can run this from Bridge.** The Fleet panel's build section has a
Preview / Main Select and one button that follows it: `Refresh preview` is
`scripts/preview --restart`, `Update to main` is `scripts/restart --main`.
Bridge asks before `--adopt`, listing the working Jobs. It runs
`scripts/restart-build`, which retries a stale `libsqlite3-sys` once itself.
`docs/practices/running-locally.md`, *Restarting from Bridge*.

**To ask him to restart, put the exact command in `waiting_for` as a `run`
item**: `scripts/preview --restart`, with `--adopt` only once he said so. ▶ on
it, or on a shell block of the command, presses that button. Never ask him to run it any other way: a shell run from Bridge is
Fleet's child, dies with the Fleet it stops, and leaves Bridge on the old
protocol. Confirmed 13 Oct 2026.

**Starting a Fleet or Bridge of your own** is not this: that is
`.claude/skills/armada-local/SKILL.md` and `.claude/skills/dev-fleet/SKILL.md`.

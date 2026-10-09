---
name: preview-app
description: Merge every in-flight branch into a preview and, when the owner wants to look at it, move his Fleet and Bridge onto it — including past a working Drone with --adopt. Load before running `scripts/preview`, and before `scripts/preview --restart`.
---

# Updating the preview, and moving the owner onto it

`docs/practices/running-locally.md`, *A preview of unlanded work*, is what the
script does. This is when to reach for which form, and what to tell the owner.

| Wanted | Run | Moves the owner's Fleet? |
|---|---|---|
| The merge and its table | `scripts/preview` | No |
| Keep it current as branches land | `scripts/preview --watch` | No, never |
| See what a restart onto it would do | `scripts/preview --restart --dry-run` | No |
| Run it in his Fleet and Bridge | `scripts/preview --restart` | Yes |
| The same with a Drone working | `scripts/preview --restart --adopt` | Yes, and the Drone is adopted |

**`--restart` is never on an allow list**, for the reason in
`.claude/skills/restart-app/SKILL.md`: Claude Code's own permission prompt is
the owner's confirmation. Dry-run first and put what it printed in front of him.

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
a watch never restarts Fleet.

**What runs after `--restart` is the preview's.** Bridge is launched from
`.armada/preview/apps/desktop` with its own `electron-vite`, and the run fails
if launchd loaded any other tree or Fleet reports a protocol ID other than the
one the preview's wire files hash to. Read the `Bridge runs ...` line in the output.

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

**A branch that conflicts with another in-flight branch is skipped.** To preview
both, merge the other into a preview-only copy of yours, and open the pull
request from the clean commit instead. Confirmed 8 Oct 2026 (#2025).

**The owner can run this from Bridge.** The Fleet panel's build section has a
Preview / Main Select and one button that follows it: `Refresh preview` is
`scripts/preview --restart`, `Update to main` is `scripts/restart --main`.
Bridge asks before `--adopt`, listing the working Jobs. It runs
`scripts/restart-build`, which retries a stale `libsqlite3-sys` once itself.
`docs/practices/running-locally.md`, *Restarting from Bridge*.

**Starting a Fleet or Bridge of your own** is not this: that is
`.claude/skills/armada-local/SKILL.md` and `.claude/skills/dev-fleet/SKILL.md`.

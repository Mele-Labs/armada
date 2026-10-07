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
if launchd loaded any other tree or Fleet reports a protocol other than the
preview's `protocol-version.toml`. Read the `Bridge runs ...` line in the output.

**Starting a Fleet or Bridge of your own** is not this: that is
`.claude/skills/armada-local/SKILL.md` and `.claude/skills/dev-fleet/SKILL.md`.

# Spike 23 — Does a mod installed in the operator's config load in a Drone?

> **7 Oct 2026, reversed by the owner.** Drones lost his user settings (permissions, env, model, plugins, hooks) under `--setting-sources project,local`, and he chose "everything, mod included". A Drone, a Judge call and a scout no longer carry the flag. Fleet sets `ARMADA_DRONE=1` in their environment and the `armada` mod reports nothing when it is set, so none of them reaches the Sessions list. The measurements below still hold; the decision they led to does not.

**Yes, and so did every plugin, skill and subagent the operator had.** A Drone
is started with the operator's real `HOME` and no flag that leaves their user
settings unread, so a Claude Code mod installed with `claude plugin install` ran
its `session.start` inside a Drone launch. `--setting-sources project,local`
stopped it, with authentication untouched. A Drone, a Judge call and a scout now
carry it; Helm does not.

Measured against Claude Code 2.1.292 on 6 Oct 2026. The question came from
Sessions step 1: a mod that reports to Fleet must not report a Drone's own
session back to Fleet as though a person had started it.

## What a launch holds today

| Launch | Where its argument list is built | `HOME` | Settings read |
|---|---|---|---|
| Drone | `crates/adapters/src/harness.rs`, `render` | the operator's, from `HostPaths` in `crates/fleet/src/drone.rs` | user, project, local |
| Judge call | `crates/adapters/src/judge.rs`, `asking` | the same environment | user, project, local |
| Scout | `crates/adapters/src/scouting.rs`, `render_scout` | the same environment | user, project, local |
| Helm | `crates/adapters/src/conversing.rs` | the operator's | user, project, local, and meant to be |

None carries `--bare` or `--safe-mode`. `--bare` also stops the CLI reading the
keychain, so a Drone could not authenticate; `--safe-mode` drops the repository's
own `CLAUDE.md` and skills, which a Drone is meant to read.

## Measurement

A throwaway mod whose `session.start` writes a file
(`023-probe-mod-register.ts`), installed with `claude plugin marketplace add` and
`claude plugin install` under a scratch `HOME` so nothing of the operator's was
touched, then a Drone's launch: environment cleared down to `PATH`, `HOME`,
`LANG`, `TERM`, `USER`, and the Drone's stream-json flags (`023-run.sh`). The
scratch `HOME` has no login, and `session.start` fires before the first request,
so the marker shows whether the mod loaded.

| Launch flags added to the Drone's | Mod's marker written |
|---|---|
| none | **yes** |
| `--setting-sources project,local` | no |
| `--safe-mode` | no |
| `--bare` | no |

`--setting-sources` was then run against the operator's real `HOME` with a
one-turn Haiku call (`023-run-authenticated.sh`), because a flag that loses the
login is not a fix.

| | Plugins in the session's init message | Tools | Skills | Answer |
|---|---|---|---|---|
| Drone flags | eleven the operator installed, three built in | 30 | operator's and built in | `ok` |
| plus `--setting-sources project,local` | the three built in | 29 | built in only | `ok` |

The operator's `~/.claude/settings.json` holds `PreToolUse` and `PostToolUse`
hooks, which ran in every Drone until now. It sets no `env` and no
`apiKeyHelper`, so nothing a Drone needs from it is lost.

## What it costs

A Drone no longer reads a skill, subagent or plugin from the operator's
`~/.claude`. A repository's own `.claude/` in the worktree is still read, since
`project` and `local` stay. Nothing in the carried workflows was found
to name a user-level skill, but that was a search and not a proof: a Job that
fails on a missing one is the sign it was relied on unseen.

## The test that holds it

`crates/adapters/src/tests/harness.rs`, `judge.rs` and `scouting.rs` each assert
the flag on every rendering, and `conversing.rs` asserts Helm lacks it.

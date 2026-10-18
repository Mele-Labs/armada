# Protocol 23.44: a held red, and each merge's run on main

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Additive.
`hub.main.checking` (`MainChecking {commit, pull_request?}`, newest first) names the newer commits whose CI is still running while main is red, and `hub.main.red_commit` the commit the red was read at.
Main's state is the newest commit whose run has finished, so a green on an intermediate commit clears a red; `checking` on a `green` is only checks running.
On a `red`, non-empty `checking` is a held red: `fix_main` refuses with `fleet.main_checks_running` and Fleet's own pickup waits.
`hub.merged[].main_run` (`MainRun {state, failed?}`, `state` passed, running or failed) is the CI run on that merge commit on main itself.
A failed job's log opens on `observe_land_check` under the branch `main@<commit>`, a new spelling of an existing free-text field.

**Older peers:** a Bridge before 23.44 ignores the three fields and draws the red band with its buttons, which Fleet then refuses while held.
`state` stays `red` rather than gaining a variant, because Bridge branches on it and a new variant would be a major.
A Bridge at 23.44 against an older Fleet is refused as any minor ahead is.

**Store columns, additive** (`main_ci.red_commit`, `main_ci.decided_commit`, `main_ci.newer_running`). Forge cost is in `docs/concepts/fleet.md`, *What Fleet knows about main's CI*.

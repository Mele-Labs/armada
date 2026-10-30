---
name: update-armada
description: Move the owner's Fleet, Bridge and Claude Code mod onto latest main in one go — preview restart with adopt, which also updates the plugin — so all he does after is /reload-plugins in a terminal job. Load when the owner types /update-armada, or asks to update, restart or refresh Armada and the mod.
---

# /update-armada

**The owner typing `/update-armada` is his say-so for `--adopt`.** Do not ask
first. The restart's own permission prompt is his confirmation, so it stays off
every allow list (`preview-app`).

```
scripts/preview --restart --dry-run   # read it; changes nothing
scripts/preview --restart --adopt     # Fleet + Bridge + mod copy + installed plugin
```

Run each from the main checkout, in the foreground, one after the other. Stop
at the first that fails and report it; `preview-app` has the known failures
(`cargo clean -p libsqlite3-sys` in `.armada/preview` for a stale build script).

**The restart moves every job onto the new mod, so there is no plugin step
here.** It copies the mod to `~/Library/Application Support/Armada/mod`, then
runs `claude plugin marketplace update armada-local` and
`claude plugin update armada@armada-local`. A warning it prints for either is
the one thing to report. Fleet lets each hosted job's process go once its turn
ends, and its next message starts it on the copy. A terminal job loads the plugin when `/reload-plugins` runs
in it. Claude Code caches the plugin by version, so a mod change bumps
`MOD_VERSION` and `plugin.json` together. A version that did not change is not
a failure: say so in the report.

## Report

| | |
|---|---|
| Protocol | the ID the restart printed |
| Adopted | the Jobs it named, or none |
| Mod | old → new version, from `claude plugin list` before and after |

End with the one thing left to him, on its own line: `/reload-plugins` in each
open cmux job. A hosted job needs nothing.

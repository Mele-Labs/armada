---
name: update-armada
description: Move the owner's Fleet, Bridge and Claude Code mod onto latest main in one go — preview restart with adopt, then the plugin update — so all he does after is /reload-plugins. Load when the owner types /update-armada, or asks to update, restart or refresh Armada and the mod.
---

# /update-armada

**The owner typing `/update-armada` is his say-so for `--adopt`.** Do not ask
first. The restart's own permission prompt is his confirmation, so it stays off
every allow list (`restart-app`).

```
scripts/preview --restart --dry-run   # read it; changes nothing
scripts/preview --restart --adopt     # Fleet + Bridge + mod copy
claude plugin marketplace update armada-local
claude plugin update armada@armada-local
```

Run each from the main checkout, in the foreground, one after the other. Stop
at the first that fails and report it; `preview-app` has the known failures
(`cargo clean -p libsqlite3-sys` in `.armada/preview` for a stale build script).

**The plugin update is what moves a session onto a new mod.** The restart copies
the mod to `~/Library/Application Support/Armada/mod`; Claude Code caches it by
version, so a mod change bumps `MOD_VERSION` and `plugin.json` together. A
version that did not change is not a failure: say so in the report.

## Report

| | |
|---|---|
| Protocol | the version the restart printed |
| Adopted | the Jobs it named, or none |
| Mod | old → new version, from `claude plugin list` before and after |

End with the one thing left to him, on its own line: `/reload-plugins` in each
open cmux session.

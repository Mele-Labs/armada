# Installing the Claude Code mod

**Part of:** [Job](../session.md).

```
claude plugin marketplace add "$HOME/Library/Application Support/Armada/mod"
claude plugin install armada@armada-local
```

**Fleet keeps its own copy of the mod** in that folder. `scripts/restart` copies
`plugins/` into it from the tree it builds, so the mod is as new as the Fleet
and a pull on the checkout changes nothing a job loads. After a restart,
`/reload-plugins` in an open job picks the new copy up. The copy is made
beside the folder and renamed over it, so a job never reads half of one.
A tree with no `plugins/` leaves the old copy and warns. Whatever compares a
job's reported mod version with the installed one reads
`Armada/mod/armada/.claude-plugin/plugin.json`.

**The mod ships an output style, `Armada`** (`plugins/armada/output-styles/armada.md`), found from the folder with no entry in `plugin.json`, and picked with `/output-style` as `armada:Armada`.
It keeps the coding instructions (`keep-coding-instructions: true`), states the job rules and sets plain, concise replies.
A hosted job needs it no more than it needs the rules in its prompt; it is for a terminal job of a person who has no style of their own.

**The mod ships a fleet of subagents** (`plugins/armada/agents/`), named `armada:` and the type, so a job in any repository can hand a decided change to `armada:mechanic` on `sonnet` or a test run to `armada:test-runner` on `haiku`.
The job rules say to prefer the repository's own types first.
Each preloads the mod's skills its work needs (`plugins/armada/skills/`): `armada:reading-a-brief`, `armada:verify-before-done` and `armada:root-cause`, so a rule like "reproduce first" is written once.
`docs/practices/subagents.md` has the fleet, the model each runs on and the skills each preloads.

It loads in a Judge call and a scout too, since they read the person's user settings.
Fleet starts each with `ARMADA_DRONE=1`; the mod reads it and reports nothing, so only the person's own jobs reach the ledger ([spike 23](../../spikes/023-does-a-user-installed-mod-load-in-a-drone.md)).

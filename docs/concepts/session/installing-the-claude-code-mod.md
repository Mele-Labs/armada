# Installing the Claude Code mod

**Part of:** [Job](../session.md).

```
claude plugin marketplace add "$HOME/Library/Application Support/Armada/mod"
claude plugin install armada@armada-local
```

**Fleet keeps its own copy of the mod** in that folder.
`scripts/restart` copies `plugins/` into it from the tree it builds, so the mod is as new as the Fleet and a pull on the checkout changes nothing a job loads.
The copy is made beside the folder and renamed over it, so a job never reads half of one.
A tree with no `plugins/` leaves the old copy and warns.
Whatever compares a job's reported mod version with the installed one reads `Armada/mod/armada/.claude-plugin/plugin.json`.

**The restart then updates the installed plugin**, so a terminal job has the new mod ready to load: `claude plugin marketplace update armada-local`, then `claude plugin update armada@armada-local`.
The CLI caches the plugin by version and moves the cache only on those two commands, so a change to the mod bumps `MOD_VERSION` and `version` in `plugin.json` together.
A `claude` that is not on `PATH`, or an update that fails, is one warning line and the restart goes on; `--dry-run` prints the two commands.
A terminal job picks the new mod up when the person runs `/reload-plugins` in it, and nothing Fleet does reaches into that process ([A mod that is out of date](a-mod-that-is-out-of-date.md)).

**Fleet runs these two commands itself on request.** `install_agent_mod` (body `{"requested": true}`; `false` is a 422) fills the copy with `scripts/sync-mod` from a served repository that holds one when it is missing, runs the two commands with each stopped after two minutes, and answers with the CLI's own words: `installed`, or `failed` with the step that stopped.
Nothing else starts it.
Onboarding's *Import* uses it, beside `list_import_folders` and `get_import_runs`, which read a person's past sessions (`crates/ipc/operations/`).

**A hosted Job loads Fleet's copy whether or not the person installed it.**
Fleet starts every process of a Job it hosts with `--plugin-dir` on `Armada/mod/armada`, so a Mac where nobody ran the two commands still gets the mod's task list, job id note, subagents and skills in its Jobs.
Beside an installed copy the CLI loads the mod once, this copy winning, so a hosted Job runs the mod as new as the Fleet (measured on 2.1.292, 2.1.308, 2.1.310 and 2.1.312, 29 Oct 2026).
Where the copy is empty, Fleet fills it with `scripts/sync-mod` from a served repository that holds one before the launch, and a launch it cannot fill starts without the mod.
A Job that still came up without it is marked and mended once ([A Job that came up without its tools](a-job-that-came-up-without-its-tools.md)).
A terminal job still loads only an installed copy.

**A hosted Job a restart of Fleet left running moves onto the new copy by itself.**
A process Fleet adopted across the restart keeps the mod it loaded when it started and reports it once, so no report says it fell behind.
The sweep that already walks every hosted process between turns, every thirty seconds, compares the version each Job reported with the `version` in the copy.
A Job whose version is older has its process restarted onto the copy, by the restart *Restart the agent* uses ([A Job that came up without its tools](a-job-that-came-up-without-its-tools.md)), once it has no turn running, none on its way, no question waiting, and no subagent or background task working.
A Job with a turn running waits for it to end, so no turn is cut, and the sweep asks again.
A Job that is paused or waiting to be moved into a slot is left as it is, since the move and the resume start a process that loads the copy.
A Job that reported no version is left too: its process may not have said so yet, and one that came up without the mod is the mend's.
The restart is once for each version of the copy, so a replacement that still reports an older one does not loop, and a newer copy is a new reason.
The replacement resumes the conversation and pays for it again, and is told `Fleet restarted your process onto armada mod 0.3.53 from 0.3.52. Nothing is asked of you.`
That word is news, kept from any mute or rule, because a restart with no unfinished line has no turn to start in without it.
A process that goes quiet is let go after the quiet spell and resumes on the copy at the next message, so the move reaches only a process that stays up.

**The mod ships an output style, `Armada`** (`plugins/armada/output-styles/armada.md`), found from the folder with no entry in `plugin.json`, and picked with `/output-style` as `armada:Armada`.
It keeps the coding instructions (`keep-coding-instructions: true`), states the job rules and sets plain, concise replies.
A hosted job needs it no more than it needs the rules in its prompt; it is for a terminal job of a person who has no style of their own.

**The mod ships a fleet of subagents** (`plugins/armada/agents/`), named `armada:` and the type, so a job in any repository can hand a decided change to `armada:mechanic` on `sonnet` or a test run to `armada:test-runner` on `haiku`.
The job rules say to prefer the repository's own types first.
Each preloads the mod's skills its work needs (`plugins/armada/skills/`): `armada:reading-a-brief`, `armada:verify-before-done` and `armada:root-cause`, so a rule like "reproduce first" is written once.
The mod ships `armada:armada-mods` too, for a Mod Job in the mods folder, which reads no repository's skills; this repository's `.claude/skills/armada-mods` is a link to it.
`docs/practices/subagents.md` has the fleet, the model each runs on and the skills each preloads.

It loads in a Judge call and a scout too, since they read the person's user settings.
Fleet starts each with `ARMADA_DRONE=1`; the mod reads it and reports nothing, so only the person's own jobs reach the ledger ([spike 23](../../spikes/023-does-a-user-installed-mod-load-in-a-drone.md)).

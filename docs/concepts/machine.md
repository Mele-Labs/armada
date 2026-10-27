# Machine

**What it is:** How this installation behaves — resources, timing, interface and notification routing. One tier, no project-level counterpart, no merge.

---

**Kind:** Entity.

Defines Machine — how this installation of Armada behaves. Split from [Kit](kit.md).

## What it is

The settings that describe **this installation** — its resources, its timing, and how loudly it speaks to you. Machine is the half of the old Guild concept that no project has an opinion about.

**A Machine setting has one value.** There is no project-level counterpart, no merge strategy and no resolution order. A [Manifest](manifest.md) never participates.

## The test

**Does a project-level version of this setting make sense?** If yes, it belongs to [Kit](kit.md) — a project adding a Skill is ordinary. If no, it is Machine.

There is no project-level version of *how loudly Armada notifies you*; the notion does not parse.

**Your Kit travels. The Machine does not.** A [Trigger](trigger.md) set for this machine is Machine's, kept in `~/.armada/machine/triggers/`.

**Port block granule is the one knowing exception.** A project-level version does parse — a monorepo of compose stacks wants a larger granule than a repo of libraries — so the test above says Kit.

It stays Machine because the preference is unfelt: demand already drives a claim's width.

## settings.json

**Every Machine setting is one key of one file**, `settings.json` beside Fleet's store.
It is a flat JSON object of dotted keys, `"limits.checksAtOnce": 3`, and a key the file does not hold is what ships.
The owner decided it on 10 Oct 2026, `.claude/decisions/2026-10-10-settings-json-replaces-the-store.md`.

| Question | Answer |
| --- | --- |
| Which keys exist | `config::settings`' table, which is also what Bridge's Settings page is drawn from |
| Who changes it | Bridge through `save_settings`, or a person by hand. Fleet watches the file |
| When a change is in force | A `live` key from the next time Fleet reads it; an `at_restart` key at the next start, and Bridge says so |
| A file Fleet will not take | An unknown key, a wrong type or a value out of range refuses it whole. The last good settings stay in force, and Bridge shows the refusal |
| What wins over it | An environment variable such as `ARMADA_MODEL`; and a repository's own value, as before |

**A person's Bridge preferences are keys too**: theme, spacing (`spacing.density`, Compact, Default or Roomy), layout, key bindings, time format, the fields a Job's row draws (`bridge.rowView`) and whether *Where things are* opens.
A mod's on-off switch is not, and stays in Fleet's store.
The binding rules are `../contracts/configuration.md`, *The Machine tier is one file*.

## Two functional groups

### Resources

- Memory headroom threshold
- Disk headroom floor
- Fleet health-check and resource-poll interval
- SQLite WAL checkpoint interval
- Concurrency cap
- Quota % floor
- Worktree root path
- Log retention and pruning
- Port range base
- Port range ceiling
- Port block granule

### Interface & Notifications

- Landing Manifest
- Notification routing: `features.notifications`, on or off for every job that needs you
- Interface sounds: `features.uiSounds`
- Pull request links: `features.deepLinks`, whether `armada://pull/<owner>/<name>/<n>` opens that review in Bridge
- Autopilot panel: `features.autopilotPanel`, whether the Cockpit draws it; off ships
- Voice/tone
- Helm budget soft-warning threshold
- Helm action authority

**Helm action authority is Machine, not Kit.** Why: what Helm may do is a property of the installation, not of a repo.

## Notification routing

**Routing is a Machine setting with one value and no merge.** What bounds it is a **product rule on `../contracts/design-system.md`**, not a config tier: the loudness order, and the rule that an approval may never be promoted to push.

That rule holds because escalations and approvals mean different things.
An escalation means work has stopped and nothing progresses until a person looks.
An approval means work is waiting to start and will keep.

## Voice/tone

**Voice governs runtime-generated prose only** — Judge summaries, Helm replies, Job summaries. Static UI chrome is not configurable.

Voice may adjust length and formality.
It may not override the voice principles, the lexicon or the status grammar: "terse" and "explanatory" are legal values, "playful" is not.
Bounded by the `../contracts/design-system.md` contract.

## Quota — measured, and not gated on

**Armada does not read quota.** The gate below was designed and is not built,
and the reason is a measurement rather than an omission: a headless agent's
stream reports *whether* it is allowed and *when* the window resets, and carries
no quantity at all. The percentage is real and live on every request, and the
channel that carries it does not run headless.

| Machine type | What was intended | What is true |
| --- | --- | --- |
| Personal machine, subscription plan | Gate on % of the 5-hour or weekly window remaining | **Not built.** No quantity reaches Armada |

**What that costs, said plainly.** On a subscription the five-hour window is
what will actually stop a fleet — dollars stop nothing, because nothing is
billed per token. So the fleet meets its real limit as a hard failure with no
approach warning: Jobs running normally, then work dying mid-flight. That shape
is expected rather than designed against.

**What reaches Armada now is no gate.** The `armada` mod in a
job reports the plan's windows (`five_hour`, `seven_day`) with the percent used
and when each resets, and Fleet serves the newest reading as `get_plan_usage` for
Bridge to draw.
An account off a subscription, such as an enterprise one, reports no windows, and Bridge draws no rings for it rather than waiting for a reading.
And a run the API already refused is held rather than failed: a
job that hit the limit is woken with `Continue` at the reset (`session.md`,
*A job that hit the plan's limit*). No floor is held against the figure.

### The Job model steps down

**The weekly window nearing its end offers the lower model as the Job default, and the owner decides.** The offer is made when the `seven_day` window is at or over `plan.downshiftAt` (80 percent), once for each window.
Accepting writes `models.session` as `plan.downshiftTo`, so Jobs started afterwards take it.
The five-hour window never offers one.
Nothing is switched without the press.

| Phase | How it is reached | What the owner can do |
| --- | --- | --- |
| none | Nothing offered, or this window was settled | |
| `offered` | The weekly reading crosses the line while `models.session` is not already the lower model | Accept, or set the Job model by hand, which settles the window |
| `shifted` | Accepted: `models.session` is the lower model until the window's `resets_at` | Restore |
| `reset` | A reading carries a different `resets_at` while shifted | Restore, or keep the lower model |

**The state is Fleet's, and it is kept.** The phase and the window an offer was last made for are one row in the store, so a restart does not forget a lowered default and an empty reading never clears one.
The last plan reading is kept in a row of its own and served again after a restart, without the windows already past their reset.
A restored reading is not a report: it publishes no `plan_usage.changed` and is not judged for a step down.
Accepting records what settings.json held for `models.session`; a value that was not saved is removed again on restore, so the shipped one is back rather than a copy of it.
A hand edit of `models.session` away from what the phase expects settles it.
`get_model_downshift` and `model_downshift.changed` serve it; `docs/practices/protocol.md`, *The Job model steps down*.

**`models.review` is the model an agent run about a pull request runs on.** It ships as the Job model and is its own setting after that, so the step down of Jobs leaves reviews where they were.
It is live: a run started after a change takes it.
It has no environment variable.
`docs/concepts/pull-review.md`, *Agent runs*.

**What follows `models.session`.** It is live.
A Job process started after the change takes it, and a process already running keeps its own until it ends and the next message starts another.
A model chosen for one Job wins, and a terminal Job reports its own.
`models.default` is never moved by the step down; `ARMADA_MODEL` names that one, and `models.session` has no variable.

## What a Job is told about this Mac

**Every process of a hosted Job or Helm Session is told what this Mac has.** It is the last section of the Logbook note (`logbook.md`, *Recall*), after the habits and the repository's entries, read from Fleet at each start.

| Line | Where it comes from | Dropped when |
|---|---|---|
| A command is not installed here | A Job's shell printed `command not found` in a result | It resolves on the `PATH` a Job is started with |
| The stand-in for it | A fixed table: `timeout` and `gtimeout` name the Bash tool's `timeout` parameter | Its command is dropped |
| The Playwright tool's browser | The newest Playwright headless shell on this Mac, or the command that installs one | Never; it is looked up again |

**Missing commands are kept for the machine, not the repository.** They are read off the stream, never from an agent's word.
A result that names a missing command counts as failed, because a pipe hides the exit of what is before it.

**The machine note is a section of the [Logbook](logbook.md).** Its detector is unchanged.
Its facts are Logbook entries in the `machine` section, and this note is the last section of the Logbook note, cut with it near 2k tokens. What the cut leaves out is found with `recall`.

### The browser

**A hosted Job's process is started with a headless Playwright shell, so the Playwright tool needs no Chrome.** Chrome is not installed here, and headless keeps windows off the owner's screen.
Bridge's Electron cannot stand in: it refuses `Target.createBrowserContext`.

| Variable | Value |
|---|---|
| `PLAYWRIGHT_MCP_BROWSER` | `chromium` |
| `PLAYWRIGHT_MCP_HEADLESS` | `true` |
| `PLAYWRIGHT_MCP_EXECUTABLE_PATH` | The shell found |

The shell is the highest-numbered `chromium_headless_shell-<N>` under `$PLAYWRIGHT_BROWSERS_PATH`, else `~/Library/Caches/ms-playwright`.
It is looked up at each launch, so a Job started after an install gets it.
Where none is found, Fleet installs one in the background once per start, and prints the install command if that fails. The variables are set only where a shell was found.
The code is `crates/adapters/src/browser_shell.rs` and `crates/fleet/src/machine_note.rs`; the install is called from `crates/armada/src/serve.rs`.

## Init

The installation half of first-run setup: resources, notification routing, interface defaults. The tool half belongs to [Kit](kit.md).
First-run onboarding starts a Job on a folder and does not walk either half first.

What this combined surface is called, and whether it is its own journey, is tracked in [Kit](kit.md).

## Configuration

All rows in the Configuration Settings registry tagged `Machine`. A Machine row carries no `Merge strategy` and no `Peer polarity` — both read `n/a — single layer` by construction.

## Open questions

- **[machine-voice-owning-tier]** Does Voice belong to Machine or to Kit?
  Voice is a Machine setting by the group mapping above, but `../contracts/agent-prompt.md` places it in prompt layer 2 beside Skills, the Agent file and Sub agent definitions — all of which are Kit.
  The two readings disagree about which concept owns it.

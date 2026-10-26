# Watch

**What it is:** A command Fleet runs on a cadence for one Job, whose last line says where a deploy or a pipeline stands.
Fleet tells the Job when it fails, ends, gives up or breaks, and the Job's agent weighs a failure.

---

**Kind:** Concept.

**Built:** the model, recipes, the store, the runner, the notices and the wire.

The design is `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`, and spike 36 measured it (`docs/spikes/036-is-one-line-enough-to-watch-a-deploy.md`).
The wire is `docs/practices/protocol.md`, *Watches on the wire*.

## What a Watch is

| Part | Holds |
|---|---|
| Name | What the person reads, such as `QA deploy` |
| Command | Whole arguments, never read by a shell |
| Every | How often Fleet runs it |
| Gives up after | How long before Fleet stops waiting for it to finish |
| Follows | The pull request it is about, where it is about one |
| Set aside | Parts the agent weighed as not its change's, each with a sentence |
| Readings | The newest 50, each a line read or a run that broke |

A watch belongs to one Job and ends with it.
Armada knows no provider: a script the repository or the person owns turns a provider's words into the line.

## What the command prints

> **Rule.** The command's last line on standard output is one JSON object, and Fleet reads nothing else.
> Why: the line is the whole contract with a provider, so Armada never learns a provider's words.

| Field | Values | Notes |
|---|---|---|
| `state` | `waiting`, `running`, `passed`, `failed` | Required |
| `finished` | true or false | Required. True once what the watch follows changes no more |
| `says` | one sentence | Required. Bridge draws it and a notice quotes it |
| `link` | an address | Optional |
| `parts` | a list of name and state | Optional. A run whose overall state misleads is read part by part |

> **Rule.** `finished` is a field of its own and is never read from `state`.
> Why: a run reads `failed` the same mid-run and at its end, so a watch with a failure set aside could not tell it was over.

With `parts`, the watch's state comes from the parts not set aside: any `failed` is `failed`, any `waiting` or `running` is `running`, all `passed` is `passed`.
Without `parts`, the line's own `state` stands.
A watch ends `passed` only on a reading that is `passed` once counted, and `finished`.

A run that exits non-zero, runs out of time or prints no such line is a **broken** reading, and its last output is kept.

Two rules every script keeps, because the line has no word for either:

- A step the provider skipped, which leaves old code running, is `waiting` until the run is finished and `failed` then.
- A step retried reports its latest attempt.

## Where a watch stands

| State | Means | Tells the Job |
|---|---|---|
| `waiting` | Nothing counted has started | No |
| `running` | Something counted is going | No |
| `passed` | Counted parts passed and the line is finished | Yes |
| `failed` | A counted part failed | Yes, once a change |
| `gave_up` | Past its time with nothing finished | Yes |
| `broken` | The command broke three readings in a row | Yes |
| `stopped` | The person or the agent stopped it, or its Job ended | No |

What Job is told, and the words, are `docs/concepts/session/what-fleet-tells-a-job.md`.

## Recipes

A recipe names a command, how often to run it and when to give up.
A person writes it as a file; the agent starts a watch by naming one and filling its values.

| Key | Holds |
|---|---|
| `name` | What the agent names it by |
| `command` | A list of arguments, where an item written `{sha}` is a value to fill |
| `every` | A span such as `60s`, `2m` or `2h` |
| `gives_up_after` | A span. Absent takes the machine's setting |
| `environment` | A Requests environment whose variables the command is given |

> **Rule.** The most specific level wins, and its copy replaces the others whole.
> Why: a field merge makes a recipe nobody wrote.

| Level | Where | Notes |
|---|---|---|
| Armada | Compiled in | Ships none |
| Repository | `.armada/watches/` | Read from `main` only |
| This machine | `~/.armada/machine/watches/` | Does not travel |

> **Rule.** A value fills one whole argument and is never read by a shell.
> Why: an item such as `--sha={sha}` would need a shell to split it, and a watch never runs one.

> **Rule.** An agent may start a watch on a command of its own, with no recipe and no person's step.
> Why: the agent can already run any command it likes, and a recipe is for a watch a repository starts often.

The recipe's `environment` is a [Requests](requests.md) environment, so its secrets come from the Keychain and are kept out of every reading and the side panel as `{{name}}`.
The cadence floor and the give-up cap are checked when a watch starts and not when the file loads, since a person moves them while Fleet runs.

## How Fleet runs one

> **Rule.** Watches run on a timer of their own, every 5 seconds, and take the ones that are due.
> Why: the pull request reading runs at most once a minute and waits on the forge, so it would halve a 30 second watch.

| Behaviour | Holds |
|---|---|
| A run | A Check's: no shell, a process group of its own, a time limit, low priority |
| Where | The Job's worktree, so an agent's own script runs as written |
| Environment | Fleet's own, with `ARMADA_SPAWNED_BY`, so a crash's leftovers are ended |
| Slowing | Every 2 minutes after 30 unchanged, every 5 after an hour |
| Kept | The newest 50 readings in the store, so a restart carries on |
| Published | A changed reading at once, an unchanged one at most every 5 minutes |

A watch whose worktree is gone gives broken readings, so it breaks after three and the agent is told.
A machine's recipe runs its command by absolute path.

| Bound | Setting |
|---|---|
| Watches running a command at once | `limits.watchesAtOnce` |
| Live watches one Job holds | `limits.watchesPerJob` |
| One run's time | `timeouts.watchRunSeconds` |
| Fastest a recipe may ask for | `timeouts.watchEverySeconds` |
| Give-up where a recipe names none | `timeouts.watchGiveUpMinutes` |

The values are `crates/config/settings.toml`.
A recipe may ask to give up after up to 24 hours, and a watch stops after three broken readings in a row.

## The agent weighs it

> **Rule.** A failed reading is weighed by the agent, ours or not ours, each with a sentence saying why.
> Why: whether a red reading matters to the change needs the diff, and no rule in a recipe has it.

| Verdict | What it does to the watch |
|---|---|
| `ours` | Ends it `failed`. The agent fixes the change, and a redeploy is a new watch |
| `not_ours`, with parts | Sets those parts aside. The watch carries on, and a change in them wakes nobody |

A weighing tells nobody, since the agent gave it.
A `not_ours` names a part some reading of the watch carried, or it is refused.
A watch may start with parts already set aside, so a redeploy keeps what the agent learned on the last one.
A set-aside lasts one watch, so a step that fails on every build costs one wake on every push.

## Telling the Job

> **Rule.** Each notice is told once: the store keeps the watch, its state and the parts that failed, and a restart repeats nothing.
> Why: a reading taken again is not news.

**An agent does not watch a pull request's own checks.**
Fleet's pull request notice already tells the Job when they fail, so a watch would wake the agent twice for one failure.
A watch is for what that reading cannot see: a deploy, an environment, a pipeline another branch's push starts.

**A watch follows what it was started on, and nothing after it ends.**
Noticing that someone replaced a deploy is a new watch's job, started by whoever needs it.

## A Job a watch holds open

> **Rule.** A Job with a live watch is not finished, and Bridge says that the watch is what holds it.
> Why: a Job is done once its deploy settles, and a person must tell a Job held by a watch from one still working.

A live watch is a wait, as an open pull request is.
When it ends, whichever way, its notice wakes the agent, which carries on and calls `all_done` once nothing is left.
A held Job is not a Needs you item, since what it waits on is Fleet's wait and not the person's.
Ending a Job ends its live watches `stopped` and tells nothing.

## In Bridge

| Where | What it draws |
|---|---|
| The Landing | A Watching block below the pull requests: a state glyph, the name, what it said and its age, with the agent's verdict under it |
| The side panel, on a press | Readings over time, each part, parts set aside struck through, a broken run's output, the link, and Stop |
| A Job's row and header | Held open by the watch's name, while only a watch keeps the Job open |

The glyphs are `docs/contracts/iconography.md`, *A watch and its states*.
The word is `docs/contracts/design-system/voice-copy.md`, *Lexicon*.

## Where it lives in code

| What | Where |
|---|---|
| The model, with the counting and the verdicts | `crates/core-model/src/watch.rs` |
| Reading a command's last line | `crates/ipc/src/watches.rs` |
| Recipes | `crates/config/src/watches.rs` |
| Start, weigh, stop and list | `crates/fleet/src/watches/serving.rs` |
| The timer and the runs | `crates/fleet/src/watches/running.rs` |
| The notices | `crates/fleet/src/watches/telling.rs` |
| What is kept | `crates/store/src/watches.rs` |
| Bridge's copy of the wire | `packages/protocol/src/watches.ts` |

## Not built

| Follow-up | Why it waits |
|---|---|
| The agent's session rule: start a watch, never poll | `crates/adapters/src/session_rules.md` says nothing of watches yet |
| A watch that sends a saved Request and reads a field | Left for after v1: it fits the same line |
| A watch on what an environment is running | A new watch for whoever needs it, so a replaced deploy is news |

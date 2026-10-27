# Watch

**What it is:** A command Fleet runs on a cadence for one Job, whose last line says where a deploy or a pipeline stands, or text Fleet waits for in the Job's terminal.
Fleet tells the Job when it fails, ends, gives up or breaks, and the Job's agent weighs a failure.

---

**Kind:** Concept.

**Built:** the model, recipes and the two Armada ships, the store, the runner, the login shell's environment, terminal watches, the notices and the wire.

The design is `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`, and spike 36 measured it (`docs/spikes/036-is-one-line-enough-to-watch-a-deploy.md`).
The wire is `docs/practices/protocol.md`, *Watches on the wire*.

## What a Watch is

| Part | Holds |
|---|---|
| Name | What the person reads, such as `QA deploy` |
| Command | Whole arguments, never read by a shell. A terminal watch has none |
| In terminal | A terminal watch's text, or regular expression, to wait for |
| Every | How often Fleet runs it |
| Gives up after | How long before Fleet stops waiting for it to finish |
| Follows | The pull request it is about, where it is about one |
| Set aside | Parts the agent weighed as not its change's, each with a sentence |
| Readings | The newest 50, each a line read or a run that broke |

A watch belongs to one Job and ends with it.
Outside its two shipped readers Armada knows no provider: a script the repository or the person owns turns a provider's words into the line.

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
| `parts[].link` | an address | Optional. Where to look at that part |
| `parts[].error` | text | Optional. The exact lines that part failed on |
| `parts[].tail` | text | Optional. The last lines of that part's own log |

> **Rule.** `finished` is a field of its own and is never read from `state`.
> Why: a run reads `failed` the same mid-run and at its end, so a watch with a failure set aside could not tell it was over.

With `parts`, the watch's state comes from the parts not set aside: any `failed` is `failed`, any `waiting` or `running` is `running`, all `passed` is `passed`.
Without `parts`, the line's own `state` stands.

> **Rule.** All counted parts `passed` while a part set aside reads `failed` is `not_ours`, never `passed`, and that holds when every part is set aside.
> Why: a deploy whose set-aside part failed did not ship, and a green mark for it is a false positive.

A set-aside part that later reads `passed`, as on a retry, no longer stands in the way of `passed`.
A watch ends `passed` only on a finished reading that is `passed` once counted with nothing set aside failing, and ends `not_ours` on a finished reading that is `not_ours`.
The readings keep the states their lines gave; only the watch's state changes.

A run that exits non-zero, runs out of time or prints no such line is a **broken** reading, and its last output is kept.
A reading that failed, overall or in a part, and a broken one keep the run's whole output as a log file under Fleet's state, linked from the reading; the file is deleted with the watch, or when its Job ends.

Two rules every script keeps, because the line has no word for either:

- A step the provider skipped, which leaves old code running, is `waiting` until the run is finished and `failed` then.
- A step retried reports its latest attempt.

## Where a watch stands

| State | Means | Tells the Job |
|---|---|---|
| `waiting` | Nothing counted has started | No |
| `running` | Something counted is going | No |
| `passed` | Counted parts passed, none set aside failed, and the line is finished | Yes |
| `failed` | A counted part failed | Yes, once a change |
| `not_ours` | Counted parts passed and a part set aside failed. An end once the line is finished | Yes, at the end, with what failed, and a Needs you item |
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
| Armada | Compiled in, from `crates/adapters/watch-recipes/` | `github-checks` and `buildkite-build` |
| Repository | `.armada/watches/` | Read from `main` only |
| This machine | `~/Library/Application Support/Armada/machine/watches/` | Does not travel |

> **Rule.** A value fills one whole argument and is never read by a shell.
> Why: an item such as `--sha={sha}` would need a shell to split it, and a watch never runs one.

> **Rule.** An agent may start a watch on a command of its own, with no recipe and no person's step.
> Why: the agent can already run any command it likes, and a recipe is for a watch a repository starts often.

### The recipes Armada ships

Each runs `armada watch-line`, a reader of Armada's own that prints the one line, by Fleet's own binary's whole path.

| Recipe | Values | Reads |
|---|---|---|
| `github-checks` | `sha`, `context` | The commit's check runs and commit statuses whose names start with `context`, each a part, with the token `gh` holds, in the repository the slot's remote names |
| `buildkite-build` | `organization`, `pipeline`, `build` | One build, each command step a part and a retried step by its latest attempt, with `BUILDKITE_API_TOKEN` from the login shell |

Each reader gives a failed part its link, its exact error lines and the end of its log: a failed check run's failure annotations, else its output's title and summary, and the end of its output's text; a failed status its description; a failed step the error lines in its own log, read from the provider, and the last 50 lines of it.

A provider's name is in the adapters only: `config` reads the files as any recipe.

### Keeping one on this Mac

`save_watch_recipe` writes a recipe into `~/Library/Application Support/Armada/machine/watches/`, and a script the Job wrote beside it, copied and made runnable.

> **Rule.** A saved recipe is never written into a repository.
> Why: a repository's recipes travel with it, and one an agent writes can name a private repository or a person's own script.

A name Armada ships is refused, and a name the repository or this Mac holds is refused unless the Job asks to replace it.

The recipe's `environment` is a [Requests](requests.md) environment, so its secrets come from the Keychain and are kept out of every reading and the side panel as `{{name}}`.
The cadence floor and the give-up cap are checked when a watch starts and not when the file loads, since a person moves them while Fleet runs.

## How Fleet runs one

> **Rule.** Watches run on a timer of their own, every 5 seconds, and take the ones that are due.
> Why: the pull request reading runs at most once a minute and waits on the forge, so it would halve a 30 second watch.

| Behaviour | Holds |
|---|---|
| A run | A Check's: no shell, a process group of its own, a time limit, low priority |
| Where | The Job's worktree, so an agent's own script runs as written and `gh` reads the repository from its remote |
| Environment | The person's login shell's over Fleet's, with `ARMADA_SPAWNED_BY`, so a crash's leftovers are ended, and the recipe's Requests environment over both |
| Slowing | Every 2 minutes after 30 unchanged, every 5 after an hour |
| Kept | The newest 50 readings in the store, so a restart carries on |
| Published | A changed reading at once, an unchanged one at most every 5 minutes |

> **Rule.** A watch's environment is the login shell's, read once when it starts and kept for its life.
> Why: Fleet runs under launchd, whose environment has neither the shell's `PATH` nor a token the person exports, and a watch whose environment changed under it would read differently for no reason it could see.

Fleet runs `$SHELL -lic` in the Job's folder, prints a marker either side of `env -0`, and reads only what lies between, since a prompt such as powerlevel10k prints escapes around it.
A shell that takes longer than 15 seconds, or prints nothing Fleet can read, gives Fleet's own environment, and the watch says why in `login_shell`.
After a restart Fleet reads it again at a live watch's first run.
A command that reads the keychain, such as `gh`, works because Fleet runs as the person.

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

## A terminal watch

A terminal watch runs nothing: it waits for text, or a regular expression, in the Job's terminal.

| Behaviour | Holds |
|---|---|
| Passes | On the first line printed after it started that matches, finished, and `says` quotes the line |
| Fails, and ends | When the terminal ends first, or is not open when it listens after a restart. Nothing is left to hear, so nobody weighs it |
| Read as | Each line as the person's pane draws it, so colours and a redrawn progress bar do not hide the words |
| Bounds | Its own give-up time; it is not one of the commands `limits.watchesAtOnce` bounds |

A start where the Job has no terminal open is refused.

## Setting one up

| Tool | Does |
|---|---|
| `try_watch` | Runs a command or a recipe once, as a watch would, and keeps nothing: what it read, the command, the folder, the `PATH`, whether the login shell was read, and the scratch folder |
| `list_watch_recipes` | Names each recipe a start may name, its values, where it came from, how often it runs and when it gives up |
| `save_watch_recipe` | Keeps a recipe and its script on this Mac |
| `list_watches` | The Job's live watches, and with `ended` its ended ones too |
| `delete_watch` | Forgets an ended watch and all kept of it. A live one is stopped first |

Each Job has a scratch folder of its own under Fleet's state, `$ARMADA_SCRATCH` in a hosted Job and `scratch` in every `try_watch` answer.
The slot guard lets the Job write, remove and chmod there as in its slots, and the folder is removed when the Job ends.

## The agent weighs it

> **Rule.** A failed reading is weighed by the agent, ours or not ours, each with a sentence saying why.
> Why: whether a red reading matters to the change needs the diff, and no rule in a recipe has it.

| Verdict | What it does to the watch |
|---|---|
| `ours` | Ends it `failed`. The agent fixes the change, and a redeploy is a new watch |
| `not_ours`, with parts | Sets those parts aside. The watch carries on, a change in them wakes nobody, and while one reads `failed` the watch can end `not_ours` and never `passed` |

A weighing tells nobody, since the agent gave it, except one that ends the watch `not_ours`: the Job is told it did not pass, so it does not report a pass.
A `not_ours` names a part some reading of the watch carried, or it is refused.
A `not_ours` carries the evidence the agent gathered for the person to send on: links to what it watched, the logs it saved, and the exact error lines.
A watch may start with parts already set aside, so a redeploy keeps what the agent learned on the last one.
A set-aside lasts one watch, so a step that fails on every build costs one wake on every push.

## Ended not ours

> **Rule.** A watch that ended `not_ours` is a Needs you item Fleet raises, never the agent.
> Why: whether a failure someone else owns is ignored, fixed here or sent to its owners is the person's call.

The item names the watch and the parts that failed, and its press opens a card with each part's sentence, the link to the failing run and three controls.

| Control | Does |
| --- | --- |
| Ignore | Settles it, and tells the agent nothing |
| Fix it | Sends the agent the parts, the sentence that set each aside and the link, and settles it |
| Escalate | Shows plain lines to copy for the owners of what broke: what failed, when, on which run, what the reading said, each part's exact error lines, the errors the agent found, every link, every log, and the end of each part's log. Fleet sends nothing anywhere, and the item stays |

Settling the item removes it, and dismissing it settles it too.

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
A held Job is not a Needs you item, since what it waits on is Fleet's wait and not the person's; an end `not_ours` is one, above.
Ending a Job ends its live watches `stopped` and tells nothing.

## In Bridge

| Where | What it draws |
|---|---|
| Delivery | A Watching block below the pull requests: a state glyph, the name, what it said and its age, with the agent's verdict under it. Live watches only, and one that ended not ours while its Needs you item is open, until Show ended draws the ended ones, each with Delete |
| The side panel, on a press | Readings over time, a run that read the same as one row with its count and first and last times, each part with its own link, exact error lines and the end of its log, parts set aside struck through, what a weighing carried as evidence, a broken run's output, a failed reading's whole log in Log with one copy control, the link, and Stop |
| A Job's row and header | Held open by the watch's name, while only a watch keeps the Job open |

The glyphs are `docs/contracts/iconography.md`, *A watch and its states*.
The word is `docs/contracts/design-system/voice-copy.md`, *Lexicon*.

## Where it lives in code

| What | Where |
|---|---|
| The model, with the counting and the verdicts | `crates/core-model/src/watch.rs` |
| Reading a command's last line | `crates/ipc/src/watches.rs` |
| Recipes | `crates/config/src/watches.rs` |
| The recipes Armada ships, and their readers | `crates/adapters/src/watch_lines.rs` |
| Start, weigh, stop and list | `crates/fleet/src/watches/serving.rs` |
| Try, list recipes and save one | `crates/fleet/src/watches/tools.rs` |
| The timer and the runs | `crates/fleet/src/watches/running.rs` |
| The login shell's environment | `crates/fleet/src/watches/environment.rs` |
| A terminal watch | `crates/fleet/src/watches/listening.rs` |
| A Job's scratch folder | `crates/fleet/src/scratch.rs` |
| The notices | `crates/fleet/src/watches/telling.rs` |
| A failed reading's log file | `crates/fleet/src/watches/logs.rs` |
| The Needs you item for a `not_ours` end | `crates/fleet/src/watches/needing.rs` |
| What is kept | `crates/store/src/watches.rs` |
| Bridge's copy of the wire | `packages/protocol/src/watches.ts` |

## Not built

| Follow-up | Why it waits |
|---|---|
| A watch that sends a saved Request and reads a field | Left for after v1: it fits the same line |
| A watch on what an environment is running | A new watch for whoever needs it, so a replaced deploy is news |

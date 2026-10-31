# Fleet watches CI and deploys for a Job, and the agent weighs what it sees

**Fleet runs the watching, on a command the repository or the person supplies, and wakes the Job's agent only when what it reads changes.**
Armada knows no provider: the command prints one line Fleet understands, and whatever a provider says is turned into that line by a script the repository or the person owns.
Whether a red reading matters to the change is the agent's call, recorded on the watch and shown beside it.

## The failure this answers

A Job pushed four pull requests to a shared QA environment, then had to know when the deploy finished and whether it failed.
The repository's CI runs on one provider and its service deploys on another, and the agent knew how to read both through skills the person wrote.
Reading was never the problem. Watching was.

- **Nobody watched for 48 hours.** A deploy was pushed on 19 Oct at 21:07, and the agent first read its state on 21 Oct at 20:37, when the person asked.
- **Watching was the person's job.** The agent ended its turn with "Run `/deploy-status 9 watch`", and that watcher was a loop spending a model turn every 4 to 5 minutes.
- **The one thing that worked was Fleet's notice.** When a pull request's checks failed, Fleet told the Job, and the agent acted on it within the turn.

Then, on 22 Oct, the watching needed judgement no rule could carry:

| What the provider said | What the agent found |
|---|---|
| The whole run `failed` | The tasks that deploy this change were still running. A run that is red overall can still be deploying |
| A unit test failed in another service | The change touched no file of that service, so the failure was not ours. The person confirmed that service's tests may fail |
| Two build tasks failed | They fail on the main branch for every branch, for a reason the change never touches |
| The QA branch's old tip was gone | Someone had reset the shared branch. The agent checked ancestry and found nothing of ours lost |

Fleet should do the watching, because it runs on a clock and costs nothing while it waits.
The agent should do the weighing, because the weighing needs the diff.
`docs/scope.md` draws the same line: "The system does the deterministic parts."

## The design

### A watch

A watch is a command Fleet runs every so often for one Job, until it says the thing it follows is finished or it gives up.
It has a name the person reads ("QA deploy"), the Job that started it, an optional pull request it follows, and the readings it has taken.

### What the command prints

> **Rule.** The command's last line on standard output is one JSON object, and Fleet reads nothing else.
> Why: the line is the whole contract between Armada and a provider, so Armada never learns a provider's words.

```json
{"state": "running",
 "finished": false,
 "says": "Deploy running, two of three services done",
 "link": "https://ci.example.com/runs/4411",
 "parts": [{"name": "build", "state": "passed"},
           {"name": "deploy-api", "state": "running"},
           {"name": "unit-tests-other", "state": "failed"}]}
```

| Field | Values | Notes |
|---|---|---|
| `state` | `waiting`, `running`, `passed`, `failed` | Required |
| `finished` | true or false | Required. True once what the watch follows will change no more |
| `says` | one sentence | Required. What Bridge draws on the row and what the notice quotes |
| `link` | an address | Optional. Where the person or agent reads more |
| `parts` | a list of name and state | Optional. A run whose overall state misleads is read part by part |

A command that exits non-zero, times out or prints no such line gives a **broken** reading, and its last output is kept for the side panel.
With `parts`, Fleet works out the watch's state from the parts the agent has not set aside, which the next section names: any `failed` is `failed`, any `waiting` or `running` is `running`, all `passed` is `passed`.
Without `parts`, the command's own `state` stands.
A watch ends `passed` only on a reading that is `passed` and `finished`.

> **Rule.** `finished` is a field of its own, and never read from `state`.
> Why: spike 36 found a run reads `failed` the same mid-run and at its end, so a watch with a failure set aside could never tell it was over.

Two rules every script keeps, because the line has no word for either (`docs/spikes/036-is-one-line-enough-to-watch-a-deploy.md`):

- **A step the provider skipped, which leaves old code running, is `waiting` until the run is finished**, and `failed` then. Reported as passed, it would end a watch on a deploy that never happened.
- **A step retried reports its latest attempt.**

### Recipes

A recipe names the command, how often to run it, and when to give up.
It sits in the repository's `.armada/watches/`, read from `main` only, or in `~/Library/Application Support/Armada/machine/watches/`, which does not travel.
The most specific level wins, and its copy replaces the others whole.

```yaml
name: qa-deploy
command: [scripts/watch/deploy-state, "{sha}", "{environment}"]
every: 60s
gives_up_after: 2h
```

The agent starts a watch by naming a recipe and filling its values, or by giving the command, how often and when to give up itself.
A value fills one whole argument and is never read by a shell.

> **Rule.** An agent may start a watch on a command of its own, with no recipe and no person's step.
> Why: the person decided a watch needs no one's sign-off; the agent can already run any command it likes, and a recipe is for a watch a repository starts often.

The side panel shows a watch's whole command, so the person can see what Fleet is running for the Job.

**My call: a watch runs in the Job's worktree**, so an agent's own script, or a branch's copy of a recipe's, runs as the agent wrote it.
A watch whose folder is gone, because the Job gave its worktree back, gives broken readings and stops, and the agent is told.
A machine's recipe runs its command by absolute path.

**My call: the command gets the environment a Check gets**, which is Fleet's own, so a provider's command-line tool finds the person's login.
A recipe may also name a [Requests](../concepts/requests.md) environment, and its variables, secrets included, are set for the command.
A secret never appears in a reading or the side panel.

### Fleet runs it

Watches run on a 5 second timer of their own, which takes the ones that are due.
The pull request reading in `crates/fleet/src/main_ci.rs` was the first choice, but it runs at most once a minute and waits on the forge, so it would have halved a watch every 30 seconds.
A reading that says the same as the last is published to Bridge at most every 5 minutes, since a watch's history would otherwise be sent whole every run.
A command runs the way a Check does (`crates/checks-runner/src/run.rs`): no shell, its own process group, a time limit, and the low priority of `crates/checks-runner/src/priority.rs`.
It carries the `ARMADA_SPAWNED_BY` mark, so `crates/fleet/src/orphans.rs` ends what a crash left running.
Readings are kept in the store, the last 50 a watch, so a restart carries on where it stopped and Bridge can draw the history.

### The agent weighs it

When a watch turns `failed`, Fleet wakes the agent with the reading.
The agent reads whatever it needs and weighs the reading in one of two ways:

| Verdict | What it does to the watch |
|---|---|
| **Ours** | The watch ends `failed`. The agent fixes the change, and a redeploy is a new watch |
| **Not ours**, with a sentence and the parts to set aside | The watch carries on. Those parts no longer count toward its state, and a change in them wakes nobody |

A watch may be started with parts already set aside, each with its sentence, so a redeploy's watch keeps what the agent learned on the last one rather than waking for it again.
Spike 36 counted four wakes for failures the agent called not ours across three runs, two of them the same step on consecutive runs.

Bridge draws the verdict and its sentence under the reading, so the person sees what the provider said and what the agent made of it, side by side.
An agent that learns a standing rule, such as the person saying one service's tests may fail, keeps it where it keeps its other lessons. The watch only holds what was set aside on it.

### Telling the Job

A watch tells its Job through the path pull request notices use (`told_by_fleet`, `crates/fleet/src/session_host/serving/telling.rs`), and under the same rules (`docs/concepts/fleet.md`, *Telling the owner of a pull request*).

| The watch | The agent is | The notice reads |
|---|---|---|
| Turns `failed` | Woken | QA deploy failed: deploy-api failed. And the link |
| Ends `passed` | Woken, so it carries on | QA deploy passed |
| Gives up | Woken | QA deploy gave up after 2h. It last said: two of three services done |
| Breaks three readings in a row | Woken | QA deploy stopped: its command failed three times. And the last output |
| `waiting` or `running` | Not told | |

> **Rule.** Each notice is told once: the store keeps the watch, its state and the parts that failed, and a restart does not repeat it.
> Why: a reading taken again is not news, as with a pull request read again on the same commit.

**An agent does not watch a pull request's own checks.**
Fleet's pull reading already tells the Job when they fail, and spike 36 found the two would have woken the agent twice for one failure.
A watch is for what the pull reading cannot see: a deploy, an environment, a pipeline the push to another branch starts.

**A watch follows what it was started on, and nothing after it ends.**
In spike 36 someone reset the shared QA branch two days after the watch would have ended `passed`, and no watch saw it.
Noticing that is a new watch's job, started by whoever needs it.

### A Job a watch holds open

> **Rule.** A Job with a live watch is not finished, and Bridge says that the watch is what holds it.
> Why: the person decided a Job is done once its deploy settles, not when its pull request merges, and he must be able to tell a Job held by a watch from a Job still working.

A live watch is a wait, as an open pull request is (`crates/adapters/src/session_rules.md`): the agent does not call `all_done` while one runs, and ends its turn.
The watch names the wait itself, so the agent adds no `waiting_for` item for it.
A Job whose turn has ended, with nothing in `waiting_for` and a live watch, shows as **held open by** the name of each live watch, in the order they started, with the watch glyph.
Fleet lists the Job's own open pull requests in the merge queue, set to merge or on their Checks in the same `held_open_by`, so a surface words the list and derives none of it.
The person's cross on one of those pull requests (`dismiss_wait`) stops Fleet listing it for that Job from then on, and does nothing to the pull request: it still merges and the agent is still told.
When the watch ends, whichever way, its notice wakes the agent, which carries on and calls `all_done` once nothing is left.
The person ends the Job with **End job**, as today (`docs/concepts/session/waiting-on-the-person.md`). Nothing ends a Job on its own.
A held Job is not a Needs you item: what it waits on is Fleet's wait, not the person's, as with a merge the forge will do (`docs/concepts/fleet.md`).

### In Bridge

| Where | What it draws |
|---|---|
| Delivery | A **Watching** block below the pull requests. A row is the state glyph with its tooltip, the watch's name, its `says` sentence and the reading's age. Under it, the agent's verdict and sentence where there is one |
| The side panel, on pressing a row | The readings over time, each part and its state, parts set aside struck through with the verdict that set them aside, the last output of a broken reading, the link out, and **Stop** |
| A Job's row on the Board and its header | **Held open by QA deploy**, with the watch glyph, while only what is not the person keeps it open |

The watch glyph is `radar`, which `docs/contracts/iconography.md` has not assigned.
`eye` is reserved to Review.

### Limits

| Limit | Default | Setting |
|---|---|---|
| Watches running a command at once, across the machine | 2 | `limits.watchesAtOnce` |
| Live watches a Job may hold. One more is refused, and the refusal says so | 5 | `limits.watchesPerJob` |
| Time one run of the command may take before it is stopped | 30s | `timeouts.watchRunSeconds` |
| Fastest a recipe may ask for | every 30s | `timeouts.watchEverySeconds` |
| Slowing when nothing changes | 2m after 30 minutes unchanged, 5m after an hour | |
| Give up when a recipe names no time | 2h. A recipe may ask for up to 24h | `timeouts.watchGiveUpMinutes` |
| Broken readings in a row before the watch stops | 3 | |

A watch also ends when its Job ends, and when the person presses Stop.

### On the wire

Four operations, for the agent through Fleet's tools and for Bridge: `start_watch` (recipe, values, name, the pull request it follows), `weigh_watch` (verdict, sentence, parts to set aside), `stop_watch`, and `list_watches`.
The names are the builder's to settle against `docs/practices/protocol.md`.

## What it costs

- **Provider knowledge moves, it does not vanish.** Each repository, or each person, writes the script that turns a provider's output into the line. For the Job above, that is two scripts.
- **Fleet runs commands an agent chose, on its own clock, outside the agent's permission prompts.** The limits bound how much, and the side panel shows what.
- **A noisy provider costs model turns** until the agent sets its noisy parts aside. Every `failed` that is not ours is one wake.
- **Fleet gains a long-lived thing of its own**: a schedule on the pull request reading, a store table, commands to end after a crash.
- **A Job gains a held state**, which the Board, the ledger and the Job's header must all draw the same way.

## What it rules out

- **The agent polls**, with a loop, a background `sleep` or a `Monitor`.
  It is what happened, and it spent a turn a tick, stopped when the conversation stopped, and left a 48 hour gap.
  `docs/scope.md`: "Every time a design reaches for 'ask the agent', it is reaching for the attempt that was already rejected."
- **An adapter per provider in `crates/adapters`.** `ARCHITECTURE.md` allows it, and Fleet reads pull requests this way.
  But deploy systems vary per repository, and Armada would chase every one.
  The person asked for none.
- **Webhooks from providers.** A laptop has no address a provider can reach, and each provider signs and shapes its hooks differently.
- **Fleet deciding relevance from rules in the recipe.** "This failure is in a service the change does not touch" needs the diff.
  A rule list in the recipe would be wrong on the first change that touches that service.
- **The watch ends with the Job, or outlives it and starts a new Job on red.** The person chose to keep the Job open until its watches settle.
- **Each watch nested under its pull request.** The person chose one Watching block. A watch can follow no pull request, such as an environment's health.
- **A watch that sends a saved [Request](../concepts/requests.md) and reads a field of the answer.** It would need no script for a plain health address, and it fits the same line. Left for after v1.

## What this changes that is already written

| Document | Change |
|---|---|
| `docs/concepts/fleet.md` | A section on watches beside *Telling the owner of a pull request* |
| `docs/concepts/session/what-fleet-tells-a-job.md` | The four watch notices |
| `docs/contracts/design-system/voice-copy.md` | **Watch** in the lexicon |
| `docs/contracts/iconography.md` | `radar` for a watch, and the glyphs for its states |
| `crates/config/settings.toml` | The settings under *Limits* |
| `crates/adapters/src/session_rules.md` | Start a watch rather than poll. A live watch is a wait, as an open pull request is |
| `docs/concepts/watch.md` | New, once built |
| Fleet's code that calls reading a pull request a watch (`pull_refreshing.rs`, `after_watching.rs`, `linked_watching.rs`, `rail_watching.rs`, `usage_watch.rs` in `crates/fleet/src/`) | Renamed, so **Watch** means one thing. The person chose the word |

## What would make this wrong

**That one line, with parts, is enough to know when to look.**
If a provider's state only makes sense with context a script cannot summarise, every reading is noise and the agent is woken for most of them.
Spike 36 checked it against the Job above, replaying three of its deploy runs a minute at a time: 7 wakes in 52 readings, each at a moment that mattered, 4 of them set aside as not ours.
Without `parts` all three runs read `failed` within 5 minutes of the push while deploys were still going, and that is why `parts` and `finished` exist.
The CI provider's half was not measured: this machine held no login its script could use.

## Left to the builder

- The recipe file's exact shape, and how a value is checked against the recipe.
- The glyph for each state, from the registry.
- How the side panel draws readings over time.
- How a `weigh_watch` naming a part the watch never reported is refused.

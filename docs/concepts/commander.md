# Commander

**What it is:** The agent that steers one Job. It is Helm's conversation, hosted by the Job and not by the repository, woken by Fleet when the Job is in trouble and by the owner whenever he writes to it.

---

**Kind:** Fleet record and host. `#2166`. Why it exists is `.claude/decisions/2026-10-11-a-job-has-a-commander.md`; Helm is [helm.md](helm.md), whose host it borrows. The code is `crates/fleet/src/helm/commander.rs`, `commander_brief.rs`, `commander_card.rs` and `conversation.rs`, with the launch in `crates/adapters/src/commanding.rs`.

## What it is not

**It does not do the work.** The Drones do the tasks. A Commander reads the Job and steers, and it has no tool that edits a file or runs a command.

**It claims nothing.** The Checks and the Judge decide whether work advances, and nothing a Commander says is read as a claim that moves a Job. `../scope.md`'s third attempt rejected a transcript read as a claim; a Commander's thread is a thread.

## What it is

| | |
|---|---|
| Conversation | One per Job, keyed `commander:<job id>` in `helm_sessions`, so it can never be a repository's key. Resumed by id each turn, on Helm's retention |
| Process | One per turn, started in the **Job's worktree**, never the owner's checkout |
| Launch | `--strict-mcp-config` with the door alone, `--tools` cut to `Read`, `Grep` and `Glob`, every other built-in denied by name, and `dontAsk`. There is no owner to ask, so a write tool is not there to be asked about. A test holds this apart from Helm's launch and a Drone's |
| Placed | By the process that holds its connection, as Helm is: `Hosting::running_commanders` names each live turn's process and Job, and `Admitting::commander_at` answers with that Job's `CommanderReach` |
| Reach | Every query, and the acts in `crates/api/src/daemon/commander.rs`, one of which is the card. **Narrowed to its own Job** where the door resolves a Job: an act naming another is refused with a line naming both. Reads of other Jobs are not narrowed |
| Model | `models.commander` (Machine, default the strongest on the roster) and `effort.commander`, with **a Job's own value over them** from `set_commander_model`, each half on its own. `docs/contracts/configuration.md`, *A Commander's model* |
| Spend | Recorded in `job_commander_spend` and shown as `commander_cost_micros` beside the Drones' `cost_micros`. **Never counted against the cost cap**, which sums the Drones' rows alone: the agent that raises a cap cannot spend it |

## Turns

**One at a time, and nothing interrupts one.** Everything that wants a turn goes on one queue: the owner's words (`ask_commander`) and Fleet's wakes. A turn takes everything waiting as one, so ten wakes in a row are one turn. The owner's words are on the thread the moment he sends them, and answered by the next turn.

**A wake is a reason in a line.** `Fleet::wake_commander` is the seam, and `crates/fleet/src/commanding_wakes.rs` decides when to call it, from the typed values a turn leaves. Where the Commander cannot start (the Job has no worktree), it writes a line to the Job's log naming why and returns `Err`, and the Job stays with the owner as it did before there was a Commander. A turn that starts and does not come back leaves `unanswered` on the thread and the same line in the log.

## When it wakes

Every Job has a Commander, and it wakes only on trouble. Each trouble is read from a typed value, never a log string, and its reason is the thread's `woke` row.

| Trouble | Read from | The `woke` row |
|---|---|---|
| A Drone ended with nothing handed in | `Worked::after` as `Aftermath::JobMoves` to `silent`, `stalled`, `blocked_by_policy` or `interrupted`; `Worked::quiet` as `Vigil::AtRest` or `Vigil::Escalated` | The Drone on step *s* ended with nothing handed in: stalled |
| A group red with its retries spent | `Worked::ruled` as `GroupEnd::Failed` | Step *s* failed its Checks with its retries spent |
| A Judge refusal | `Worked::ruled` as `Ruling::Refused` or `Ruling::Questioned` | The Judge refused step *s* |
| Work past the plan | `Worked::drifting`; `Worked::wandering` once the Drone has not answered the instruction to report | The Drone on step *s* is working outside the plan |
| A broken worktree or environment | `Turned::broken`: a Job moved onto `no_worktree`, `not_configurable`, `would_not_start`, `resource_exhausted` or `not_prepared` | No Drone could run: the worktree was unusable |
| A cap nearly spent | `Turned::spent`: a Job a Drone's spend was just written for, against `Allowance::nearly_spent_by` at 80% of the cost cap or of the turn cap | The cost cap is nearly spent: $4.25 of $5.00 |

The last two have no turn to sit on, because an environment failure happens while a Job is admitted and spend is written when a Drone ends. Each leaves a note in memory (`Stirred`) that the next turn carries.

**Once each.** A wake is recorded in `job_commander_wakes` before it is made, keyed by trouble, step and the step's attempt. A Job-level trouble has no step, and its attempt is how many times the Job has escalated; a cap has none either, and its attempt is the cap in force, so a raised cap is a new fact. The 250 ms tick and a Fleet restart both find the row and wake nothing. A crash between the row and the wake loses that wake and never repeats it.

**The loop bound.** The same trouble on the same step, woken for three times while the Job stands exactly as it did at the last wake, is a loop, and the fourth wake is not made. The Job stands as it did when its status, any step's state, or the number of tasks in any state is the same as at the last wake; a Job that restarted a step and met the same trouble again has not moved, and one that handed in a task has. At the bound the Commander stops acting on that problem, the Job's log says so in one warning line, and **a card goes to the owner in its place** (below). The bound moves nothing: a Job still running (drift, a cap nearly spent) keeps running, and an already escalated one stays escalated with the card beside it. A different problem still wakes the Commander, and so does the owner's answer to the card.

## The card

A Commander cannot reach the owner in a reply he may not read. It reaches him with one card, which is the only way it does.

| | |
|---|---|
| Raised by | The Commander, with `ask_the_owner`: what is wrong, what is out of its reach, a question, and two to four options. Fleet, at the loop bound, with the trouble for what is wrong and *Try again* and *Leave it* for the options |
| Kept as | A row in `commander_cards`, **modelled on a Drone's question and never a held tool call**. Helm's ask is a call held open that dies at five minutes and silence there is a refusal; a card waits until he answers, however long, and a Fleet that restarts still has it |
| One at a time | A Job has one card out. A second `ask_the_owner` is refused with the question already waiting, and the loop bound leaves a card out as it is |
| Counts as | The Job waiting on the owner, **like an escalation and without one**: the Job's row reads `asking`, which is what puts it in Your move, and `list_alerts` names it in the blocked list. No status moves |
| Sleep mode | Records it as blocked, whether it went out before the night or during it, and never answers it: the pick is his |
| Answered by | `answer_commander_card`: the card's id and an option's id, no prose. A card that is not the one out, or an option it never offered, is refused. `agent_access` is `No` |
| Then | The answer is kept, then the Commander is woken with it: *The owner answered your card, "<question>": <label>*. That wake is not counted against the bound, so he can send it back in |
| Only a Commander raises one | `ask_the_owner` is `Commander only` in the operations inventory, listed to a Commander and to no other caller, Helm included, and the route refuses a call the door did not place as a Commander's |

Where the Commander cannot start at all, nothing here applies: the Job escalates as it did before there was a Commander.

## On the wire

`observe_commander` is a socket per Job, `helm.md`'s shape: `opened`, the thread so far, then every message, with `missed` for what a slow viewer lost. Its rows are the transcript's `Saw`, and `woke` is the message only this socket has. `ask_commander` takes the owner's words and answers 202. `commander.changed` says whether the Commander is at work, and `commander.card` says a card went out or was answered. `agent_access` is `No` on those and on `answer_commander_card`: an agent must not write the owner's words to a Commander, nor pick for him. `../practices/protocol.md`, *The Commander socket*.

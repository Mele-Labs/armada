# Commander

**What it is:** The agent that steers one Job. It is Helm's conversation, hosted by the Job and not by the repository, woken by Fleet when the Job is in trouble and by the owner whenever he writes to it.

---

**Kind:** Fleet record and host. `#2166`. Why it exists is `.claude/decisions/2026-10-11-a-job-has-a-commander.md`; Helm is [helm.md](helm.md), whose host it borrows. The code is `crates/fleet/src/helm/commander.rs`, `commander_brief.rs` and `conversation.rs`, with the launch in `crates/adapters/src/commanding.rs`.

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
| Reach | Every query, and the acts in `crates/api/src/daemon/commander.rs`. **Narrowed to its own Job** where the door resolves a Job: an act naming another is refused with a line naming both. Reads of other Jobs are not narrowed |
| Model | `models.commander` (Machine, default the strongest on the roster) and `effort.commander`. A Job's own value over them is owed with the card |
| Spend | Recorded in `job_commander_spend` and shown as `commander_cost_micros` beside the Drones' `cost_micros`. **Never counted against the cost cap**, which sums the Drones' rows alone: the agent that raises a cap cannot spend it |

## Turns

**One at a time, and nothing interrupts one.** Everything that wants a turn goes on one queue: the owner's words (`ask_commander`) and Fleet's wakes. A turn takes everything waiting as one, so ten wakes in a row are one turn. The owner's words are on the thread the moment he sends them, and answered by the next turn.

**A wake is a reason in a line.** `Fleet::wake_commander` is the seam `#2167` calls, and it decides when to call it: a Drone ending with nothing handed in, a group red with its retries spent, a cap nearly spent. Where the Commander cannot start (the Job has no worktree), it writes a line to the Job's log naming why and returns `Err`, and the caller escalates as it did before there was a Commander. A turn that starts and does not come back leaves `unanswered` on the thread and the same line in the log.

**The loop bound is the brief's, counted elsewhere.** Three wakes on one problem without the Job moving means stop and say so. The brief states the rule; counting wakes is `#2167`'s.

## On the wire

`observe_commander` is a socket per Job, `helm.md`'s shape: `opened`, the thread so far, then every message, with `missed` for what a slow viewer lost. Its rows are the transcript's `Saw`, and `woke` is the message only this socket has. `ask_commander` takes the owner's words and answers 202. `commander.changed` says whether the Commander is at work. `agent_access` is `No` on all three: an agent must not write the owner's words to a Commander. `../practices/protocol.md`, *The Commander socket*.

## What is owed

**A card that outlives a turn** is owed with the second part of `#2166`: a pending question in `commander_cards`, modelled on a Drone's question and never a held tool call, counted as the Job waiting on the owner. Until it lands, a Commander that cannot fix something says so in its reply.

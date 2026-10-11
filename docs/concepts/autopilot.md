# Autopilot

**What it is:** A repository working its own backlog: a record per repository holding one plain-language instruction, how many of its Jobs may run at once, optional caps, and whether it is running, paused or stopped, with every issue it picked.

---

**Kind:** Fleet record. `#2092`. Why it exists, and what it is not, is [`../capabilities/autopilot.md`](../capabilities/autopilot.md). The code is `crates/fleet/src/autopilot.rs` and the modules under it; the store's half is `crates/store/src/autopilot.rs`.

## What it holds

| Field | Notes |
|---|---|
| Instruction | The person's words. The picker reads it; Fleet never parses it |
| At once | How many of its Jobs may hold a place together |
| Until | A time, held in UTC. Optional |
| Spend | A dollar total across its Jobs, in millionths. Optional |
| Jobs | How many it may start. Optional |
| State | `running`, `paused` or `stopped`, with when it started and ended |
| Capped by | `until`, `spend` or `jobs`, where a cap rather than a person stopped it |
| Picks | Each issue it picked, the workflow, the Job it started, the picker's reason, and `started`, `set_aside`, `withdrawn` or `dismissed` |
| Fault | Why the last pass picked nothing: the tracker would not answer, or the picker's answer was refused. In memory, and gone once a pass goes through. The only place a pass's failure is told |

**One per repository.** Starting a stopped one again replaces it, and the old run's picks are no longer served. Starting over one that is running or paused is refused.

## How a start happens

1. The person types one sentence. One model call, made the way the [Job proposer](job-proposer.md)'s is, reads it as the five settings. **It starts nothing.**
2. The person confirms or changes them, and the start takes what they confirmed. A blank instruction, no room, or a cap already reached is refused.

## How a pick happens

On Fleet's noticing interval, in a task of its own so a landing never waits behind it, each running Autopilot with room is asked for picks:

1. **The caps first.** The first one reached stops it, and says which.
2. **The room.** Its Jobs that hold a place are counted against *at once*.
3. **What it may pick.** The open issues Fleet holds for the [tracker](tracker.md), less any a pull request already closes, any a Job not yet ended came from, and any this run already picked.
4. **One call per pick**, a Policy rather than a Drone, so it holds no place. It names an issue it was offered, a workflow the repository holds, whether it is sure, and why.

| The picker | What happens |
|---|---|
| Is sure | The Job is drafted as a dispatch from the issue would be, with the issue as its source, origin `autopilot_picked`, and queued by Fleet with no approval |
| Is unsure | Set aside with its reason, and the next issue is asked about in the same pass |
| Finds none that fits | Nothing is picked, and it is not asked again until the list it was offered changes |
| Names an issue or workflow it was not offered | Refused. Nothing starts and nothing is set aside |

A pass makes at most four calls per Autopilot, and asks again before keeping each answer, so a Pause pressed while a call was out wins.

## Rules

> **Rule.** The dispatch gate is skipped for a sure pick and nowhere else.
> Why: the owner's call of 11 Oct 2026, `.claude/decisions/2026-10-11-autopilots-clear-picks-start-unasked.md`. In code it is one function that takes the sure pick whole, and only reading the picker's answer makes one.

> **Rule.** A Job waiting on a person holds no place.
> Why: the owner's *doesn't block the queue*. A Job whose Drone asked and is waiting, a paused one, and one escalated, at review, awaiting repair or attestation, or piloted, is not counted against *at once*.

> **Rule.** Each cap stops the next pick and never a running Job.
> Why: a Job's cost arrives when its Drone ends, the reason #51's cap refuses the next dispatch. Autopilot can overshoot a cap by as many Jobs as it runs at once. The per-Job ceilings of #51 still hold under it.

> **Rule.** Pause and Stop never stop a Job that is spending.
> Why: `.claude/decisions/2026-10-11-pause-drains-stop-withdraws.md`. That is the Job's own Pause. Pause stops the picks and Resume carries on. Stop also withdraws every pick whose Job has never had a Drone, killing that Job as the person's act, and ends it.

> **Rule.** A set-aside pick a person dispatches goes through the gate.
> Why: a person is choosing it, so the 3 Sep rule holds. Its Job is proposed from the issue as any dispatch is and waits at `awaiting_approval`, origin `manual`; the pick reads `started` and counts toward the spend and the Jobs started.

## On the wire

`list_autopilots`, `get_autopilot`, `propose_autopilot`, `start_autopilot`, `pause_autopilot`, `resume_autopilot`, `stop_autopilot`, `dispatch_set_aside`, `dismiss_set_aside`, and the event `autopilot.changed`. `crates/ipc/operations/` has each, and `../practices/protocol.md` has the section.

## What it does not do yet

| Missing | Where |
|---|---|
| The Cockpit lane that draws it | `#2092`'s Bridge half |
| `autopilot.changed` reaching the windows | The same. Fleet publishes it; main does not pass it on yet |
| Its two prompts in settings.json | Both are shipped text in `crates/fleet/src/autopilot/` |

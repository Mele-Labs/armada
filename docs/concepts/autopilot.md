# Autopilot

**What it is:** A repository working its own backlog: a record per repository holding one plain-language instruction, how many of its Jobs may run at once, optional caps, and whether it is running, paused or stopped, with every issue it picked.

---

**Kind:** Fleet record.
`#2092`; on Jobs since the 15 Oct 2026 decision, `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`.
Why it exists, and what it is not, is [`../capabilities/autopilot.md`](../capabilities/autopilot.md).
The code is `crates/fleet/src/autopilot.rs` and the modules under it; the store's half is `crates/store/src/autopilot.rs`.

## What it holds

| Field | Notes |
|---|---|
| Instruction | The person's words. The picker reads it; Fleet never parses it |
| At once | How many of its Jobs may hold a place together |
| Until | A time, held in UTC. Optional |
| Spend | A dollar total across its Jobs' reported costs, in millionths. Optional |
| Jobs started | How many Jobs it may start. Optional. The store calls it `jobs` and the wire `job_cap` |
| State | `running`, `paused` or `stopped`, with when it started and ended |
| Capped by | `until`, `spend` or `jobs`, where a cap rather than a person paused it. A Resume clears it |
| Picks | Each issue it picked, the workflow, the Job it started, the picker's reason, and `started`, `set_aside`, `withdrawn` or `dismissed` |
| Fault | Why the last pass picked nothing: the tracker would not answer, or the picker's answer was refused. In memory, and gone once a pass goes through. The only place a pass's failure is told |

**One per repository.** Starting a stopped one again replaces it, and the old run's picks are no longer served. Starting over one that is running or paused is refused.

## How a start happens

1. The person types one sentence.
   One model call, made the way the [Proposer](job-proposer.md)'s is, reads it as the five settings, each with the words of the sentence it was read from.
   A quote the sentence does not hold is dropped rather than shown as the person's.
   **It starts nothing.**
2. The person confirms or changes them, and the start takes what they confirmed. A blank instruction, no room, or a cap already reached is refused.

## How a pick happens

On Fleet's noticing interval, in a task of its own so a landing never waits behind it, each running Autopilot with room is asked for picks:

1. **The caps first.** The first one reached pauses it, and says which.
2. **The room.** Its Jobs that hold a place are counted against *at once*: the ones it started that are live, at work, and not waiting on a person.
3. **What it may pick.** The open issues Fleet holds for the [tracker](tracker.md), less any a pull request already closes, any a live Autopilot Job of any run came from, and any this run already picked.
4. **One call per pick**, a Policy rather than an agent, so it holds no place. It names an issue it was offered, a workflow the repository holds, whether it is sure, and why.

| The picker | What happens |
|---|---|
| Is sure | A Job is started with `start_session`, as Dispatch and `start_another_session` start one: titled with the issue, on the workflow the picker chose, and sent a first message from Autopilot that holds the issue's link, its body read the way a dispatch from the link reads it (else its title and excerpt) and the picker's reason. No one is asked. A Job that cannot be sent it is closed and the pass says why |
| Is unsure | Set aside with its reason, and the next issue is asked about in the same pass |
| Finds none that fits | Nothing is picked, and it is not asked again until the list it was offered changes |
| Names an issue or workflow it was not offered | Refused. Nothing starts and nothing is set aside |
| Fails, or answers in a shape that cannot be read | The fault is kept, and it is not asked again for 30 seconds, twice as long after each further failure in a row, up to 15 minutes. An answer ends the rest |

Autopilot never comments on, labels or closes the issue it picked, and nothing in Fleet does that for the owner unless he presses for it (see [tracker.md](tracker.md)).

Its two prompts are settings, `prompts.autopilotSentence` and `prompts.autopilotPicker`, under Prompts: Autopilot.
A pass makes at most four calls per Autopilot, and asks again before keeping each answer, so a Pause pressed while a call was out wins.

## Rules

> **Rule.** Only a sure pick starts a Job with no person pressing.
> Why: the owner's call of 11 Oct 2026, `.claude/decisions/2026-10-11-autopilots-clear-picks-start-unasked.md`.
> A Job has no dispatch gate, so what the call decides now is whether a person is asked.
> In code it is one function that takes the sure pick whole, and only reading the picker's answer makes one.

> **Rule.** A Job waiting on a person holds no place, nor one that has replied or gone idle, and an ended one holds none.
> Why: the owner's *doesn't block the queue*.
> A Job with a question, a permission, a plan to approve or a done standing for the person is not counted against *at once*.
> So is one that has replied or gone idle with nothing standing: an Autopilot Job rarely ends on its own, and holding its place until it did would stall the queue (the owner's 17 Oct).
> It stays open, and still counts as started.

> **Rule.** Each cap stops the next pick and never a running Job.
> Why: a Job's cost arrives as it works. Autopilot can overshoot a cap by as many Jobs as it runs at once.

> **Rule.** A reached cap pauses Autopilot, and Resume waits for it to be raised.
> Why: the owner's *fix the gaps*, 12 Oct 2026, `.claude/decisions/2026-10-11-autopilot-carries-time-money-and-job-caps.md`.
> A cap that ended it made a person start it again to carry on.
> `update_autopilot` changes the instruction, at once and the caps of one running or paused, in its state; Resume is refused, naming the cap, while any cap is still reached, since it would pick once and pause again.

> **Rule.** Pause and Stop never stop a Job that is working.
> Why: `.claude/decisions/2026-10-11-pause-drains-stop-withdraws.md`.
> Pause stops the picks and Resume carries on.
> Stop also withdraws every pick whose Job has never run, closing that Job as the person's act, and ends it.
> A Job starts on its brief at once, so there is usually none to withdraw.

> **Rule.** A set-aside pick a person dispatches starts a Job at once.
> Why: a person is choosing it.
> The Job starts as a sure pick's does, with no reason line in its brief; the pick reads `started` and counts toward the spend and the Jobs started.
> A cap does not refuse it.

## On the wire

`list_autopilots`, `get_autopilot`, `propose_autopilot`, `start_autopilot`, `update_autopilot`, `pause_autopilot`, `resume_autopilot`, `stop_autopilot`, `dispatch_set_aside`, `dismiss_set_aside`, and the event `autopilot.changed`.
`crates/ipc/operations/` has each, and `../practices/protocol.md` has the section.

## In Bridge

The Cockpit draws it: the Autopilot panel along the deck's foot, whose switch is its title and takes the running or paused state of the picked repository's Autopilot, and the Autopilot lane on the stage.
Switching it on, with a repository picked, gives the stage to the lane and its bar for that repository, and switching it off puts the lane away without stopping anything (the owner, 13 Oct 2026).
The bar holds the switch, the caps used of their limit, Pause, Resume and Stop, Edit caps, and the start form, which quotes under each setting the words Fleet read it from and says *not said* where it quoted none.
A cap that paused it is named in the bar's state and lit; Edit caps raises it in the start form's fields, and Resume stays pressable and says Fleet's refusal while the cap holds.
On **All** the lane asks for a repository with a picker, and the bar opens once one is picked.
Under the lane the sidings are Next up, as a numbered list, and Set aside; each set-aside pick is also a call.
The dispatch panel offers Autopilot as a third kind: the repository is read from the words or asked, the panel's act reads the sentence into settings it draws for the person to edit, and Start starts it.
Stop says before it is pressed which picks it withdraws and which Jobs go on, read from the Jobs the window holds by each pick's `session_id`.
`autopilot.changed` reaches every window through main, which keeps no copy.
The code is `apps/desktop/src/renderer/src/cockpit/AutopilotLane.tsx` and the composition `AutopilotBar`.

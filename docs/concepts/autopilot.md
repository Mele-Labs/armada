# Autopilot

**What it is:** A repository working its own backlog: a record per repository holding one plain-language instruction, how many of its Jobs may run at once, optional caps, and whether it is running, paused or stopped, with every issue it picked and every decision it answered.

---

**Kind:** Fleet record.
`#2092`; on Jobs since the 15 Oct 2026 decision, `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`.
Why it exists, and what it is not, is [`../capabilities/autopilot.md`](../capabilities/autopilot.md).
The code is `crates/fleet/src/autopilot.rs` and the modules under it, the decision pass in `crates/fleet/src/autopilot/decisions.rs`; the store's half is `crates/store/src/autopilot.rs`.

## What it holds

| Field | Notes |
|---|---|
| Instruction | The person's words. The picker reads it; Fleet never parses it |
| At once | How many of its Jobs may hold a place together |
| Until | A time, held in UTC. Optional |
| Spend | A dollar total across its Jobs' reported costs, in millionths. Optional |
| Jobs started | How many Jobs it may start. Optional. The store calls it `job_cap` and the wire `session_cap` |
| On autopilot | `backlog` and `running`, two booleans: whether it works the backlog, and whether it works Jobs already running. Kept as checked at start. A start without them, and a record from before them, read as `backlog` true and `running` false |
| Decisions | The kinds of decision a helper may answer: `recommended` (a question with a recommended answer), `open` (a question with none), `plan`, `draft`, `walk`, `land` (a green pull request), `unsure` (an issue Autopilot was unsure of), `red` (a fix for a red main), `reads` and `writes`. There is no destructive kind: it always waits for a person, and a body naming it is refused. A kind not checked waits for a person. Kept as checked at start, and a change of settings leaves them |
| State | `running`, `paused` or `stopped`, with when it started and ended |
| Capped by | `until`, `spend` or `sessions`, where a cap rather than a person paused it. A Resume clears it |
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

Its prompts are settings, `prompts.autopilotSentence`, `prompts.autopilotPicker` and `prompts.autopilotDecider`, under Prompts: Autopilot.
A pass makes at most four calls per Autopilot, and asks again before keeping each answer, so a Pause pressed while a call was out wins.

## How a decision happens

While an Autopilot is running and has a kind checked, a loop of its own beside the pick's, so a landing never waits behind a call, goes over the repository on Fleet's noticing interval.
A paused or stopped Autopilot answers nothing.

1. **The Jobs in scope.** Every live Job of the repository where *on autopilot* holds `running`, and otherwise only the ones it started.
2. **The kind.** An item waiting on a person is sorted into one kind, and a kind not checked waits.
3. **The wait.** An item has waited two minutes before it is asked about, so an agent can carry on by itself first.
4. **One fresh call per decision, per repository.** It is shown the instruction, the decision and the repository's last twenty answers, each correction beside the answer it replaces.
   It names one way to answer and says whether it is sure, then ends.
5. **The answer** is given as Bridge's press gives it, and put on record.

A pass makes at most four calls per repository.
A call that names no way, says it is not sure or cannot be read leaves the item with the person, and it is not asked again until the item changes.
A call that fails rests as the picker's does.

| Kind | What it answers | What the answer does |
|---|---|---|
| `recommended` | A question whose options include one labelled *(Recommended)* | Chooses the option the call names |
| `open` | A question with options and none recommended | Chooses the option the call names |
| `plan` | A plan waiting for approval | Approves it |
| `draft` | A draft waiting on the person | Approves it as written |
| `walk` | A walk window not yet approved | Approves it |
| `land` | A Job's pull request, once every deciding check has passed and nothing conflicts or already merges it | Asks the forge to merge when its checks pass, as Auto land does. A Job with Auto land on is left to it, and a commit pushed since is decided again |
| `unsure` | An issue the picker set aside | Starts it, as dispatching it from the pile does |
| `red` | A red main | Starts one Job, once for each red commit, with no call. The Job holds the red's claim before it is briefed, so a Fleet that holds it refuses this one |
| `reads` | A permission card for a tool that only reads | Allows it once |
| `writes` | A permission card for an edit, or for a run command Fleet can read whole | Allows it once |

**What always waits.** An item that may delete or overwrite something, read from its words and its options before the call and from the way the call chose after it.
A Job in a terminal, whose prompt is answered in the terminal.
A run command Fleet cannot read whole, and a card about a path past the Job's slots.
A question with no options, or one that asks the person to run or approve something.
A question asked in the walk, a start card, and every other item.

## What it keeps

Every answer is kept once, with the Job, what was asked, what was chosen, why, and when. An answer is not a call.
The person keeps one with Agree, or corrects it with Change, which sends the Job `Re: <asked>` and then their words.
A correction is read the next time, beside the answer it replaces.

## Rules

> **Rule.** Only a sure pick starts a Job with no person pressing.
> Why: the owner's call of 11 Oct 2026, `.claude/decisions/2026-10-11-autopilots-clear-picks-start-unasked.md`.
> A Job has no dispatch gate, so what the call decides now is whether a person is asked.
> In code it is one function that takes the sure pick whole, and only reading the picker's answer makes one.

> **Rule.** A decision is one fresh call, and no agent lives between them.
> Why: `docs/scope.md` records orchestrator agents as abandoned, because having a conversation was not the tool.
> The call reads the repository's earlier answers from the record, so a correction shapes the next answer without a conversation to carry it.

> **Rule.** Sure, or the item waits.
> Why: the call is told that where the ways differ in what they cost, or it cannot tell, it chooses none. A waiting item stays in Needs you.

> **Rule.** Destructive always waits.
> Why: the words are matched conservatively, so a word found inside another holds the question, and a held question costs the person a glance.

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

`list_autopilots`, `get_autopilot`, `propose_autopilot`, `start_autopilot`, `update_autopilot`, `pause_autopilot`, `resume_autopilot`, `stop_autopilot`, `dispatch_set_aside`, `dismiss_set_aside`, `get_autopilot_decisions`, `review_autopilot_decision`, `override_autopilot_decision`, and the events `autopilot.changed` and `autopilot.decisions_changed`.
`crates/ipc/operations/` has each, and `../practices/protocol.md` has the section.

## In Bridge

The switch is the only control, a guarded red switch on the Map's lip. It is on where any repository's Autopilot is running, whoever started it, so a cap that pauses one brings it down.
Bridge has no Pause, Resume or Edit caps: the caps are set at start, in the setup that comes up over the field when the guard lifts, with the repositories, what goes on autopilot, the instruction, the limits and who takes each kind of decision.
Where a repository's Autopilot stopped by itself, the setup says so beside Start: which cap paused it, or why its last pass picked nothing. A running one with nothing to say shows nothing.
Fleet refuses a start over a paused Autopilot, so Start and the switch stop a paused one first and then start it, as one press.
The dispatch panel offers Autopilot as a third kind: the repository is read from the words or asked, the panel's act reads the sentence into settings it draws for the person to edit, and Start starts it.
Under the setup, Next up lists what it is choosing from and Set aside what it was unsure of.
While one runs, the first key or press after ten minutes with no input asks whether to keep it on. Either answer opens what Autopilot answered meanwhile.
That list is a HUD over the map, opened from the lamp on the switch's plate, which is lit while an answer waits.
A row is the Job, what was asked, what Autopilot chose, why, and when.
Agree keeps it, Change sends the Job his own words, and an answer Fleet cannot correct, such as a landed pull request, offers Agree alone.
Each leaves the list.
`autopilot.changed` reaches every window through main, which keeps no copy.
The code is `apps/desktop/src/renderer/src/cockpit/deck/AutopilotArm.tsx` and `AutopilotAnswers.tsx`.

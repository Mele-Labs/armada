# Spike 22 — The wire lock for the new Job

**Status: signed off by the owner, 2 Oct 2026.**
Read against `main` at protocol 21.9 on 2 Oct 2026. The backend
slices at the foot of this page run wave to wave without asking again, until
one of them meets a question.

**Only a task's two new states and the Record's new signers break a peer.** A
task gains *handed in, waiting for Checks*, spelled `handed_in` on the wire, and
`failed`. A Judge and a Check signing in their own names is also a store change
with a migration. Every other field the boards draw is additive, and most of the
majors spike 020 priced are no longer needed.

**The bound #1545 asks for is built on `/events` and nowhere else needs one.**
`crates/api/src/stream.rs` is drop-oldest, and a drop on `/events` is a `missed`
message followed by a resync (`crates/api/src/sockets.rs`). The per-Job turn
socket sends `missed` with a count and no resync, and Bridge keeps the gap. The
Job log and a Check's output follow a file by byte offset
(`crates/api/src/journal.rs`, `crates/api/src/following.rs`), so they cannot
drop anything. What nobody has is a rate, so slice 0b measures one and answers
the turn socket's gap.

A line number never follows a path here, because the gate reads a path inside
backticks and a path with a line on it resolves to nothing. A file a slice
would create is named in plain text for the same reason.

## The owner's answers

Each question blocked the slice it names, and nothing else. The options he
weighed stay beside each answer, so the cost he took is on the page.

### 1. When a task says it is done

Implement has four tasks. T1's agent finished and handed its work in ten
minutes ago, T2's agent is working, and the step's Checks run once T4 is in.
You open the plan.

**Today almost nothing marks a task** (#1752): of eleven Drones told to mark
theirs, two did, so tasks read open from start to finish. The rule written
down for when Fleet marks them is *working when its agent starts, done when its
group's Checks come back green* (`docs/concepts/plan.md`). Under that rule T1 reads working while nobody is
working on it, and all four turn done together at the end.

- **A. Done when its agent hands in.** If the group's Checks then go red, every
  task in the group turns failed. Cost: done stops meaning the Checks passed.
- **B. Keep the rule: working until the group's Checks pass.** Cost: a finished
  task reads working, sometimes for an hour, and tasks flip together.
- **C. A word in between: handed in, waiting for Checks.** Done arrives at
  green. Cost: one more task word to learn, and it rides a major bump.

**Decided by the owner, 1 Oct 2026: C, a word in between: handed in, waiting for Checks.** Done arrives at
green. Neither A's nor B's sentence is true of T1, and the major bump rides with
`failed` and the signers in slice 1a. **Unblocks slices 1a and 1b.**

### 2. Who gets the next agent when the machine is full

The machine allows two agents. Job A is on task 3 of 6, and Job B is on task 1
of 4. You approve Job C. Then Job A's task 3 finishes.

**The machine's number is being recounted in agents rather than Jobs** (your
call of 22 Sep). What it does not say is whether a Job halfway through its plan
keeps its place between one task and the next.

- **A. Work already started goes first.** A's task 4 starts and C waits for a
  whole Job to finish. Cost: C may wait hours.
- **B. First come, per agent.** C asked before A's task 4 did, so C starts and A
  pauses mid-plan. Cost: a half-done Job sits holding its branch, and can starve.
- **C. Every running Job keeps one agent; extra ones wait their turn.** While
  tasks run one at a time this is A. Once tasks run at once, a Job's second
  agent yields to a Job waiting to start. Cost: a busy Job runs slower while
  others queue.

**Decided by the owner, 1 Oct 2026: C, every running Job keeps one agent; extra ones wait their turn.**
Nothing stalls mid-plan, and the extra speed is what gives way. **Unblocks slice
1b** (between tasks) **and slice 5** (the extra agents).

**The agent a Job keeps is never asked for again.** A Job holds its place from
admission to its end (`crates/fleet/src/slots.rs`), and a step's next Drone
already spawns without asking the cap, memory or disk. The next task's Drone
spawns the same way, so a machine short of memory still starts it: headroom
refuses admissions, and in slice 5 a Job's extra Drones, and nothing else.

### 3. When the Judge refuses a group

Group 2 has three tasks. All three handed in and the Checks passed. The Judge
then refuses: *T5's change does not do what its done-when said.*

**Today a red Check can send a step round again on its own**, where the workflow
allows retries. **A Judge refusal never does**: it stops the step and waits for
you. Groups make it possible to send just the group round.

- **A. A refusal stops the group and waits for you, as today.** You answer it,
  run one task again, or edit the task first. Cost: the Job sits until you look.
- **B. The group goes round once more on its own,** with the refusal in each
  agent's brief, and stops for you if refused again. Cost: a round of spend on
  a refusal that may be wrong.
- **C. Only the tasks the refusal names go round on their own.** Cost: a refusal
  naming the group re-runs everything, and the Judge has to learn to name tasks.

**Decided by the owner, 1 Oct 2026: A, a refusal stops the group and waits for him, as today.** A refusal is
the exception that meets a person, and **Restart this task** (#1656) makes the
retry one press. **Unblocks slice 2.**

### 4. Your override, when the repository's rule changes after you approve

Feature's merge step says *the repository decides*, and today the repository
says a person merges. At approval you override it: this Job lands on its own.
An hour later somebody edits `armada.yml` to stop every automatic merge before a
release.

**Today there is no override**, and the rule is read again at every gate, so a
change mid-Job moves the Job too. Since 21.9 (#1683, your call of 1 Oct) the
Record says what the rule said at each gate.

- **A. Your override wins for the life of the Job.** Cost: a repository-wide stop
  does not stop a Job you overrode; you stop it yourself.
- **B. The repository wins whenever it moves after you approved.** Cost: a file
  edit undoes your choice; the Record says so, and nothing else does.
- **C. Whichever is stricter wins.** Cost: an override can only add a person,
  never remove one, which narrows what you decided on 22 Sep.

**Decided by the owner, 1 Oct 2026: A, his override wins for the life of the Job.** The override is a
person's explicit choice about one Job, and the Record shows what every gate
read. **Unblocks slice 4.**

### 5. When Armada notices the issue a Job came from has changed

You dispatch against issue 812. Its checklist becomes what the Job is held to,
you correct one line, and you approve. Two hours in, somebody edits 812.

**The Job keeps the words it froze and says the issue has moved since** (your
call of 22 Sep). Today Fleet reads an issue's title and body once, when the
request arrives (`crates/adapters/src/issue_lookup.rs`), and never asks again.
It asks the forge about open pull requests on a rotation, and about nothing else.

- **A. On the rotation that watches pull requests.** One more forge call per
  running Job that came from an issue, and a line on the Record when it moves.
  Cost: forge calls, and their rate limit.
- **B. Only when you open the Job.** Cost: the Board never shows it, and a Job
  open on screen asks again every time it is redrawn.
- **C. At the two moments you decide: approval and review.** Cost: a change
  between those moments shows only at review.

**Decided by the owner, 1 Oct 2026: A, checked on the rotation that watches pull requests.** It is the only
one where the sentence is true whenever it is read. **Unblocks slice 4.**

**The rotation today reaches only Jobs with an open pull request.**
`Fleet::due_to_ask` (`crates/fleet/src/noticing.rs`) picks one Job per interval,
sixty seconds as shipped, from `pull_requests_unsettled()`. A running Job that
came from an issue has no pull request yet, so it is in no rotation at all, and
A holds only once slice 4 adds one.

| | What slice 4 builds, chosen here; stood at the owner's sign-off, 2 Oct 2026 |
|---|---|
| The set | Every running Job whose request came from an issue |
| The mechanism | A second cursor on the same interval, beside the pull-request one |
| Forge cost | One more issue read per interval, whatever the number of such Jobs |
| How fresh | Each such Job is asked once every interval times their number |
| Pull-request cadence | Unchanged: they keep their own cursor and their own call |

Sharing the one cursor was the alternative, at no added forge cost. It would
slow every pull request's reading by the number of issue-linked Jobs, and that
cadence is his call (`docs/concepts/fleet.md`), not this lock's.

### 6. Two agents in one copy, writing one file neither said it would touch

Group 3 runs T7 and T8 at once, in the Job's one copy of the repository. T7 said
it touches `auth.rs` and T8 said `session.rs`, but T8 also fixes an import in
`auth.rs`. The later save wins, and T7's change to that file is gone without a
trace.

**Tasks running at once share the Job's copy** (`docs/concepts/plan.md`), and
nothing catches this today. `docs/contracts/system-architecture.md` still says
one copy per agent, and #1530 holds that it has to be amended before this is
built.

- **A. Stop the agent the moment it writes outside its own files** while another
  task runs, and ask for the path the way a scope widening is asked now. Cost:
  agents are stopped mid-work more often.
- **B. Catch it at the group's end.** Where two tasks wrote one file, the group
  does not commit, and those tasks run again one after the other. Cost: both
  tasks' work is done twice.
- **C. A copy per agent, joined at the group's end.** Cost: disk — this machine
  once held 74 copies and 220 GB — and a merge conflict to resolve at every join.

**Decided by the owner, 1 Oct 2026: B, caught at the group's end.** Where two tasks wrote one file, the group
does not commit, and those tasks run again one after the other. The lost write
never lands, and it costs time only on the groups where it happened. **Unblocks
slice 5.**

**Who wrote a file is read from each Drone's file-edit tool calls**, by answer
10 below. Two tasks of one group whose Drones' edit calls named one path is the
overlap. A write made through the shell — `sed`, a formatter, a code generator —
names no path to Fleet and goes unseen, and the plan says so to the person.

### 7. Slice 1, split in two

**Decided by the owner, 1 Oct 2026: slice 1 splits.** 1a lands the three
breaking changes — the waiting word, `failed`, and a Judge and a Check signing
the Record — with the store migration, the new `TaskCounts` fields, and the
writers that sign as a Judge and as a Check. 1b lands one Drone per task, one
at a time. 1a's scope includes the files that write those rows:
`crates/fleet/src/gate.rs`, `crates/fleet/src/settling.rs` and
`crates/fleet/src/judging/`.

### 8. A tier on the Job's model map left empty

**Decided by the owner, 1 Oct 2026: an empty tier on the Job's model map means
Armada picks, and the Drone's row says which model it ran** — #1549's wording,
*Auto means Armada picks; say what it picked once it has*. It replaces this
page's earlier *null is the Job's own model*. On the wire an empty tier is a
key left out, never `null`, by `docs/practices/protocol.md`'s rule.

### 9. When a group's Checks go red

**Decided by the owner, 1 Oct 2026: a red group's Checks send it round on its
own where the workflow allows retries, as today.** Restart this task (#1656) and
Move apply once the retries run out. A failed task is the state after the
retries are exhausted. That is what `docs/concepts/plan.md` means by *the retry
that re-runs the group is what clears it*: the person's retry, once Fleet's own
are spent.

### 10. How Fleet knows which task wrote a file

**Decided by the owner, 1 Oct 2026: overlap in a shared copy (answer 6) is
attributed from each Drone's file-edit tool calls.** Shell writes — `sed`,
formatters, code generation — go unseen, and the plan says so to the person.
Slice 5 builds it.

## What is already decided, and where

**These are the questions #1545 and #1530 name, read against the record before
calling any open.** A question this page answers by choosing is marked
*chosen here*; each stood at the owner's sign-off, 2 Oct 2026. *Answer N* is
one of his answers above.

| Question | Answer | Where |
|---|---|---|
| Where task transitions ride | `job.plan_changed`, which gains the task and its new state | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| Where per-task turns ride | `/observe`, filtered by `drone_id` (21.4); a Drone row names its task. Live rows follow one Drone at a time until slice 5 | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| Plan on `get_job`, or `get_plan` | Stays on `get_job` | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| The waiting word on the wire | `handed_in` | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| How a task's Drone hands in | `submit_evidence`, unchanged; Fleet keeps it as the task's | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| Which step runs a Drone per task | A new step key on `implement`; not `follows_plan` | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| Who does a group's own round | One Drone for the group, as a step retry is one Drone today | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| A move's place | `after`, as `add_task` places one | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| Per-step gate ticks | Nothing ticked: nobody looks, Fleet's drift look stays | #1530, 22 Sep |
| The repository-decides state | A fourth state, overridable for this Job | #1530, 22 Sep |
| What a gate resolved to | Both policies recorded on every attempt; older reads absent | Landed at 21.9, #1683 |
| Which wins after an override | The override, for the life of the Job | Answer 4 |
| Difficulty tiers | A map on the Job, a tier on the task | #1530, 22 Sep |
| A tier left empty | Armada picks, and the Drone's row says what it ran | Answer 8 |
| A task's model | A person picks it directly, over the map | `.claude/decisions/2026-09-30-a-person-can-pick-a-tasks-model.md` |
| A model nobody holds | Refused, as `set_model` refuses one today | `crates/fleet/src/job_settings.rs` |
| A per-Job Drone cap | Yes, inside the machine's; stored in slice 4, enforced in slice 5 | #1550, 22 Sep |
| The machine's cap | Recounted in Drones | #1530, 22 Sep |
| Who goes first when full | Every running Job keeps one agent | Answer 2 |
| Criteria read from an issue | Editable to the press, frozen at approval, origin labelled | #1551, #1641 |
| Noticing the issue moved | A second cursor on the pull-request rotation's interval | Answer 5 |
| Overlap at dispatch | Taken out | `.claude/decisions/2026-09-23-take-out-what-else-is-running.md` |
| `CriterionView.cases` | No; *not covered* belongs to a case alone | `.claude/decisions/2026-09-22-no-verdict-recorded.md` |
| A Judge and a Check sign the Record | Stored, with a migration | `.claude/decisions/2026-09-22-judge-and-check-sign-the-record.md` |
| The bounded stream | Built on `/events`; slice 0b measures it and answers `/observe`'s gap | `crates/api/src/stream.rs` |
| A group's id | Minted by Fleet when the plan is recorded | Forced by #1685's move |
| A group's clock | Fleet stamps a group's start and end | Chosen here; stood at the owner's sign-off, 2 Oct 2026 |
| Classifying | Is `proposing`, on the wire since 19.0 | `.claude/decisions/2026-09-21-classifying-is-proposing.md` |
| A request that splits | One Job; the rest are its members | `.claude/decisions/2026-09-30-a-dispatched-request-is-a-job.md` |
| A member counts as landed | When its pull request merged | `.claude/decisions/2026-09-21-a-parent-job-holds-members.md` |
| One copy per agent, in the contract | Amended in slice 5; overlap caught at the group's end | Answer 6 |
| Who wrote a file | Each Drone's file-edit tool calls; shell writes unseen | Answer 10 |
| A Judge refusal at a group | Stops the group and waits for a person | Answer 3 |
| Red Checks at a group | Round on its own while retries last; `failed` after | Answer 9 |

### Why the plan stays on `get_job`

**An open Job already reads `get_job` again on every event that names it**
(`apps/desktop/src/main/arrivals.ts`, `job.plan_changed` included), and every
tab draws from that one read. The split precedent is size: `get_diff` and
`get_call` are split out because their payloads are large, a Check's output
alone running to two 64 KiB streams, and a plan is titles and paths. A second read would let Plan and Workflow draw two different
moments of one Job.

**The cost moves with the rate.** A `job.plan_changed` per task transition is a
`get_job` per transition on the Job a person has open, and slice 0b's tally is
what says whether that needs coalescing.

### Why a transition rides `job.plan_changed`

**It already fires on every kept change to a plan, and carries the counts a
Board row needs.** Adding the task's id and its new state, and later the group's,
lets a timeline move without a read, and costs two optional fields. The plan
itself still does not ride: titles are free text, and the channel is one
drop-oldest bound every Job shares.

### Which model a Drone runs, chosen here; stood at the owner's sign-off, 2 Oct 2026

A person's pick on the task, then the Job's map for the task's tier, then the
model the workflow step declares, then the Job's own. A later and more specific
choice wins, and every spawn records which one it ran as. **A tier the map
leaves empty is Armada picking** (answer 8): the order falls through to the
step's model and then the Job's, and the Drone's row names the model it ran.

### How a task's Drone hands in, chosen here; stood at the owner's sign-off, 2 Oct 2026

**The tool is `submit_evidence`, unchanged in shape.** A Drone is bound to its
Job by `crate::peer` already, and 1b binds it to its task at the spawn, so the
call needs no task argument and a Drone cannot name the wrong one. A second tool
was rejected on #1752's measurement: two of eleven Drones called `update_task`
when told to, and a Drone choosing between two hand-in tools is one more choice
it can get wrong.

| On a task's Drone, the call | |
|---|---|
| Records | The task's hand-in: `shown` from `shown_by`, the task to `handed_in` |
| Answers | `recorded`, as today |
| Does not | Put anything in the step's evidence inbox, so the gate's turn finds nothing to weigh |
| Then | The Drone ends, and the next open task's Drone is spawned |

**The step's gate fires once no task is open or working.** Fleet then puts one
submission in the step's inbox, carrying every task's claim, and the gate weighs
it as it weighs a step's submission today. 1b owns this; 1a only adds the word.

### Which step runs a Drone per task, chosen here; stood at the owner's sign-off, 2 Oct 2026

**A new step key, not `follows_plan`.** `follows_plan` grants `add_task` and
`update_task`, and it is already set where a Drone per task is not meant:
revert's one step that records the plan and works it, epic's, code-review's,
design-plan's, prototype's, and feature's `tests`. Reusing it would turn every
one of those into a Drone per task.

**Feature, bug and refactor set the new key on `implement`.** Feature's `tests`
step keeps one Drone and keeps `follows_plan`: its tests are written against the
whole change, and its gate is the whole set running again before the pull
request (#1530), placed under slice 2.

## The protocol changes, bundled

**The majors ride together, in slice 1a.** A major is the lifeboat for any
Fleet caught mid-Job, pre-alpha or not
(`.claude/decisions/2026-09-22-armada-is-pre-alpha.md`), so the milestone takes
one rather than several.

| Change | Bump | Why | Slice |
|---|---|---|---|
| `TaskState` gains `failed` | Major | Strict wire set; the task machine grew a state | 1a |
| `TaskState` gains `handed_in` | Major | Same set, same bump | 1a |
| `Actor` gains `judge` and `check` | Major | Strict set, stored on every recorded row | 1a |
| `TaskCounts` gains optional `handed_in` and `failed` | Minor, riding 1a's major | Optional fields, absent at zero | 1a |
| Every other row on this page | Minor | New DTO, route, event kind or optional field | Per row |

**Where the new counts sit in a person's figure.** `TaskCounts`
(`crates/ipc/src/work_plan.rs`), on `JobPlanChanged` and `JobSummary.tasks`, is
read as done over `done + working + open`. A `handed_in` task and a `failed` one
both join that total and neither joins `done`; `dropped` stays out of it.

**What spike 020 priced as major and the lock does without:**

| 020's major | Why it is not needed |
|---|---|
| `JobStatus` gains `classifying` | The status is `proposing`, already on the wire |
| `Actor::Human` renamed `person` | The wire keeps `human`; Bridge says *you* |
| `JobResources.worktree` becomes a list | Groups run in the Job's one copy; members are Jobs with their own |
| `Criterion.criterion_id` optional | Fleet mints an id at every edit before approval |
| `source` renamed `verified_by` | The wire keeps `source`; the draft renames on the way in |

**The store change is designed for an older Fleet, not around it.** Adding the
two signers is a migration in `crates/store/src/migrations.rs`, on top of V87
(`job_step_policies`, from #1683), which moves `KNOWN_SCHEMA_VERSION`. An older
Fleet then refuses to open the store, rather than folding a row it cannot spell
in `crates/store/src/fold.rs`.

### The bodies the lock agrees

**Bridge already sends a body to every route below** (`packages/protocol/src/pending.ts`).
Where it disagrees with its issue, the wire's own vocabulary decides, and Bridge
changes its body in the slice that ships the route.

| Route | Bridge sends | The issue says | The wire takes | Slice |
|---|---|---|---|---|
| `restart_task` (#1656) | Nothing | An optional note, `RestartRequested`'s shape | An optional `note`; nothing is valid | 2 |
| `move_plan` (#1685) | `group`, `task?`, `to: number` | By `after` | A task: `task`, `group`, `after?`. A group: `group`, `after?`. Absent `after` is first | 2 |
| `edit_task` (#1657) | `EditTask`, only the changed fields | Title, `note`, `scope`, `expects`, model | `EditTask` as sent | 3 |
| `edit_job` (#1699) | `EditJob`: `title?`, `brief?`, `expects?: string[]` | #1641's fields, shared | `title?`, `facts?`, `criteria?` in #1641's shape | 4 |
| `approve_wave` (#1694) | `ApproveWave`: `jobs` | One act, all or nothing | `ApproveWave` as sent; refused unless it names the wave Fleet holds | 6 |

**`after` wins over `to`.** `add_task` already places a task by `after`, and an
index counted after the move is stale the moment a Drone adds or drops a task
between the drag and the send.

**#1641's criteria win over a list of lines.** A line has no id, so every save
would mint new ids and drop each criterion's `source` and its issue origin
(#1642). `brief` becomes `facts`, the name `JobDetail` already serves; a line a
person adds carries no id and the `judge` source, as #1641 says.

## The drafts, one module at a time

Every exported type in `packages/screens/src/draft/`. *None* in the bump column
is a type served today or one that never crosses.

### `coord.ts`, `task.ts`

| Type | Lands on | Change | Bump | Source of truth | Slice |
|---|---|---|---|---|---|
| `RunCoord` | `CheckRun`, `Recorded` | Optional `group`, `group_attempt`, `task` | Minor | Fleet's group record | 2 |
| `TaskState` | `TaskState` | Adds `handed_in` and `failed` | Major | `crates/core-model/src/job/work_plan.rs` | 1a |
| `TaskTier` | `PlanTask.tier` | New, optional | Minor | The planner's `record_plan` | 3 |
| `TaskTreatment` | Nothing | Every task gets its own Drone | None | — | — |
| `TaskView` | `PlanTask` | Field by field below | Minor | `WorkPlan` in core-model | 1b–5 |

| `TaskView` field | On the wire | Slice |
|---|---|---|
| `id`, `title`, `note`, `scope`, `expects`, `state`, `reason` | Served today | — |
| `shown` | Served; Fleet fills it from the task Drone's hand-in | 1b |
| `drone_id`, `turns`, `cost_micros` | From the `JobDrone` naming the task | 1b |
| `group`, `failed_reason`, `touched_after_done` | New on `PlanTask` | 2 |
| `tier`, `model` | New on `PlanTask`; `model` is a person's pick | 3 |
| `concurrent_with` | New on `PlanTask`, declared by the planner | 5 |
| `cases` | Not this milestone; `COVERS` is parked (#1274) | — |
| `coord`, `treatment` | Derived in Bridge; not sent | — |

### `group.ts`, `drone.ts`

| Type | Lands on | Change | Bump | Source of truth | Slice |
|---|---|---|---|---|---|
| `GroupState` | A group's `state` | Registry row exists; Fleet starts writing it | Minor | Fleet's group machine | 2 |
| `GroupView` | `WorkPlan.groups` | New DTO: id, tasks, state, start, end, attempts | Minor | Minted at `record_plan` | 2 |
| `DroneState` | `JobDrone.state` | Served at 21.3 | None | Job history | — |
| `DroneView` | `JobDrone` | Adds `task`, then `model` | Minor | History and spend rows | 1b, 3 |
| `DroneThought` | `Saw` | Served at 21.4 as `thinking`; the sidecar can go | None | The transcript | — |

`GroupView.concurrent` lands in slice 5. `checks_selected` reads the `CheckRun`
rows that carry the group, and `cases_at_boundary` waits on #1274 with `cases`.

### `criterion.ts`, `dispatch.ts`, `landing.ts`, `branches.ts`

| Type | Lands on | Change | Bump | Source of truth | Slice |
|---|---|---|---|---|---|
| `VerifiedBy` | `Criterion.source` | Kept as `source` | None | — | — |
| `CriterionOrigin` | `Criterion.origin` | New: issue with ref and url, prompt, or person | Minor | Proposer, then the approval body | 4 |
| `CriterionView` | `Criterion` | Adds `origin`, `origin_moved_at`; id stays required | Minor | `crates/core-model/src/job/fields.rs` | 4 |
| `OriginSaid` | Nothing | Bridge's sentence | None | — | — |
| `LandsWhen` | `JobRequest` settings | New, optional | Minor | The dispatch form | 4 |
| `DispatchSettingsView` | `JobRequest` settings | Workflow, tiers, cap, lands, all optional | Minor | The dispatch form | 4 |
| `LandingUnit`, `PrMode`, `CompleteWhen` | A Job's landing | New closed sets | Minor | The Job's frozen settings | 4 |
| `LandingRule` | A Job's landing on `JobDetail` | New DTO, frozen at approval; `land_together` excluded | Minor | Manifest base, then the person | 4 |
| `BranchView`, `BranchesAnswer` | A new branch-list read | New route | Minor | git, through the Vcs adapter | 4 |

**Which `CompleteWhen` values Fleet honours** follows `COMPLETE_WHEN_SERVED` in
`packages/surfaces/jobs/src/draft/landing.ts`. `delivered` is honoured from slice 4,
and `all_members_landed` from slice 6. `pr_merged` stays unserved by #1532's
decision, `pr_opened` has no writer that tells it from `delivered`, and the
approval body refuses both.

**`land_together` does not cross in this lock.** Where it lives is open, as
`[landing-where-land-together-lives]` in `docs/OPEN.md`. A per-group pull
request needs slice 2's groups.

**A `from_ref` other than `target` is new worktree behaviour.** Today a worktree
is cut from the Manifest's base and lands back in it. Slice 4 cuts the worktree
from `from_ref` and opens the pull request against `target`.

### `ledger.ts`, `members.ts`, `wave.ts`, `peers.ts`

| Type | Lands on | Change | Bump | Source of truth | Slice |
|---|---|---|---|---|---|
| `LedgerActor` | `Actor` | Adds `judge`, `check`; no `contributor` yet | Major | `crates/core-model/src/envelope.rs` | 1a |
| `LedgerRow` | `Recorded` | Optional coordinate fields; `kind` stays opaque | Minor | `job_events` | 2 |
| `LedgerFamily`, `LedgerReads` | Nothing | Bridge's composition | None | — | — |
| `MemberLink` | Nothing | Only `merged` is built; Bridge derives it | None | — | — |
| `MemberView` | `JobSummary`, `JobDelivery` | Adds `merged_at`, read off the forge | Minor | The pull request | 6 |
| `JobMembersView` | Nothing | Bridge's composition | None | — | — |
| `WaveJobView` | `DispatchOrigin`, `JobDetail` | Pass stamp, a new stored column; `waits_on` is `dependencies` | Minor | The child's own record | 6 |
| `WaveRoundView` | Per-pass line | New | Minor | The parent's plan, per pass | 6 |
| `WaveView` | Nothing | Bridge's composition | None | — | — |
| `PeerView`, `PeerOverlapView`, `PeerOverlapAnswer` | `ScopeOverlap` | Served; the dispatch half was taken out | None | — | — |

`contributor` needs the store between Armada instances that #1530 files apart,
and `dropped` needs a forge act that closes a pull request, which nothing has.
`WaveView.judged_by` is not in this milestone.

### `pulse.ts`, `proposal.ts`, `revision.ts`

| Type | Lands on | Change | Bump | Source of truth | Slice |
|---|---|---|---|---|---|
| `PulseProcess` | `JobProcess` | `drone_id` in place of the draft's branch owner | Minor | The process tree | 5 |
| `PulseWorktree` | `WorktreeOnDisk` | Served | None | — | — |
| `PulseLog` | `LogFile` | Served at 21.2; `writing` is `being_written` | None | — | — |
| `PulseView` | `JobResources` | One copy per Job; a parent reads its members'. *Drones running* counts every live Drone | Minor | The process tree | 5 |
| `RepositoryDecides`, `RepositorySays` | `advance_gate`, `ManifestSummary` | Served | None | — | — |
| `GateView` | Per-step gates on the Job | New, frozen at approval | Minor | The approval body | 4 |
| `TierModels` | A Job's tiers | New; a tier left out is Armada picking | Minor | Dispatch, then approval | 3 |
| `ProposalView` | `JobDetail`, the approval body | Adds `approved_at`; the rest per row | Minor | The Job at `proposing` | 4 |
| `GateDeclared`, `GateReading`, `Declared` | Nothing | Bridge's reading | None | — | — |
| `PlanAskKind`, `PlanAnswer`, `PlanRefusal`, `PlanRevisionView` | Nothing | Moves are direct now; a rewrite is a redirect | None | — | — |

`ProposalView.machine_cap` is `LimitValues.concurrency`, whose meaning becomes
Drones in slice 5 with no change of shape. `fleet_always_looks` is a constant
and never crosses.

### `cases.ts`, `sketch.ts`, `sketch-png.ts`, `words.ts`, `held.ts`

| Type | Lands on | Change | Bump | Slice |
|---|---|---|---|---|
| `CaseState`, `DroppedBy`, `CaseView` | Nothing this milestone | `NamedSpec` and `ShownSet` stay the source | None | — |
| `CaseRunActor`, `CaseRunPurpose`, `CaseRunTree`, `CaseRunOutcome`, `CaseRunView` | Nothing this milestone | Waits on #1274 | None | — |
| `ScopeRevisionView` | Nothing this milestone | — | None | — |
| `SketchShape`, `SketchJoin`, `SketchPoint`, `SketchStroke`, `SketchPicture`, `Drawing` | A Studio's Sketch | Served at 20.0 | None | — |
| `SketchAttachment` | `AttachmentRef` | The PNG is served; `drawn` never crosses | None | — |
| `SketchOpening`, `SketchInk`, `SketchPng` | Nothing | Bridge's own | None | — |
| `DraftWord` | `crates/core-model/domain/enum-verbs.toml` | Every task-state row, `handed_in` included; the `classifying` word goes | Registry | 1a |
| `JobDraft` | Nothing | The carrier; a field goes as each lands | None | — |

**Every task state owes a registry row**, the served ones included, which 020
found and is still true; they land together in 1a as a `task_state`
vocabulary. The criterion's *no verdict recorded* owes a row of its own, in
slice 4. A Sketch's provenance on an attachment (`produced_by`, `said`) is not
in this milestone.

## What spike 020 said that is no longer true

| 020 said | Today |
|---|---|
| No Drone id on a turn; thinking dropped | Both served at 21.4 |
| An exited Drone is lost | `list_job_drones` keeps it, 21.3 |
| Nothing measures a log | Size and *being written* served at 21.2 |
| The wave's order is thrown away | `JobDetail.dependencies` already served the edges; the pass that made each child is #1692, slice 6 |
| Overlap at dispatch is an open question | The panel was taken out, 23 Sep |
| The machine cap's unit is open | Recounted in Drones, 22 Sep |
| A sketch's boxes are an open question | A Studio's Sketch is the pad, 20.0 |
| A model per tier is free text | Free text, refused unless `list_models` offers it |

**These draft comments are now wrong**, each left for the slice that moves its
type to the wire.

| Draft | Says | True under the lock | Slice |
|---|---|---|---|
| `packages/surfaces/jobs/src/draft/task.ts`, `TaskState` | `failed` is a task whose own agent stopped without finishing | A task whose group's Checks were still red when the workflow's retries ran out; an agent that stops without handing in stops the step | 1a |
| `packages/surfaces/jobs/src/draft/task.ts`, `tier` | The planner picks the tier and the model follows | A person may pick the task's model directly, over the map (30 Sep) | 3 |
| `packages/surfaces/jobs/src/draft/task.ts`, `model` | The model the Job's tier map resolved | A person's pick; the model a Drone ran is on its `JobDrone` row | 3 |
| `packages/surfaces/jobs/src/draft/proposal.ts`, `TierModels` | `null` is Auto, the harness chooses | A tier left out means Armada picks, and the Drone's row says which model it ran | 3 |

## The issues filed since

| Issue | What it needs from the wire | Slice |
|---|---|---|
| #1652 | A group coordinate on every `CheckRun` | 2 |
| #1656 | A route restarting one task, holding the group's Checks | 2 |
| #1657 | A route editing a task, model included; `PlanTask.model` | 3 |
| #1666 | Redirect and kill taking a Drone id | 5 |
| #1685 | A route moving a task or a group | 2 |
| #1694 | Children at `awaiting_approval` at the split; one approve for the wave | 6 |
| #1699 | A route editing a Job before approval | 4 builds it, 6 uses it |
| #1752 | Fleet writes task state, and fills `shown` | 1b |
| #1641 | One approval body carrying the whole proposal | 4 |
| #1642 | An issue's address on a criterion's origin | 4 |
| #1651 | The header's Drone kill moves to the task's Drone | 5 |
| #1605 | A read listing a repository's branches | 4 |
| #1648 | A process's owner, and *Drones running* above one | 5; the rest is left out below |
| #250 | Piloting one task | Left out below |

**#1752's latest comment decides 1b's shape.** Drones were offered
`update_task` and told to call it in `crates/fleet/src/crossing.rs`, and across
this machine two of eleven did. So Fleet writes a task's state, and fills `shown`
from the hand-in the task's own Drone already makes, or `the_evidence_accounts_for_itself`
refuses every task with an `expects`.

## The event stream

**This lock makes the risk worse, by a stated amount, and slice 0b exists to
put a number on it.** 1b adds a `job.plan_changed` per task transition and a
spawned and exited pair per task, against one Drone pair per step today. Slice
5 multiplies that by how many tasks run at once.

| Stream | What the lock adds | Bound, and what a drop does |
|---|---|---|
| `/events` | Task and group transitions, Drone lifecycle per task | `BACKLOG`, drop-oldest; `missed`, then a resync |
| `/jobs/:job_id/observe` | Rows from several Drones of one Job, from slice 5 | `WATCHING`, drop-oldest; `missed` with a count, no resync |
| A transcript's file queue | Unchanged, one per Drone | Drops a row and writes `missed` into the file |
| The Job log, a Check's output | Nothing | Read from the file by byte offset; nothing drops |

**Transcript rows never reach `/events`**, which is the decision that keeps the
Board's channel cheap. Several Drones share one per-Job socket rather than a
socket each, for 21.4's reason: one order on one clock.

**Today the live half of that socket follows one Drone at a time.** Only the
backfill interleaves every Drone the Job has had. `Turns::feeding`
(`crates/api/src/observing.rs`) gives each Drone a channel of its own and moves
the Job's slot onto it with `send_replace`. A viewer already watching stays on
the old Drone's channel until it closes, and one arriving later hears only the
newest, so two Drones at once would each be heard by half the viewers. Slice 5
reworks this; it is also what keeps a Drone's peek a live tail
(`.claude/decisions/2026-09-29-a-drone-peek-is-a-live-tail.md`).

**A gap between task Drones can read as the Job going quiet.** A viewer waits
`HANDOVER`, a quarter of a second, for the next Drone, then is told
`drone_ended`. In 1b no gate runs between tasks, so the gap is the exit and the
next spawn, as at a step boundary with no gate. Where it runs longer, Bridge
reopens the socket on `drone.spawned` (`apps/desktop/src/main/screen.ts`), and
slice 5's channel removes the gap.

## The backend milestone

**Claim: a Job's plan is worked by a Drone per task, group by group, and I can
see and act on each task, each group and each Drone.** One acceptance test,
written before slice 1a and added to slice by slice, in a new file beside
`crates/acceptance/tests/drone_per_step.rs`.

```
1a ──► 1b ──► 2 ──► 3 ──► 4 ──► 6
                    │
                    ▼
            0b ───► 5
7: a design first, and nothing waits on it
```

**One ordering departs from #1545's list, because the code argues for it.** 0b
runs beside slices 1a and 1b rather than before them: the bound and its resync
are built, and 1b runs one Drone per Job at a time, so its events arrive in
sequence rather than at once. The rate multiplies at slice 5, which 0b must
precede.

**A second departure: `task_brief` moves into 1b.** A Drone cannot be put on
a task without being told which task, so slice 3 keeps only the tier and the
edit.

**Slice 5 waits on slice 3 as well as 2.** Both spawn Drones in
`crates/fleet/src/spawning.rs` — slice 3 to resolve each one's model, slice 5
to run several — and both add commands to `crates/fleet/src/commanding.rs`.

### What every slice that ships wire also writes

| Path | Why |
|---|---|
| `protocol-version.toml` | The bump |
| `packages/protocol/src/`, `packages/protocol/src/generated/protocol-version.ts` included | The hand-mirrored types, and the regenerated version |
| `docs/practices/protocol.md` | The version's own section |
| `crates/ipc/operations.toml` | A new operation or event kind |
| `crates/api/src/routes/served.rs`, `crates/api/src/daemon/commands.rs`, `crates/api/src/tests/fake/commands.rs` | A route, its `Commands` method, and the test fake |

**`crates/api/src/routes/served.rs` is one line under the size the gate
refuses** (`xtask/src/rules.rs`). Slice 2 is the first to add a route, so slice 2
splits it. `crates/fleet/src/commanding.rs` is under the same rule with less
room, and the slice its additions would carry over splits it by subject.

**Slices 4 and 5 can run at once, and collide on two things.** Each adds
methods to `crates/fleet/src/commanding.rs` and each moves the minor. Whichever
lands second takes the next minor at its merge, as 21.9 was taken after 21.8
landed first, and keeps its commands in methods of its own.

### 0b — The stream, measured

**Claim: after a Job runs a Drone per task, I can read how many events of each kind Fleet published each minute, and when my Bridge falls behind a Job's turns the pane is redrawn whole rather than left with a gap.**

Both halves fail today. No tally exists, and the events a Board needs most often
are not recorded anywhere to count afterwards: `job.files_changed` and
`proposal.moved` are broadcast and never stored (the V15 comment in
`crates/store/src/schema.rs`). On `/observe` a `missed` only adds to a count
(`packages/protocol/src/folding.ts`).

| | |
|---|---|
| Issues | #1759 |
| Builds | A per-kind tally of what the broadcaster published, written to Fleet's log each minute; on `/observe`, a `missed` reopens the socket so the backfill redraws the pane |
| Measures | Events per Job-minute by kind, from live runs on this machine, against `BACKLOG` |
| Already built | `a_client_that_cannot_keep_up_is_told_and_resynced` in `crates/api/src/tests/stream.rs` covers `/events`' drop and resync |
| Write scope | `crates/api/src/stream.rs`, `packages/protocol/src/folding.ts`, `apps/desktop/src/main/observe.ts` |
| Blocks | 5 |
| Waits on | Nothing |

**Reopening is the turn socket's answer, chosen here; stood at the owner's sign-off, 2 Oct 2026.** It is `/events`' resync
one socket over: the history is the transcript file, so a redraw costs one
bounded backfill and `skipped` says what lies beyond it. The number answers
`[broadcast-capacity]` in `docs/practices/protocol.md`, which is a person's to
close.

### 1a — The breaking changes, and the Record's new signers

**Claim: when a Judge refuses or a Check fails, the Record names that Judge or that Check as the one who signed, not Fleet; and a Fleet built before this refuses the store rather than misreading it.**

| | |
|---|---|
| Issues | #1760 |
| Wire | Major: `handed_in` and `failed` on a task, `judge` and `check` on `Actor`; `TaskCounts` gains optional `handed_in` and `failed` |
| Blocks | 1b |
| Waits on | Nothing |

- `TaskState` gains `handed_in` and `failed`. The `task_state` vocabulary lands
  in `crates/core-model/domain/enum-verbs.toml` with every row, from
  `packages/surfaces/jobs/src/draft/words.ts`, and the draft's `classifying` word goes.
- `Actor` gains `judge` and `check`. The rows a Judge's verdict and a Check's
  outcome write are signed by them, in `crates/fleet/src/gate.rs`,
  `crates/fleet/src/settling.rs` and `crates/fleet/src/judging/`.
- The migration sits on top of V87 and moves `KNOWN_SCHEMA_VERSION`.
- `TaskCounts` gains its two fields, placed as the bundle above says.
- **Nothing writes `handed_in` or `failed` yet.** 1b writes `handed_in`, and
  slice 2 writes `failed`.

Write scope: `crates/core-model/src/job/work_plan.rs`, `crates/core-model/src/envelope.rs`,
`crates/core-model/domain/enum-verbs.toml`, `xtask/src/rules_enums.rs`,
`crates/ipc/src/work_plan.rs`, `crates/ipc/src/enums.rs`, `crates/fleet/src/gate.rs`,
`crates/fleet/src/settling.rs`, `crates/fleet/src/judging/`, `crates/store/src/migrations.rs`,
`crates/store/src/fold.rs`, `packages/surfaces/jobs/src/draft/words.ts`,
`packages/surfaces/jobs/src/draft/task.ts`, and the wire set above.

### 1b — Task by task, each by its own Drone

**Claim: a Job's plan is worked one task at a time, each by a Drone of its own, and the plan tells me which task is being worked, which are done, and what showed each one done.**

| | |
|---|---|
| Issues | #1762, carrying #1752 |
| Wire | Minor: `JobDrone.task`; `job.plan_changed` gains the task and its state |
| Blocks | 2 |
| Waits on | 1a |

- The new step key says a step works its tasks one Drone each; feature, bug and
  refactor set it on `implement`, as chosen above. Bridge reads the key where
  it now derives it.
- Each task's Drone gets a brief naming its task, spawned in plan order. The
  Job keeps its one agent between tasks, so the next spawn asks neither the cap
  nor headroom (answer 2).
- The hand-in is `submit_evidence`, kept as the task's, as chosen above.
- Fleet writes `working` at the spawn and `handed_in` at the hand-in, with
  `shown` from it, then `done` when the step's Checks pass. `JobDrone` gains
  `task`; `job.plan_changed` gains the task and its state.
- The step's Checks and Judge run once, at the step's end, as one group. A Drone
  that exits without handing in stops the step, as today.

Write scope: `crates/fleet/src/spawning.rs`, `crates/fleet/src/briefing.rs`,
`crates/fleet/src/crossing.rs`, `crates/fleet/src/work_plan.rs`,
`crates/fleet/src/drones_had.rs`, `crates/fleet/src/evidence.rs`,
`crates/core-model/src/job/work_plan.rs`, `crates/core-model/src/job/workflow.rs`,
`crates/config/src/workflow/step.rs`, `crates/core-model/domain/workflowdef-fields.toml`,
`.armada/workflows/`, `crates/ipc/src/work_plan.rs`, `crates/ipc/src/drones.rs`,
`docs/concepts/plan.md`, `docs/contracts/agent-prompt.md`, and the wire set above.

**Waiting on it in Bridge:** Plan's task sheet already draws what a task's own
Drone is doing now and the last file it wrote, mock-fed from the draft and drawn
for no task on today's Fleet (`packages/surfaces/jobs/src/task-live.ts`). This slice
is what turns them on.

### 2 — Groups, and a task that failed

**Claim: when a group's Checks go red, it goes round again on its own, with no press of mine, as many times as the step's retries allow; only when the last round is still red do its tasks read failed, with the group and the run named on the Record, and only then can I restart one task or move it.**

The test runs a step allowing two retries against a Check that stays red. It
fails if the group stops for a person after the first red, if a task reads
`failed` before the last retry, or if Restart this task answers before the
retries run out.

| | |
|---|---|
| Issues | #1763, carrying #1652, #1656, #1685 |
| Wire | Minor; the majors landed in 1a |
| Blocks | 3 |
| Waits on | 1b |

- The planner records groups; Fleet mints each group's id and stamps its start,
  its end and each attempt's verdict and commit. A group commits once.
- **At each group's end the step's gate Checks run, then its Judge.** A Check
  marked `runs_at: handoff` waits for the last group of the step before handoff,
  as `held_for_handoff` already says.
- **A red group goes round on its own while the step's `retry_limit` lasts**
  (answer 9). The round is one Drone, briefed with the red Checks and the
  group's tasks, as a step retry is one Drone today, and the tasks stay
  `handed_in`. When the retries run out, every task in the group turns `failed`
  with its reason.
- A Judge refusal stops the group and waits for a person; it never sends the
  group round on its own (answer 3).
- `CheckRun` and `Recorded` carry the coordinate. Fleet writes the flag a later
  task's write sets.
- Restart one task and move a task or a group, as routes, with the bodies
  agreed above. Restart is refused on a task that is not `failed`.

**The test decisions of 22 Sep, placed:**

| Decision | Here |
|---|---|
| `.claude/decisions/2026-09-22-tests-follow-both-lists.md` | Deferred with #1274: no group selects tests by file yet |
| `.claude/decisions/2026-09-22-a-silent-drop-shows-on-the-retry.md` | Deferred with #1274: a dropped test is a case |
| `.claude/decisions/2026-09-22-a-test-runs-at-its-last-group.md` | Deferred with #1274: every group runs every gate Check over the whole copy |
| #1530's *the whole set runs again before the pull request is offered* | Placed: the gate of the step before handoff runs every Manifest Check after its last group — feature's `tests`, and bug's and refactor's `implement` |

Write scope: `crates/fleet/src/gate.rs`, `crates/fleet/src/checking.rs`,
`crates/fleet/src/settling.rs`, `crates/fleet/src/work_plan.rs`,
`crates/fleet/src/commanding.rs`, `crates/core-model/src/job/work_plan.rs`,
`crates/store/src/migrations.rs`, `crates/store/src/fold.rs`, `crates/ipc/src/enums.rs`,
`crates/ipc/src/checks.rs`, `crates/ipc/src/history.rs`, `crates/ipc/src/work_plan.rs`,
`packages/protocol/src/pending.ts`, the split of `crates/api/src/routes/served.rs`,
and the wire set above.

### 3 — A model per task

**Claim: a hard task runs on the strong model and an easy one on the cheap one, a task I gave a model of my own runs on that one, and each Drone tells me which it ran.**

| | |
|---|---|
| Issues | #1764, carrying #1657 |
| Wire | Minor |
| Blocks | 4, 5 |
| Waits on | Slice 2 |

- A Job carries a tier map, and the planner gives each task a tier. A tier the
  map leaves out is Armada picking (answer 8).
- Edit a task's title, brief, files, done-when and model, before the plan's gate
  and after a failure, recorded as a person's change. A model `list_models` does
  not offer is refused.
- Each spawn resolves the model in the order chosen above, and `JobDrone` says
  which it ran.
- The draft comments on tier and model above are corrected.

Write scope: `crates/fleet/src/job_settings.rs`, `crates/fleet/src/spawning.rs`,
`crates/fleet/src/commanding.rs`, `crates/core-model/src/job/work_plan.rs`,
`crates/ipc/src/work_plan.rs`, `crates/ipc/src/drones.rs`, `crates/ipc/src/detail.rs`,
`packages/protocol/src/pending.ts`, `packages/surfaces/jobs/src/draft/task.ts`,
`packages/surfaces/jobs/src/draft/proposal.ts`, and the wire set above.

### 4 — Deciding what a Job will be, and how it lands

**Claim: everything I change on a proposal — its words, workflow, gates, criteria, tiers, Drone cap and how it lands — is what the Job runs after I approve it, and a criterion read from an issue says so and says when that issue moved.**

| | |
|---|---|
| Issues | #1765, carrying #1641, #1642, #1605, #1581, #1699's route |
| Wire | Minor |
| Blocks | 6 |
| Waits on | Slices 2 and 3 |

- One approval body carries the whole proposal (#1641). An edit that saves without
  releasing shares its fields, and is #1699's route, with the body agreed above.
- Criteria freeze at approval, carry their origin and the issue's address, and say
  when the issue moved. The three documents #1581 names move in the same change.
- Per-step gates with the override, which holds for the life of the Job however
  the repository's rule moves. The landing settings, with the `CompleteWhen`
  values and the `from_ref` worktree above; the branch list; the dispatch
  settings; `approved_at`.
- **The per-Job Drone cap is stored here and enforced in slice 5.** Until then
  a Job runs one Drone at a time, which any cap allows.
- A second cursor on the pull-request rotation's interval asks whether a linked
  issue moved, as answer 5 sets out.

Write scope: `crates/fleet/src/proposing.rs`, `crates/fleet/src/commanding.rs`,
`crates/fleet/src/policy.rs`, `crates/fleet/src/noticing.rs`, `crates/fleet/src/delivery.rs`,
`crates/adapters/src/worktree.rs`, `crates/core-model/src/job/fields.rs`,
`crates/core-model/src/job/declared.rs`, `crates/ipc/src/job.rs`, `crates/ipc/src/detail.rs`,
`crates/adapters/src/issue_lookup.rs`, `packages/protocol/src/pending.ts`, the three
documents, and the wire set above.

### 5 — Several Drones at once

**Claim: tasks the planner marked as safe together run at once, within this Job's cap and the machine's; I can watch, message or stop any one of their Drones without touching the others; and where two of them edited one file, the group does not commit.**

| | |
|---|---|
| Issues | #1766, carrying #1666, #1651, #1648's remaining half |
| Wire | Minor |
| Blocks | Nothing in this milestone |
| Waits on | Slices 0b and 3 |

- The planner declares which tasks may run together, as `concurrent_with` on
  each task. The machine's cap counts Drones, and the Job's cap from slice 4
  sits inside it. A Job's extra Drone asks the cap, memory and its repository's
  disk before it spawns, waits where refused, and yields to a Job waiting to
  start; the Drone the Job keeps goes on.
- **One channel per Job carries every live Drone's rows.** It is made at
  admission and dropped at the Job's end, every Drone's feed sends into it, and
  `HANDOVER` goes. A viewer hears every Drone from whenever it joins.
- Redirect and kill take a Drone id and refuse one that is not live. A process
  names its Drone, *Drones running* counts every live one, and the header's
  Drone kill goes.
- **Overlap is read from each Drone's file-edit tool calls** (answer 10). At the
  group's join, two tasks whose Drones' edit calls named one path means the group
  does not commit, and those tasks run again one after the other. A shell write
  names no path and goes unseen, and the plan tells the person so.
- `docs/contracts/system-architecture.md` is amended in the same change.

Write scope: `crates/fleet/src/slots.rs`, `crates/fleet/src/admitting.rs`,
`crates/fleet/src/resources.rs`, `crates/fleet/src/commanding.rs`,
`crates/fleet/src/spawning.rs`, `crates/fleet/src/ending.rs`, `crates/api/src/observing.rs`,
`crates/core-model/src/job/work_plan.rs`, `crates/ipc/src/work_plan.rs`,
`crates/ipc/src/mcp/planning.rs`, `crates/ipc/src/resources.rs`, `crates/ipc/src/limits.rs`,
`crates/config/settings.toml`, `docs/contracts/system-architecture.md`, a new module
reading a group's edit calls from its Drones' transcripts, and the wire set above.

**Waiting on it in Bridge:** Plan's task sheet already offers Hold to stop this
task, on a task with a Drone of its own. It is mock-fed and mocked — the press
ends the Job's Drone, as the Drones sheet's kill does — and #1666 is what makes
it end that task's Drone alone.

### 6 — Jobs under a Job

**Claim: an Epic's next wave is real Jobs I can read and correct before I approve it, one press starts them all, and the parent finishes when every member's pull request has merged.**

| | |
|---|---|
| Issues | #1767, carrying #1694, #1699, #1692 |
| Wire | Minor |
| Blocks | Nothing |
| Waits on | Slice 4 |

- An Epic's split creates its children at `awaiting_approval`, with their waits-on
  edges and the pass that made them. One act releases the wave, all or nothing.
- **The pass is a store change.** `DispatchOrigin` is read from two columns on
  the Job (`crates/store/src/read.rs`), and the pass needs a third, by a
  migration.
- A merged pull request records when it merged, read off the forge. A parent
  completes when every member has landed.

Write scope: `crates/fleet/src/sub_dispatch.rs`, `crates/fleet/src/noticing.rs`,
`crates/fleet/src/commanding.rs`, `crates/core-model/src/job/fields.rs`,
`crates/store/src/migrations.rs`, `crates/store/src/read.rs`, `crates/ipc/src/job.rs`,
`crates/ipc/src/detail.rs`, and the wire set above.

### 7 — Workflow profiles

**No claim yet**: what a profile is has no design.

| | |
|---|---|
| Issues | #1768, a design pass first |
| Blocks | Nothing |
| Waits on | A design |

The three-phase header waits on it (#1530, 21 Sep), and no field on this page
does.

## What this lock leaves out

| Left out | Why |
|---|---|
| Cases and their runs | `COVERS` is parked (#1274) |
| A `contributor` signer | Needs a store shared between Armada instances |
| A task promoted to a Job | Sub-dispatch refuses depth two |
| Stacked members, and dropping one | No forge act closes a pull request or stacks a branch |
| Land together | Where it lives is open, `[landing-where-land-together-lives]` |
| Piloting one task (#250) | A person in a group's shared copy beside running Drones has no design; the pending entry stays |
| A Judge brief listed before it is written (#1648) | Nothing knows a brief's path until the Judge is asked |
| A sketch's provenance on an attachment | Nothing reads it yet |
| Who judged a wave | Nothing records the model on a verdict |

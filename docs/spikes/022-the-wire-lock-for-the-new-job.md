# Spike 22 — The wire lock for the new Job

**Status: questions answered 1 Oct 2026; awaiting the owner's read and sign-off.**
Read against `main` at protocol 21.5 on 1 Oct 2026. Once signed, the backend
slices at the foot of this page run wave to wave without asking again, until
one of them meets a question.

**Only a task's two new states and the Record's new signers break a peer.** A
task gains *handed in, waiting for Checks* and `failed`. A Judge
and a Check signing in their own names is also a store change with a migration.
Every other field the boards draw is additive, and most of the majors spike 020
priced are no longer needed.

**The bounded stream #1545 asks for is already built.** `crates/api/src/stream.rs`
is drop-oldest, and a drop is a `missed` message followed by a resync, on
`/events` and on every per-Job socket. What nobody has is the number, so slice
0b measures rather than builds.

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
`failed` and the signers in slice 1. **Unblocks slice 1.**

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
1** (between tasks) **and slice 5** (the extra agents).

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
change mid-Job moves the Job too. From #1683 (your call of 1 Oct), the Record
will say what the rule said at each gate.

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

## What is already decided, and where

**These are the questions #1545 and #1530 name, read against the record before
calling any open.** A question this page answers by choosing is marked
*chosen here*; the owner may overturn any of them at sign-off. *Question N* is
one of his answers above.

| Question | Answer | Where |
|---|---|---|
| Where task transitions ride | `job.plan_changed`, which gains the task and its new state | Chosen here |
| Where per-task turns ride | `/observe`, filtered by `drone_id` (21.4); a Drone row names its task | Chosen here |
| Plan on `get_job`, or `get_plan` | Stays on `get_job` | Chosen here |
| Per-step gate ticks | Nothing ticked: nobody looks, Fleet's drift look stays | #1530, 22 Sep |
| The repository-decides state | A fourth state, overridable for this Job | #1530, 22 Sep |
| What a gate resolved to | Both policies recorded on every attempt; older reads absent | #1683, 1 Oct |
| Which wins after an override | The override, for the life of the Job | Question 4 |
| Difficulty tiers | A map on the Job, a tier on the task | #1530, 22 Sep |
| A task's model | A person picks it directly, over the map | `.claude/decisions/2026-09-30-a-person-can-pick-a-tasks-model.md` |
| A model nobody holds | Refused, as `set_model` refuses one today | `crates/fleet/src/job_settings.rs` |
| A per-Job Drone cap | Yes, inside the machine's | #1550, 22 Sep |
| The machine's cap | Recounted in Drones | #1530, 22 Sep |
| Who goes first when full | Every running Job keeps one agent | Question 2 |
| Criteria read from an issue | Editable to the press, frozen at approval, origin labelled | #1551, #1641 |
| Noticing the issue moved | On the pull-request rotation | Question 5 |
| Overlap at dispatch | Taken out | `.claude/decisions/2026-09-23-take-out-what-else-is-running.md` |
| `CriterionView.cases` | No; *not covered* belongs to a case alone | `.claude/decisions/2026-09-22-no-verdict-recorded.md` |
| A Judge and a Check sign the Record | Stored, with a migration | `.claude/decisions/2026-09-22-judge-and-check-sign-the-record.md` |
| The bounded stream | Built; slice 0b measures it | `crates/api/src/stream.rs` |
| A group's id | Minted by Fleet when the plan is recorded | Forced by #1685's move |
| A group's clock | Fleet stamps a group's start and end | Chosen here |
| Classifying | Is `proposing`, on the wire since 19.0 | `.claude/decisions/2026-09-21-classifying-is-proposing.md` |
| A request that splits | One Job; the rest are its members | `.claude/decisions/2026-09-30-a-dispatched-request-is-a-job.md` |
| A member counts as landed | When its pull request merged | `.claude/decisions/2026-09-21-a-parent-job-holds-members.md` |
| One copy per agent, in the contract | Amended in slice 5; overlap caught at the group's end | Question 6 |
| A Judge refusal at a group | Stops the group and waits for a person | Question 3 |

### Why the plan stays on `get_job`

**An open Job already reads `get_job` again on every event that names it**, and
every tab draws from that one read. The split precedent is size: `get_diff` and
`get_call` are split out because their payloads are large, a Check's output
alone running to two 64 KiB streams, and a plan is titles and paths. A second read would let Plan and Workflow draw two different
moments of one Job.

### Why a transition rides `job.plan_changed`

**It already fires on every kept change to a plan, and carries the counts a
Board row needs.** Adding the task's id and its new state, and later the group's,
lets a timeline move without a read, and costs two optional fields. The plan
itself still does not ride: titles are free text, and the channel is one
drop-oldest bound every Job shares.

### Which model a Drone runs, chosen here

A person's pick on the task, then the Job's map for the task's tier, then the
model the workflow step declares, then the Job's own. A later and more specific
choice wins, and every spawn records which one it ran as.

## The protocol changes, bundled

**The majors ride together, in whichever slice first needs one.** A major is the
lifeboat for any Fleet caught mid-Job, pre-alpha or not
(`.claude/decisions/2026-09-22-armada-is-pre-alpha.md`), so the milestone takes
one rather than several.

| Change | Bump | Why | Slice |
|---|---|---|---|
| `TaskState` gains `failed` | Major | Strict wire set; the task machine grew a state | 1 |
| `TaskState` gains the waiting state | Major | Same set, same bump | 1 |
| `Actor` gains `judge` and `check` | Major | Strict set, stored on every recorded row | 1 |
| Every other row on this page | Minor | New DTO, route, event kind or optional field | Per row |

**What spike 020 priced as major and the lock does without:**

| 020's major | Why it is not needed |
|---|---|
| `JobStatus` gains `classifying` | The status is `proposing`, already on the wire |
| `Actor::Human` renamed `person` | The wire keeps `human`; Bridge says *you* |
| `JobResources.worktree` becomes a list | Groups run in the Job's one copy; members are Jobs with their own |
| `Criterion.criterion_id` optional | Fleet mints an id at every edit before approval |
| `source` renamed `verified_by` | The wire keeps `source`; the draft renames on the way in |

**The store change is designed for an older Fleet, not around it.** Adding the
two signers is a migration in `crates/store/src/migrations.rs`, which moves
`KNOWN_SCHEMA_VERSION`. An older Fleet then refuses to open the store, rather
than folding a row it cannot spell in `crates/store/src/fold.rs`.

## The drafts, one module at a time

Every exported type in `packages/screens/src/draft/`. *None* in the bump column
is a type served today or one that never crosses.

### `coord.ts`, `task.ts`

| Type | Lands on | Change | Bump | Source of truth | Slice |
|---|---|---|---|---|---|
| `RunCoord` | `CheckRun`, `Recorded` | Optional `group`, `group_attempt`, `task` | Minor | Fleet's group record | 2 |
| `TaskState` | `TaskState` | Adds the waiting state and `failed` | Major | `crates/core-model/src/job/work_plan.rs` | 1 |
| `TaskTier` | `PlanTask.tier` | New, optional | Minor | The planner's `record_plan` | 3 |
| `TaskTreatment` | Nothing | Every task gets its own Drone | None | — | — |
| `TaskView` | `PlanTask` | Field by field below | Minor | `WorkPlan` in core-model | 1–5 |

| `TaskView` field | On the wire | Slice |
|---|---|---|
| `id`, `title`, `note`, `scope`, `expects`, `state`, `reason` | Served today | — |
| `shown` | Served; Fleet fills it from the task Drone's hand-in | 1 |
| `drone_id`, `turns`, `cost_micros` | From the `JobDrone` naming the task | 1 |
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
| `DroneView` | `JobDrone` | Adds `task`, then `model` | Minor | History and spend rows | 1, 3 |
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
| `LandingRule` | A Job's landing on `JobDetail` | New DTO, frozen at approval | Minor | Manifest base, then the person | 4 |
| `BranchView`, `BranchesAnswer` | A new branch-list read | New route | Minor | git, through the Vcs adapter | 4 |

`complete_when: all_members_landed` is answered in slice 6, and a per-group
pull request needs slice 2's groups.

### `ledger.ts`, `members.ts`, `wave.ts`, `peers.ts`

| Type | Lands on | Change | Bump | Source of truth | Slice |
|---|---|---|---|---|---|
| `LedgerActor` | `Actor` | Adds `judge`, `check`; no `contributor` yet | Major | `crates/core-model/src/envelope.rs` | 1 |
| `LedgerRow` | `Recorded` | Optional coordinate fields; `kind` stays opaque | Minor | `job_events` | 2 |
| `LedgerFamily`, `LedgerReads` | Nothing | Bridge's composition | None | — | — |
| `MemberLink` | Nothing | Only `merged` is built; Bridge derives it | None | — | — |
| `MemberView` | `JobSummary`, `JobDelivery` | Adds `merged_at`, read off the forge | Minor | The pull request | 6 |
| `JobMembersView` | Nothing | Bridge's composition | None | — | — |
| `WaveJobView` | `DispatchOrigin`, `JobDetail` | Pass stamp; `waits_on` is `dependencies` | Minor | The child's own record | 6 |
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
| `PulseView` | `JobResources` | One copy per Job; a parent reads its members' | None | — | — |
| `RepositoryDecides`, `RepositorySays` | `advance_gate`, `ManifestSummary` | Served | None | — | — |
| `GateView` | Per-step gates on the Job | New, frozen at approval | Minor | The approval body | 4 |
| `TierModels` | A Job's tiers | New; null is the Job's own model | Minor | Dispatch, then approval | 3 |
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
| `DraftWord` | `crates/core-model/domain/enum-verbs.toml` | Task-state rows; the `classifying` word goes | Registry | 2 |
| `JobDraft` | Nothing | The carrier; a field goes as each lands | None | — |

**Every task state owes a registry row**, the served ones included, which 020
found and is still true. The criterion's *no verdict recorded* owes a row of its
own, in slice 4. A Sketch's provenance on an attachment
(`produced_by`, `said`) is not in this milestone.

## What spike 020 said that is no longer true

| 020 said | Today |
|---|---|
| No Drone id on a turn; thinking dropped | Both served at 21.4 |
| An exited Drone is lost | `list_job_drones` keeps it, 21.3 |
| Nothing measures a log | Size and *being written* served at 21.2 |
| The wave's order is thrown away | `JobDetail.dependencies` serves the edges (#1692) |
| Overlap at dispatch is an open question | The panel was taken out, 23 Sep |
| The machine cap's unit is open | Recounted in Drones, 22 Sep |
| A sketch's boxes are an open question | A Studio's Sketch is the pad, 20.0 |
| A model per tier is free text | Free text, refused unless `list_models` offers it |

**One draft comment is now wrong.** `packages/screens/src/draft/task.ts` says the
planner picks the tier and the model follows; on 30 Sep a person may pick the
model directly. It is left for the slice that moves the type to the wire.

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
| #1752 | Fleet writes task state, and fills `shown` | 1 |
| #1641 | One approval body carrying the whole proposal | 4 |
| #1642 | An issue's address on a criterion's origin | 4 |
| #1651 | The header's Drone kill moves to the task's Drone | 5 |
| #1605 | A read listing a repository's branches | 4 |

**#1752's latest comment decides slice 1's shape.** Drones were offered
`update_task` and told to call it in `crates/fleet/src/crossing.rs`, and across
this machine two of eleven did. So Fleet writes a task's state, and fills `shown`
from the hand-in the task's own Drone already makes, or `the_evidence_accounts_for_itself`
refuses every task with an `expects`.

## The event stream

**This lock makes the risk worse, by a stated amount, and slice 0b exists to
put a number on it.** Slice 1 adds a `job.plan_changed` per task transition and
a spawned and exited pair per task, against one Drone pair per step today. Slice
5 multiplies that by how many tasks run at once.

| Stream | What the lock adds | Bound |
|---|---|---|
| `/events` | Task and group transitions, Drone lifecycle per task | `BACKLOG`, drop-oldest, `missed` then resync |
| `/jobs/:job_id/observe` | Rows from several Drones of one Job, interleaved | `WATCHING`, drop-oldest, `missed` |
| A transcript's file queue | Unchanged, one per Drone | Drops a row and writes `missed` into the file |

**Transcript rows never reach `/events`**, which is the decision that keeps the
Board's channel cheap. Several Drones share one per-Job socket rather than a
socket each, for 21.4's reason: one order on one clock.

## The backend milestone

**Claim: a Job's plan is worked by a Drone per task, group by group, and I can
see and act on each task, each group and each Drone.** One acceptance test,
written before slice 1 and added to slice by slice, in a new file beside
`crates/acceptance/tests/drone_per_step.rs`.

```
1 ──► 2 ──► 3 ──► 4 ──► 6
      │           ▲
      ▼           └── #1714 and #1716, a Job at proposing (being built)
0b ─► 5
7: a design first, and nothing waits on it
```

**One ordering departs from #1545's list, because the code argues for it.** 0b
runs beside slice 1 rather than before it: the bound and its resync are built,
and slice 1 runs one Drone per Job at a time, so its events arrive in sequence
rather than at once. The rate multiplies at slice 5, which 0b must precede.

**A second departure: `task_brief` moves into slice 1.** A Drone cannot be put on
a task without being told which task, so slice 3 keeps only the tier and the
edit.

**`crates/api/src/routes/served.rs` is one line under the size the gate
refuses.** Slices 2 to 6 add routes, so the first of them splits it.

### 0b — The stream, measured

**Claim: when my Bridge falls behind a Job running a Drone per task, it is told what it missed and redrawn, and never shows me a Board that is quietly wrong.**

| | |
|---|---|
| Issues | To file |
| Builds | A test driving per-task rates through the broadcaster against a stalled reader |
| Measures | Events per Job-minute by kind, from this machine's recorded Jobs |
| Write scope | `crates/api/src/stream.rs` and its tests |
| Blocks | 5 |
| Waits on | Nothing |

The number answers `[broadcast-capacity]` in `docs/practices/protocol.md`, which
is a person's to close.

### 1 — Task by task, each by its own Drone

**Claim: a Job's plan is worked one task at a time, each by a Drone of its own, and the plan tells me which task is being worked, which are done, and what showed each one done.**

| | |
|---|---|
| Issues | #1752; per-task dispatch, to file; the two signers, to file |
| Wire | Major: the waiting state, `failed`, and the two signers |
| Blocks | 2, 3, 5 |
| Waits on | Nothing |

- A workflow step says it works the plan's tasks, one Drone each; feature, bug and
  refactor set it on `implement`. Bridge reads the flag where it now derives it.
- Each task's Drone gets a brief naming its task, spawned in plan order, with
  the machine's cap and headroom asked again at every spawn. A Job keeps its one
  agent between tasks, so a Job waiting to start never takes it mid-plan.
- Fleet writes `working` at the spawn and *handed in, waiting for Checks* at the
  hand-in, with `shown` from it, then `done` when the step's Checks pass.
  `JobDrone` gains `task`; `job.plan_changed` gains the task and its state.
- The milestone's majors land here together: the waiting state and `failed` on a
  task, and a Judge and a Check signing the Record, with the store migration
  above. Fleet writes `failed` from slice 2.
- The step's Checks and Judge run once, at the step's end, as one group. A Drone
  that exits without handing in stops the step, as today.

Write scope: `crates/fleet/src/spawning.rs`, `crates/fleet/src/briefing.rs`,
`crates/fleet/src/crossing.rs`, `crates/fleet/src/work_plan.rs`,
`crates/fleet/src/drones_had.rs`, `crates/fleet/src/admitting.rs`,
`crates/core-model/src/job/work_plan.rs`, `crates/core-model/src/job/workflow.rs`,
`crates/config/src/workflow/step.rs`, `.armada/workflows/`, `crates/ipc/src/work_plan.rs`,
`crates/ipc/src/drones.rs`, `crates/ipc/src/enums.rs`, `crates/ipc/operations.toml`,
`crates/core-model/src/envelope.rs`, `crates/store/src/migrations.rs`,
`crates/store/src/fold.rs`, `docs/concepts/plan.md`, `docs/contracts/agent-prompt.md`.

### 2 — Groups, and a task that failed

**Claim: when a group's Checks go red, the Record names that group and that run, its tasks say they failed and why, and I can run one task again or move it before the group goes round.**

| | |
|---|---|
| Issues | #1652, #1656, #1685; groups recorded and run, to file |
| Wire | Minor; the majors landed in slice 1 |
| Blocks | 3, 4, 5 |
| Waits on | Slice 1 |

- The planner records groups; Fleet mints each group's id and stamps its start,
  its end and each attempt's verdict and commit. Checks run at each group's end,
  and a group commits once.
- `CheckRun` and `Recorded` carry the coordinate. Fleet writes `failed` with its
  reason, and the flag a later task's write sets.
- A Judge refusal stops the group and waits for a person; it never sends the
  group round on its own.
- Restart one task and move a task or a group, as routes. The task-state registry
  rows land.

Write scope: `crates/fleet/src/gate.rs`, `crates/fleet/src/checking.rs`,
`crates/fleet/src/settling.rs`, `crates/fleet/src/work_plan.rs`,
`crates/fleet/src/commanding.rs`, `crates/core-model/src/envelope.rs`,
`crates/core-model/src/job/work_plan.rs`, `crates/store/src/migrations.rs`,
`crates/store/src/fold.rs`, `crates/ipc/src/enums.rs`, `crates/ipc/src/checks.rs`,
`crates/ipc/src/history.rs`, `crates/core-model/domain/enum-verbs.toml`,
`packages/protocol/src/pending.ts`.

### 3 — A model per task

**Claim: a hard task runs on the strong model and an easy one on the cheap one, a task I gave a model of my own runs on that one, and each Drone tells me which it ran.**

| | |
|---|---|
| Issues | #1657; the tier map on the Job, to file |
| Wire | Minor |
| Blocks | 4 |
| Waits on | Slice 2 |

- A Job carries a tier map, and the planner gives each task a tier.
- Edit a task's title, brief, files, done-when and model, before the plan's gate
  and after a failure, recorded as a person's change. A model `list_models` does
  not offer is refused.
- Each spawn resolves the model in the order chosen above, and `JobDrone` says
  which it ran.

Write scope: `crates/fleet/src/job_settings.rs`, `crates/fleet/src/spawning.rs`,
`crates/fleet/src/commanding.rs`, `crates/core-model/src/job/work_plan.rs`,
`crates/ipc/src/work_plan.rs`, `crates/ipc/src/drones.rs`, `crates/ipc/src/detail.rs`,
`packages/protocol/src/pending.ts`.

### 4 — Deciding what a Job will be, and how it lands

**Claim: everything I change on a proposal — its words, workflow, gates, criteria, tiers and how it lands — is what the Job runs after I approve it, and a criterion read from an issue says so and says when that issue moved.**

| | |
|---|---|
| Issues | #1641, #1642, #1605, #1581, #1683 if still open, #1699's route; issue noticing, to file |
| Wire | Minor |
| Blocks | 6 |
| Waits on | #1714 and #1716 landing |

- One approval body carries the whole proposal (#1641). An edit that saves without
  releasing shares its fields, and is #1699's route.
- Criteria freeze at approval, carry their origin and the issue's address, and say
  when the issue moved. The three documents #1581 names move in the same change.
- Per-step gates with the override, which holds for the life of the Job however
  the repository's rule moves. The landing settings, the branch list, the
  dispatch settings and `approved_at`.
- The rotation that watches pull requests also asks whether a linked issue moved.

Write scope: `crates/fleet/src/proposing.rs`, `crates/fleet/src/commanding.rs`,
`crates/fleet/src/policy.rs`, `crates/fleet/src/noticing.rs`,
`crates/core-model/src/job/fields.rs`, `crates/core-model/src/job/declared.rs`,
`crates/ipc/src/job.rs`, `crates/ipc/src/detail.rs`, `crates/adapters/src/issue_lookup.rs`,
and the three documents.

### 5 — Several Drones at once

**Claim: tasks the planner marked as safe together run at once, within this Job's cap and the machine's, and I can message or stop any one of their Drones without touching the others.**

| | |
|---|---|
| Issues | #1666, #1651; concurrency, to file |
| Wire | Minor |
| Blocks | Nothing in this milestone |
| Waits on | Slices 0b and 2 |

- The planner declares which tasks may run together. The machine's cap counts
  Drones, and a Job carries its own cap inside it. A Job's second Drone yields
  to a Job waiting to start.
- Redirect and kill take a Drone id and refuse one that is not live. A process
  names its Drone, and the header's Drone kill goes.
- The group joins before its Checks. Where two tasks wrote one file, it does not
  commit, and those tasks run again one after the other.
  `docs/contracts/system-architecture.md` is amended in the same change.

Write scope: `crates/fleet/src/slots.rs`, `crates/fleet/src/admitting.rs`,
`crates/fleet/src/resources.rs`, `crates/fleet/src/commanding.rs`,
`crates/ipc/src/resources.rs`, `crates/ipc/src/limits.rs`, `crates/config/settings.toml`,
`docs/contracts/system-architecture.md`.

### 6 — Jobs under a Job

**Claim: an Epic's next wave is real Jobs I can read and correct before I approve it, one press starts them all, and the parent finishes when every member's pull request has merged.**

| | |
|---|---|
| Issues | #1694, #1699, #1692 if still open; a parent completing on its members, to file |
| Wire | Minor |
| Blocks | Nothing |
| Waits on | Slice 4 |

- An Epic's split creates its children at `awaiting_approval`, with their waits-on
  edges and the pass that made them. One act releases the wave, all or nothing.
- A merged pull request records when it merged, read off the forge. A parent
  completes when every member has landed.

Write scope: `crates/fleet/src/sub_dispatch.rs`, `crates/fleet/src/noticing.rs`,
`crates/fleet/src/commanding.rs`, `crates/core-model/src/job/fields.rs`,
`crates/ipc/src/job.rs`, `crates/ipc/src/detail.rs`.

### 7 — Workflow profiles

**No claim yet**: what a profile is has no design.

| | |
|---|---|
| Issues | To file, after a design pass |
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
| A sketch's provenance on an attachment | Nothing reads it yet |
| Who judged a wave | Nothing records the model on a verdict |

# Plan

**What it is:** A Job's own record of what it means to do the work — an
approach and an ordered list of tasks, recorded by one step and kept current
by the steps that follow it.

---

**Kind:** Entity. Owned by [Job](job.md), one per Job.

Designed with the owner as the Plan milestone. #893 built the record and the
Drone tools that write it, #894 put it in a Drone's brief, and #895 wired the
workflow fields. #896 draws it in job detail, #897 lets a person change it,
and #898 draws it on the Board — none of the three built yet. #1006 let any
step record it beside its own product, so all eight carried workflows keep
one.

## What it is

**One Plan per Job, not per step.** [Job](job.md) owns it the way it owns
`facts[]` and `escalations[]` — a record the Job carries, not an actor.

A Plan holds an `approach` — a paragraph — and an ordered list of Tasks.

**Tasks sit in groups, and a group is what runs.** The plan's tasks are ordered into groups; the groups run one at a time in plan order, in the Job's one worktree, and the step's Checks run at the end of each group rather than once over the whole step. A verdict then names the group that broke, and a retry re-runs only that group. Built in spike 022's slice 2 for a step declaring `drone_per_task`; *Groups* below has how.

Groups exist because a parallel schedule cannot be derived. Intersecting the tasks' `scope` lists finds every edge where two tasks write one file, and finds none of the edges where one task uses what another made — four of the eight tasks on the Job the owner killed on 18 Sep were linked that way and shared no declared path. Plan order carries those edges, so running in plan order honours them; scope intersection catches a plan whose order contradicts its own paths.

| Field | On | Meaning |
| --- | --- | --- |
| `approach` | Plan | The step's own account of how it means to do the work |
| `id` | Task | Stable, assigned once, never reused |
| `title` | Task | One line |
| `note` | Task | One line for what the other fields cannot hold. Optional |
| `scope` | Task | The repository-relative paths this task touches |
| `expects` | Task | What should prove it, written by the step that plans |
| `shown` | Task | What did prove it, written by the step that does the work |
| `state` | Task | `open`, `working`, `handed_in`, `done`, `failed`, `dropped`. The wire carries all six since protocol 22.0; Fleet writes `handed_in` since 23.1 and `failed` since 23.4 |
| `group` | Task | The group it runs in, `G1` and on, minted by Fleet at the recording and never renumbered by a move. The planner numbers groups with `record_plan`'s `group`; a task naming none joins the one before it, so a plan naming none is one group |
| `reason` | Task | Required when `state` is `dropped` |
| `concurrent_with` | Task | Which tasks in its group may run at the same time, declared by the planner and read both ways. A pair whose edit calls once named one file is run apart for the rest of the plan. Since spike 022's slice 5 |

**`scope` is a list because the step after the planning one reads it.** It
was prose inside `note` until `#1421`, and a step handed prose went looking
for the same files again — the Job the owner killed on 17 Sep had recorded
`packages/surfaces/overview/src/overview.ts` against the task that changes it, an hour
before its next Drone grepped for the same file.

**`expects` and `shown` are two fields and are never reconciled into one.**
The planner names an artifact before the work starts; whoever does the work
finds out what actually proved it. The pair disagreeing is the fact worth
reading, so nothing collapses them — the same shape declared `scope` has
against the files a task actually touched.

**A Judge reads the pair, and refuses silence rather than difference.**
Feature, Bug and Refactor ask `the_evidence_accounts_for_itself` on the step
that follows the plan: for each task set `done`, does its `shown` demonstrate
its `expects`, or say why the work proved it another way? On a step working
a Drone per task the Judge runs before the tasks turn `done`, so both
criteria ask it of every task done or handed in. A task that proved
its work differently and says so passes — that is how work finds the real
seam. What is refused is a `shown` that neither matches nor accounts for
itself. Epic plans too and asks nothing, because its steps that follow the
plan are handed no `reference_docs` and a task there is a dispatched Job.

**Every change records who made it** — a step, or a person — and **every
change is an appended row**. The current list is derived; history is never
overwritten.

## Who may change it

| Who | May do |
| --- | --- |
| The step that records the plan | Record it whole, while its step runs. A retry, or a loop's return to that step, replaces the plan |
| A step declaring `follows_plan: true` | Add a task, and move one to `working`, `done` or `dropped` with a reason. A `done` task may move back; a `dropped` one stays dropped. Legal on the recording step itself, so one step may plan and keep its own tasks current |
| A step declaring `drone_per_task: true` | Nothing, by its Drones. Fleet marks each task from its Drone, as below, and the step's Drones are given no plan tool even beside `follows_plan` |
| A person | Add a task, or drop one with a reason, from Bridge, while the Job runs. Move a group, or a task into any group, by the one it comes after (`move_plan`, #1685), never a task still working or handed in. Restart a failed task (`restart_task`, #1656). Edit an open or failed task's title, note, scope, expects or model (`edit_task`, #1657) |
| Any other step | Read the plan. Change nothing |

A retry of a step that follows the plan keeps task states — the work behind
them is still on the branch, so the plan does not reset with the step.

## Fleet writes a task's state, and a Drone stops claiming it

**On a step declaring `drone_per_task`, built in spike 022's slice 1b.** Fleet marks a task `working` when it spawns the task's Drone, `handed_in` when that Drone hands in, with `shown` taken from the hand-in's `shown_by`, and `done` when its group's Checks pass, in place of a Drone claiming any of them. Since slice 2 it marks `failed` too, once its group's retries run out. On the Job of 18 Sep three tasks flipped to `done` within 1.6 seconds of each other and five never entered `working` at all, which is what a self-reported state is worth; across one machine, two of eleven Drones told to call `update_task` did (#1752). A step that keeps `follows_plan` without the key still self-reports.

**`handed_in` is the state in between**, the owner's answer 1 in spike 022: the task's agent handed its work in and its group's Checks have not answered. Done arrives at green, and a Judge refusing after the Checks does not undo it.

## A Drone per task

**Feature, Bug and Refactor set `drone_per_task` on `implement`.** Feature's `tests` keeps one Drone and keeps `follows_plan`: its tests are written against the whole change.

| When | What Fleet does |
| --- | --- |
| The step is entered, or a task's Drone handed in | Puts a Drone on the first task, in the group being worked, that is `open`, or `working` under a Drone that is gone, and marks it `working`. Its brief names the task and says the hand-in ends it |
| That Drone calls `submit_evidence` | Keeps the hand-in as the task's, marks it `handed_in`, answers `recorded`, and puts nothing in the step's evidence inbox |
| The first turn after that Drone comes to rest | Ends it and spawns the next task's on the same worktree. No gate runs, and the cap and headroom are not asked: the Job keeps its one agent (answer 2). It waits for the rest because the Drone's turns and cost arrive on its last line; one that has not rested within the report grace is ended without them |
| No task of the group is open or working | Puts one submission in the inbox carrying each of the group's claims, labelled `T1: …`. The step's Checks and Judge run at the group's end |
| The group's Checks pass | Marks its `handed_in` and `failed` tasks `done`, and *Groups* below says what follows |

**The group's last task's Drone stays for the outcome.** Its hand-in is what fills the inbox, so a red Check hands the work back to it, as a step retry is one Drone today. **A Drone that exits without handing in stops the step**, as any Drone does. A step restarted with every task of its group already handed in gets one Drone for the step.

**What crosses from one task's Drone to the next** is the step's baseline and every path declared so far, so the gate measures the step's whole diff against every task's declaration rather than the last one's. Which Drone worked which task, and what each handed in, is kept beside the plan (store V92), so `list_job_drones` names each Drone's task.

**`failed` is a task whose group's Checks were still red when the step's retries ran out** (answer 9). It is not `open`, not `working`, not `done` and not `dropped`. Restart this task puts a Drone on it again, and a green gate over its group is what clears it.

**A re-run that passes clears it too** (4 Oct 2026, Job 3). Run Checks again (#1105) takes no group's gate through `done`, so after one the failed tasks stayed `failed` with the red run's reason, and the Judge read them beside Checks that had passed, refused `the_evidence_accounts_for_itself`, and stopped the step again. A green ruling on a re-run now marks the failed tasks of the group it ruled on `done` and drops the reason, before the step moves. A task failed in another group is left alone. **The Drone's `shown` is kept as it wrote it**; the Judge's brief carries a line after the plan naming the settled tasks and saying each `shown` was written before the re-run. `crates/fleet/src/rerun_settles.rs`. A re-run that is still red changes nothing.

**A re-run that passes closes the group and starts the next** (5 Oct 2026, #1792, https://github.com/NickMele/armada/issues/1792). The pass used to advance the step whatever the plan still owed, so with two groups and G1 red to the end of its retries, a press that went green moved on and G2 never ran. A green re-run now does what the group's own gate does: G1's run is closed `passed`, with its one commit where a group follows, and where a group follows the step stays `running` and the Job is queued, so admission puts a Drone on G2's first task. The step advances only when no group is left, which is the single-group case as before. The Judge has already ruled on G1 by then; it is not asked again before G2. `crates/fleet/src/rechecking.rs`, `grouping.rs` (`group_closed`).

**Neither is a Drone's or a person's to set.** `update_task` refuses both, because Fleet marks them from a hand-in and from a group's Checks.

**A task carries its needs** (#1059). A task of `record_plan` or `add_task` takes `needs`, each a path and what is needed there, and Fleet declares them as it keeps the plan. They are the Job's and not the task's: dropping the task does not give one back. `docs/concepts/fleet.md`, *Declared needs*.

**A done task a later task edits stays done, and is flagged.** The work behind it is still on the branch, so nothing reopens it; what a person needs is to know that somebody wrote into its files afterwards, which the flag says and the state does not.

## Groups

**Spike 022, slice 2.** Fleet keeps each group's runs beside the plan (store V95): when each began and was answered, the step's run it was filed under, its verdict, and the commit a green run made. A group's run begins at its first task's spawn.

| At a group's end | What Fleet does |
| --- | --- |
| Its Checks and Judge pass | Its tasks are `done`. Where a group with work follows, it commits once and the next group's first task gets its Drone; the step moves only after the last group |
| A Check is red and the step's `retry_limit` allows another run | The group goes round on its own: the same Drone is told the red Checks and then every task of the group, and the tasks stay `handed_in` |
| A Check is red on the last run allowed | Every task in the group turns `failed`, with a reason naming the group and the run, and the step stops for a person |
| The Judge refuses | The group stops for a person, as a step does (answer 3). The Checks passed, so its tasks read `done`, and Restart this task answers each of them: the idle Drone ends and a new one works that task alone, opening with the refusal |

**The budget is the group's own runs**, not the step's: a later group starts with all of its retries. A person's restart does not reset it, as `restart_step` does not reset a step's, so a restarted group that is red again fails at once.

**Every group runs every gate Check over the whole copy.** Selecting tests by file and a test's last group wait on #1274. A Check declared `runs_at: handoff` is held back at every group but the last, so the step before handoff runs every Manifest Check once, after its last group.

**A Check run and a Record row name their group.** A gate Check's run carries the group and its run, and so does the step move a red run or a stop made, so a passed group's Checks stay its own beside a later group's.

## A model per task

**Spike 022, slice 3.** The planner gives each task a tier, `difficult`, `medium` or `easy`, or none, and the Job carries a map from tier to model (`set_tiers`). Each task's Drone is spawned on, in order: a person's pick on the task, then the map for the task's tier, then the step's own model, then the Job's. **A tier the map leaves out is Armada picking** (answer 8), which is the last two. Every spawn keeps the model it ran (store V97), and `list_job_drones` names it.

**A person's edit is a change after the recording, never a new one.** Edit this task reaches an open or a failed task, so it works while the plan waits on a person and after a group fails; the plan still reads as the planner's, and the history shows the edit after it. The next Drone put on the task reads the edit in its brief.

## Tasks that may run at once are declared, never inferred

**Built in spike 022's slice 5.** The planning Drone names which tasks in a group may run at the same time, as `concurrent_with` on `record_plan`. It is a declaration because it cannot be a derivation: intersecting declared paths finds the write-write edges and none of the read edges, and a planner that has just written the plan knows both.

**They share the Job's one worktree.** Two Drones writing different files in one checkout do not collide. What needs care is anything that *reads* the tree, which is the group boundary the design already has: the concurrent tasks join, then the Checks run on a still tree, then the group commits once. Tasks that ran at once cannot each have their own commit.

**Every running Job keeps one Drone, and its extra Drones wait their turn** (answer 2). The Job's kept Drone holds its place from admission to its end. A Drone beside it, on a task safe beside every task being worked, asks the machine's cap, which counts Drones, the Job's own Drone cap inside it, memory and the repository's disk, and gives way to a Job waiting to start. Refused, it waits a turn. A Drone beside the kept one that leaves without handing in, or that a person stops, puts its task back to `open` for the kept Drone, and none is started beside it again for that task.

**An undeclared write is caught at the join, off the Drones' edit calls** (answers 6 and 10). Each task Drone's edit calls are kept at its first hand-in. When the group's last task is in, two tasks whose Drones ran at the same time and named one file go back to `open` and are run apart for the rest of the plan, so the group does not reach its gate and does not commit; they run again one after the other. **A write through the shell — `sed`, a formatter, a code generator — names no file and is not seen.**

**A later task coming back to a done task's file is said, not refused.** Where a later group's task's edit calls named a file a done task's had, that task reads `touched_after_done`.

## A task's cost appears when its agent stops

**Turns while it runs, cost when it ends.** Cost arrives on a session's last line — see [Machine](machine.md) — so a live figure would be invented. It is the task's own agent stopping that settles it, not the group passing.

## Task state is never a gate

**Rule.** Fleet never refuses a step's submission because a task is open.
Why: while task state is a Drone's own claim, `docs/scope.md` treats
self-report as a signal and not a source of truth — and once Fleet writes the
state itself, the state says what Fleet already knows rather than adding a
second gate over it.

A step declaring `follows_plan: true` may still be judged against the plan's
task states — whether the diff does what a task set `done` claims, whether a
`dropped` task's reason holds, and whether a done task's `shown` accounts for
itself against its `expects`. Those are Judge criteria, asked of the diff and
of the Drone's own account of its evidence, never a mechanical gate on the
plan itself. `plan_recorded` is the one mechanical check touching a Plan, and
it asks only that one exists with at least `min_tasks` tasks — see
[Workflow](workflow.md).

## How a workflow declares it

A workflow names the step that records the plan one of two ways: giving it
`evidence.submitted.type: "plan"`, where the plan is the whole of the
step's product, or giving it `records_plan: true` beside another product —
Code Review's `read` keeps `read.md` and records the plan too. Either way
the step needs a `plan_recorded` mechanical check. A later step that keeps
the plan current declares `follows_plan: true`, legal on the recording step
itself as well as on every step after it. [Workflow](workflow.md) owns the
field-by-field schema and the refusals a definition can trip — two steps
recording a plan, a recording step missing `plan_recorded`, `follows_plan`
on a step before the recording step or in a workflow with none.

## The plan never replaces a step's own product

A step that records the plan beside another product keeps both. The Judge
reads the step's own deliverable or diff and the plan, labelled apart, never
one folded into the other — Epic's `plan` step keeps its split drawing this
way, not just the tasks it broke the wave into. A later step naming
`<recording_step>.evidence` in `reference_docs` reads the same pairing: the
recording step's own submission, with the plan appended after it.

## Distinct from the declared scope's "plan"

**The word "plan" already names something else in this codebase.** A
step's declared file scope — what `declare_plan_at: step_start` and the
`job_step_plans` table hold, read against by the drift check — is a list of
paths a Drone said it would touch, unrelated to this Plan's tasks. The two
must not collide in name: this document's Plan and Task are the Job's own
record of its work, not a file-scope declaration.

## Open questions

- **[case-waits-for-last-group]** When a later group edits a file an earlier group's task wrote, should that task's cases still wait for the later group's boundary, and is the overlap worth drawing? Today a case runs at the boundary of the last group holding a task it covers, and again at handoff (#1530): if group 3's T6 writes `running-rows.tsx` and group 4's T7 edits it again, T6's case is skipped at group 3's boundary and runs at group 4's. Nothing runs at the same time, since groups still run one at a time. The cost is timing: a break T6 made surfaces a group late, after T7 has changed the same file. Plan draws the overlap as an amber callout on the later group ("Group 4 writes these files too") and as a line in the earlier task's panel ("T7 edited a file this task had already finished"). Neither says the consequence, and a later task editing an earlier one's file is not a fault in itself. What decides it: once Fleet runs groups, whether a break found a group late costs more than running a case twice, and whether a person reading the plan needs to know which boundary a case waits for. Groups are not yet built in Fleet, so there is nothing to measure yet.

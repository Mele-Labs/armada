# Trigger

**What it is:** Something that runs at a moment in a [Job](job.md), named in a file a person wrote.

---

**Kind:** Concept.

**Built:** the model, the loader, the freeze at approval, Fleet firing a Command Trigger, `repair`, the wire to Bridge, Bridge's saved Triggers, step cards and Job card, steps added to one Job in Fleet and on the wire, the `+` that adds one on Bridge's approval canvas and a running Job's Workflow tab, the repair branch on a Job's canvases, and `block`, which holds the Job, with the owner's Rerun and Skip, a bell on a Board row, and the hold on Bridge's canvas and list. Skills, Drone steps and asking the owner about a destructive Command are not.

## What a Trigger is

| Field | Values | Notes |
|---|---|---|
| `name` | text | Part of its identity |
| `when` | `step_starts`, `step_passes`, `pr_opened` | `pr_opened` is the delivering step's entry |
| `workflow` | a `workflow_id` | Absent is every workflow |
| `step` | a step id | Absent is every step. Not allowed with `pr_opened` |
| `command` or `skill` | a name | One of the two |
| `on_failure` | `block`, `repair` | Both default to false |

**A Command is run by Fleet with no Drone, and a skill by a Drone.** Skills are modelled and not executed: one is recorded `skipped`.

## Where one is set

> **Rule.** The most specific level wins, and its copy replaces the others whole.
> Why: a field merge makes a Trigger nobody wrote.

| Level | Where | Notes |
|---|---|---|
| Armada | Compiled in | Ships none |
| Repository | `.armada/triggers/` | Read from `main` only |
| This machine | `~/.armada/machine/triggers/` | Machine, not [Kit](kit.md): it does not travel |

**Identity is when, step and name.** The workflow a Trigger applies to is not part of it.

> **Rule.** A repository's Triggers are read from `main` and never from the Job's branch.
> Why: a Drone must not write the Trigger it runs under, the fence [Workflow](workflow.md) has.

## What becomes of one

| Case | Result |
|---|---|
| Names a Command the repository declares | Runs |
| Names a Command it does not declare | Skipped, marked on the Job |
| The Command is `destructive` | Not run. Recorded `awaiting_owner`, and nothing asks him yet |
| The file does not parse | Left out with its reason, the others stand |
| Two files in one place share an identity | Both left out, named together |

> **Rule.** A Trigger's exit code never advances or fails a gate.
> Why: a gate is a [Check](manifest.md), and a Trigger is not one.

> **Rule.** Triggers freeze onto the Job at approval.
> Why: the workflow does, so one saved later applies from the next Job.

## When one fires

| Moment | Where Fleet fires it |
|---|---|
| `step_starts` | The step's Drone is being put on, after the catch-up |
| `step_passes` | The step moves to `advanced`. An override is not a pass |
| `pr_opened` | Right after the delivering step's entry opens the pull request. Not for a pull request found already open, and not for a Job that lands `local` |

A Command runs in the Job's worktree under the Check budget, with no shell. What it prints goes to the Job's log and is never read. Each firing is a row in `job_triggers`: Trigger, level, moment, step, state, exit code and times. The states are `skipped`, `running`, `passed`, `failed`, `awaiting_owner`, `repairing`, `rerunning`, `fix_ready` and `held`.

> **Rule.** A failed Trigger changes neither the Job's status nor its step, unless it blocks.
> Why: a Trigger is not a Check. `block` is the one thing a person can ask for that holds the Job, and it holds it and does not fail or advance it.

## A failed Trigger with `repair` on

| State | Means |
|---|---|
| `repairing` | It failed, and a repair Drone is working on a branch cut from the Job's. This is where a failure with `repair` on stops, in place of `failed` |
| `rerunning` | The Command is running again, on the repair branch or on the Job's |
| `fix_ready` | The Command passes on the repair branch. **The owner chooses where the fix goes**, and Fleet never does |

`this_branch` merges the fix onto the Job's branch and pushes it, so it lands on the Job's open pull request, and the Command runs again there. `new_pr` pushes the repair branch and opens a pull request of its own against the Job's target. Either ends `passed`, and `this_branch` ends `failed` if the Command still fails, or `held` where the Trigger blocks.

> **Rule.** A repair is bounded at 2 tries. A Trigger that fails after both is `failed` for good, or `held` where it blocks, and the Job gets an alert.
> Why: a third repair is one that does not hold, as the worktree's is. The alert is the existing `list_alerts`, and the Job's status is where it was.

The firing's row carries the tries, the repair branch, the files the fix changes, the choice and the pull request. All of it is on the wire as `JobTrigger.repair` (protocol 23.68), each state is a `job.trigger_changed`, and `choose_trigger_fix` is the owner's choice. Bridge draws the repair as a branch off the Job's workflow, on the Overview canvas and the Workflow tab, from the same rows. `docs/concepts/fleet.md`, *A failed Trigger's repair*, has the mechanism.

## A failed Trigger with `block` on

A failure holds the Job at the place its moment stands in front of, and the firing is `held`.

| Moment | Holds | Job |
|---|---|---|
| `step_starts` | Before the step's Drone runs | `escalated`, reason `trigger_held` |
| `step_passes` | Before the next step starts. **Not the last step's**: nothing follows it, so a failure there is `failed` and holds nothing | `escalated`, reason `trigger_held` |
| `pr_opened` | Before the review gate can be answered. The delivering step's advance becomes a held review, and the approval and the merge are refused with `fleet.trigger_holds` | Where it was |

> **Rule.** A hold is not `awaiting_repair`.
> Why: `awaiting_repair` is a spent Check budget, and a Trigger is not a Check. A hold is a state of the firing, and a status of the Job only as the existing `escalated` with its own reason.

With `repair` also on the hold waits through the repair, and each state of it still holds: `repairing`, `rerunning` and `fix_ready` on a Trigger that blocks. A repair that ends `passed` lets it go by itself, and two failed tries leave it `held`. **Jobs heal themselves first; the owner is the last resort.** An added step that blocks holds the same way, and its `repair` is still carried and not acted on, so it holds at once.

| Owner's act | Does | Refused with |
|---|---|---|
| Rerun | Runs the Command again with no Drone. It passes, the firing is `passed` and the hold is let go. It fails, the firing stays `held` | `fleet.no_hold`, `fleet.hold_repairing`, `fleet.hold_has_a_fix`, `fleet.hold_nothing_to_run`, `fleet.hold_no_worktree`, `fleet.hold_job_working` |
| Skip | Lets it go and records the firing `skipped`, reason `by_owner` | `fleet.no_hold`, `fleet.hold_repairing` |
| Kill | The act that already exists | |

A hold let go with nothing else holding the Job puts an `escalated` Job back in the queue, and admission starts the step it stopped before. **A hold let go before a step starts is not fired again when the Job gets there**: that would undo a skip, and rerun a pass.

A held Job is an alert, and so is a repair fix waiting on his choice and a Trigger that failed after its repair tries. `JobSummary.alert` names the Trigger, so a Board row draws the bell, and `list_alerts` says why.

The frozen set is `job_frozen_triggers`, one row per step a Trigger fires on. The repository's files are read from the base branch by `adapters::triggers_on_base`, and this machine's by `armada::Locator`.

## On the wire

Protocol 23.58, the four operations and one event, 23.68, steps added to one Job, 23.68, a failed Trigger's repair, and 23.68, a Trigger that blocks. `docs/practices/protocol.md`.

| Operation | What it does |
|---|---|
| `list_triggers` | What a repository runs, each with its level and file, what it replaced, and the files left out with why |
| `get_trigger` | One Trigger as YAML text, from any level that holds a copy |
| `save_trigger`, `remove_trigger` | Write or delete a file in the repository's folder or this machine's |
| `job.trigger_changed` | One of a Job's Triggers moved. `JobDetail.triggers` is the rows, with the pending ones |
| `add_job_step`, `remove_job_step` | Add a step to a running Job, or take one off before it fires. An `approve_dispatch` carries the ones placed at the press |
| `job.addition_changed` | One of a Job's added steps moved. `JobDetail.additions` is the rows, with the pending ones |
| `rerun_trigger`, `skip_trigger` | The owner's two acts on a hold. The body names a Trigger or an added step, and the answer says where the firing stands and whether the Job holds nothing now |

> **Rule.** A save is checked with the loader's rules before anything is written.
> Why: a Trigger that is saved is one that loads.

**A repository's save refuses a Command its `armada.yml` lacks, and a machine's does not.** The loader skips a Trigger like that so the same file is right in the next repository. A machine's applies to every repository, so refusing it for the one in front of a person would undo that. Its answer says it is skipped here.

> **Rule.** A repository's Trigger runs once it is on `main`.
> Why: Fleet reads a repository's files from the base branch. A save writes the checkout and answers `waits_for_main`, and a removal there waits the same way.

Nothing is held, so a save is on the next `list_triggers`. A Job's log line for a firing is stamped with the firing's own end, which is what `JobTrigger.log_at` points at.

## In Bridge

| Where | What it draws |
|---|---|
| Workflows, beside the list | The Triggers that apply to every workflow, and an editor for one: When, Step, Applies to, Set in, Runs, and the two switches under If it fails |
| A workflow's step | The Triggers that fire at it, one line a moment. The delivering step also carries the Draft PR switch, which is its `draft_pr` |
| Settings, This machine | The Draft pull requests switch, which is `draft_pull_requests` |
| A Job's Overview | Its Triggers, each with the state as a mark and the level that sets it. A firing's name opens its line in the Job's log through `log_at`. A hold is drawn at the node the Trigger fired at, with Rerun and Skip beside it, and the list view of the Job offers the same two |
| A Board row | The bell, with a tooltip naming the Trigger, for a hold, a fix waiting on his choice, and a Trigger that failed after its repair tries |

A copy a more specific level replaced is drawn struck through under the one that runs. A repository's save is marked as waiting for `main`, and a machine Trigger on a Command the repository lacks is marked skipped. The repository's own `pr_mode` is edited with `set_pr_mode` in `edit_manifest`, and `ManifestDeclared.pr_mode` carries it back, so Bridge can offer the repository's Draft default beside the machine's and the step's.

## Steps added to one Job

A person can add a step to one Job without writing a workflow. It lives in the Job's own record, `job_additions`, **beside the frozen workflow and never in it**, and it is that Job's alone unless he keeps it.

| Kind | Runs | Becomes |
|---|---|---|
| Script | A Command named in `armada.yml`, by Fleet with no Drone | Fires as a command Trigger does |
| Skill | A skill, by a Drone | Recorded `skipped`, as a skill Trigger is |
| Drone step | A short brief, by a Drone | Recorded `skipped`, and says so |

**A place is a moment and a step**, a Trigger's. Before a step is its `step_starts`, after it is its `step_passes`, the gap before the pull request opens is the delivering step's `step_starts`, and the gap after is `pr_opened`. After merge is deferred. Each carries `block` and `repair`, off unless set, carried and not acted on yet.

| When it is added | Which gaps |
|---|---|
| At dispatch, in the approval | Every one: before the first step, between steps, and both sides of the pull request |
| On a running Job | The step it is on and every one after. A gap behind is refused with `fleet.added_step_behind` |

> **Rule.** A step added to a Job fires through the Trigger path and records on its own row.
> Why: it has no level, so `job_triggers` would need a fourth one. The latest state is kept on the addition, and each firing is a line in the Job's log.

> **Rule.** A Drone step is recorded `skipped` and not run.
> Why: a step a Drone works needs a gate, and the frozen workflow's step rows are the only one Fleet has. A second gate model for one Job is not a step.

**Fleet has no edit for an addition**, so its switches are set before it is added: at the gate it is held in the approval until the press, and on a running Job it is filled in and then added. An addition can be removed until its moment has come. **Keeping it for every Job** is `save_trigger` with `kept_from`: the editor draws the Trigger, Fleet writes it at This machine or Repository, and the addition says where it went. A Script or a Skill can be kept and a Drone step cannot. A kept one applies from the next Job, as every saved Trigger does.

The loader is `config::TriggerCatalogue`, the type is `core_model::Trigger`, and the decision is `.claude/decisions/2026-10-07-a-trigger-runs-at-a-moment-in-a-job.md`.

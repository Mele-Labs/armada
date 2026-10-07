# Trigger

**What it is:** Something that runs at a moment in a [Job](job.md), named in a file a person wrote.

---

**Kind:** Concept.

**Built:** the model, the loader, the freeze at approval, Fleet firing a Command Trigger, and the wire to Bridge. Skills, `block`, `repair`, asking the owner and Bridge's screens are not.

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

A Command runs in the Job's worktree under the Check budget, with no shell. What it prints goes to the Job's log and is never read. Each firing is a row in `job_triggers`: Trigger, level, moment, step, state, exit code and times. The states are `skipped`, `running`, `passed`, `failed` and `awaiting_owner`.

> **Rule.** A failed Trigger changes neither the Job's status nor its step.
> Why: a Trigger is not a Check. `block` and `repair` are carried in the record and nothing acts on them yet.

The frozen set is `job_frozen_triggers`, one row per step a Trigger fires on. The repository's files are read from the base branch by `adapters::triggers_on_base`, and this machine's by `armada::Locator`.

## On the wire

Protocol 23.58, the four operations and one event, `docs/practices/protocol.md`.

| Operation | What it does |
|---|---|
| `list_triggers` | What a repository runs, each with its level and file, what it replaced, and the files left out with why |
| `get_trigger` | One Trigger as YAML text, from any level that holds a copy |
| `save_trigger`, `remove_trigger` | Write or delete a file in the repository's folder or this machine's |
| `job.trigger_changed` | One of a Job's Triggers moved. `JobDetail.triggers` is the rows, with the pending ones |

> **Rule.** A save is checked with the loader's rules before anything is written.
> Why: a Trigger that is saved is one that loads.

**A repository's save refuses a Command its `armada.yml` lacks, and a machine's does not.** The loader skips a Trigger like that so the same file is right in the next repository. A machine's applies to every repository, so refusing it for the one in front of a person would undo that. Its answer says it is skipped here.

> **Rule.** A repository's Trigger runs once it is on `main`.
> Why: Fleet reads a repository's files from the base branch. A save writes the checkout and answers `waits_for_main`, and a removal there waits the same way.

Nothing is held, so a save is on the next `list_triggers`. A Job's log line for a firing is stamped with the firing's own end, which is what `JobTrigger.log_at` points at.

The loader is `config::TriggerCatalogue`, the type is `core_model::Trigger`, and the decision is `.claude/decisions/2026-10-07-a-trigger-runs-at-a-moment-in-a-job.md`.

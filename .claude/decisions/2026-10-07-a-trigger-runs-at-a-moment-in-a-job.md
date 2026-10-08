# How does a person have something run at a moment in a Job?

**Decided 2026-10-07.**

He wants a formatter run when a step passes, a script run when a pull request opens, and a repair when one of them fails, without writing a workflow step for each. The first two pieces are the model and the loader, in `docs/concepts/trigger.md`.

**Chosen: a Trigger is a file that names a moment and what runs there.**

- **When:** `step_starts`, `step_passes` or `pr_opened`. `pr_opened` is the delivering step's entry, because `delivers` is read on entry.
- **Applies to:** every workflow, or one `workflow_id`, optionally narrowed to a step id.
- **Runs:** a Command named in the repository's `armada.yml`, which Fleet runs with no Drone, or a skill, which a Drone runs. Skills are modelled and not executed yet.
- **If it fails:** `block` and `repair`, both off unless written.
- **Three levels, the most specific winning:** Armada's, which ships none, the repository's `.armada/triggers/`, and this machine's. Identity is when, step and name, and the machine copy replaces the repository copy whole, with no field merge.
- **The repository's are read from `main` only**, never from the Job's branch, which is the tamper fence workflows have.
- **This machine's do not travel with Kit.** They are Machine, kept in `~/.armada/machine/triggers/`, a folder of its own so a sync of Kit's folders cannot pick them up.
- **A Command the repository does not declare skips the Trigger and marks it on the Job.** It is not refused, because the same file is right in the next repository.
- **A file that will not parse is left out with its reason**, and the others stand.
- **Triggers freeze onto the Job at approval**, as the workflow does. One saved later applies from the next Job.
- **A Trigger on a destructive Command asks the owner first.** The flag is modelled now and Fleet enforces it later.
- **A Trigger is not a Check.** Its exit code never advances or fails a gate.

**Later changes build on these four:**

- **Self repair.** The repair Drone finds the fix, then the owner chooses this branch, which is the Job's open pull request, or a new pull request. The Job's Overview canvas draws the repair as a branch off the workflow. Repair is bounded at 2 tries, then an alert.
- **Draft pull request.** A machine default and a repository default, which a workflow step can override.
- **Steps added to one Job** live in an additions record beside the frozen workflow and are never written into it.
- **Gaps in the delivery lane** mean before the pull request opens and after it opens. After merge is deferred.

**Block holds the Job, and where depends on the moment.** Chosen 7 Oct 2026 by the session, for the owner; a walk (`?walk=aTriggerHoldsTheJob`) confirms it.

| Moment | The hold stands | Job |
|---|---|---|
| `step_starts` | Before the step's Drone runs | `escalated`, reason `trigger_held` |
| `step_passes` | Before the next step starts. The last step's pass holds nothing: no step follows, and `pr_opened` holds the landing | `escalated`, reason `trigger_held` |
| `pr_opened` | Before the review gate can be answered. The pull request is out, so the approval and the merge are refused | Where it was. The gate is held for a person |

- **With `repair` also on, the hold waits through the repair.** A repair that ends passed lets it go by itself, and two failed tries leave it held. Jobs heal themselves first and the owner is the last resort.
- **Held is a state of the firing and not a status of the Job.** A status owes a verb, an icon, a column in the step machine and a row in the edge table, and what a hold adds over `escalated` is a reason, so the reason is new and the status is not. `awaiting_repair` stays what it was, a spent Check budget.
- **The owner's acts are Rerun and Skip.** Rerun runs the Command again with no Drone and lets it go if it passes. Skip lets it go and records the Trigger skipped by him. Kill is the act that already existed.
- **A hold let go before a step starts is not fired again when the Job gets there**, or a skip would be undone by the Trigger running a second time.
- **An added step that blocks holds the same way.** Its `repair` is acted on too, through the same repair as a Trigger's, and the hold waits through it.
- **A Board row carries the bell** for a hold, a repair fix waiting on his choice, and a Trigger that failed after its repair tries.

**Cost he took:** a machine Trigger narrowed to one workflow still replaces a repository Trigger of the same identity that applied to all of them, because identity leaves the workflow out. A pull-request Trigger names no step, since the delivering step is the one that fires it.

**Where it landed:** branch `triggers/model`, `core_model::Trigger` and `config::TriggerCatalogue`, with `crates/acceptance/tests/triggers.rs`. The wire and Bridge's saved Triggers, step cards and Job card are built on `triggers/wire` and `triggers/bridge`. Fleet freezes the set at approval and fires Command Triggers on a second branch, `triggers/fire`. The Draft defaults landed on `triggers/draft-default`: a machine preference, the repository's `pr_mode` and the delivering step's `draft_pr`, under the Job's own choice at approval. Steps added to one Job are on `triggers/added-steps`, protocol 23.68: `core_model::AddedStep`, `store::additions`, `fleet::added_steps`, and the repository's `pr_mode` edit through `edit_manifest`. A Script fires through the Trigger path. A Skill and a Drone step are recorded `skipped`. Bridge draws them on `triggers/added-steps-ui`: the `+` on the approval canvas, the Workflow tab's canvas and its stacked run, the added step's card and panel, and the repository's Draft pull requests switch. Self repair is on `triggers/repair`: a failed Trigger with `repair` on gets a repair Drone on a branch cut from the Job's, the Command runs again there, and the fix is held as `fix_ready` for the owner to place with `choose_trigger_fix`. Two tries, then `failed` and an alert on the Job. The wire for the new states and the act, and the repair branch on Bridge's canvas, are on `triggers/repair-bridge`. Block is on `triggers/block`, protocol 23.68: a failed Trigger or added step with `block` holds the Job, with the owner's `rerun_trigger` and `skip_trigger`, `JobSummary.alert` for a Board row's bell, and the hold on Bridge's canvas and list. Repair on an added step and the deletion of a repair branch once its fix is placed are on `triggers/added-step-repair`, protocol 23.72: a failed added Script with `repair` goes through the Trigger's repair, `choose_trigger_fix` names it as `addition`, and `AddedStep.repair_record` carries it. Skills and asking the owner are not built.

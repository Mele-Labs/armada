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

**Cost he took:** a machine Trigger narrowed to one workflow still replaces a repository Trigger of the same identity that applied to all of them, because identity leaves the workflow out. A pull-request Trigger names no step, since the delivering step is the one that fires it.

**Where it landed:** branch `triggers/model`, `core_model::Trigger` and `config::TriggerCatalogue`, with `crates/acceptance/tests/triggers.rs`. The wire, Bridge, repair, block, skills, asking the owner and the Draft defaults are not built. Fleet freezes the set at approval and fires Command Triggers on a second branch, `triggers/fire`.

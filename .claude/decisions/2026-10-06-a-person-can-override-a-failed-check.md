# Can a person override a failed Check?

**Decided 2026-10-06, built 2026-10-06.**

He said: "I should be able to override things like this, not just judge checks. If I know the job has the work completed and it should move on I should be able to move it forward." Asked how far it reaches, he chose any failed Check, `build` and `test` included.

**Chosen: `override_verdict` is offered and accepted at `awaiting_repair`, over a step stopped on `gate_failure` by a Check.**

- **It reverses a written position.** `docs/concepts/job.md` said a failed Check cannot be overruled and `build` failing is not a matter of opinion (#208). That sentence is gone from the page, the operation notes and the transition notes.
- **The case.** Job 12 sat at `awaiting_repair`, out of retries, failing only `diff_nonempty` because of a baseline bug (PR #1831). Run Checks again reused the failure and Restart step would have failed the same way.
- **Recorded as an override, never as a pass.** The step reads `advanced` with `failed(gate_failure)` still on it, the Check's failed run stays on the step, and the person is the actor.
- **The reason is required**, because the trigger is `gate_failure`. The body is `{reason}`; no closed `claim` exists on this act, since that set belongs to `file_report`.
- **The Job lands as an escalated one does.** `awaiting_repair -> queued` with a step left, and `awaiting_repair -> running` into completion where the step was the last.
- **The worktree must be on disk.** Everything else offered at `awaiting_repair` stays offered.

**Not built:** a rate of overridden Checks beside overridden Judge refusals. Both are `stopped -> advanced` on `gate_failure`; telling them apart needs the Check runs read beside the step.

**Where it landed:** `fleet/override-a-failed-check`. No wire change: `Recourse::OverrideVerdict` and `override_verdict` already existed.

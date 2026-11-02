# A Job whose settings hooks are skipped

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since mod `0.3.56`. `docs/concepts/session/how-a-job-gets-in.md`, *Where Claude Code skips the mod's hooks*.
**Additive only**: one word on an open-text field. No `SessionFact` variant, operation, event kind or error is new, and `packages/protocol` is unchanged.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionFact::Attached` (`report_session`), `attachment.kind` `hooks_skipped` | `target` `classic`; `detail.turns`: how many turns ended with no settings hook heard, two or more | The mod's `classic.*` hooks did not run for this Job. Told once a Job, and again from the mod's memory when a Fleet that was out of reach answers |

**Detected, never asked of the agent.** The mod counts main-thread turns that ended with no `classic.*` hook of its own run since it loaded, and tells at the second.
A turn that was interrupted or died does not count.
A Drone tells nothing.
**What it names is the reading, not the cause.** An organization's Claude Code can pass the hooks over; `disableAllHooks` has the same effect. Fleet does not tell them apart.
**Kept, drawn nowhere.** Fleet keeps the row like any attachment of a kind it has no meaning for, as `kind` is open text.
Bridge's `attachmentsOfRecord` leaves out a kind it has no screen for, so nothing is drawn.
The debug report and the Doctor read the row from the Job's record when they name it.

**Skew.** An older mod tells nothing, so a Job on one reads as having its hooks.

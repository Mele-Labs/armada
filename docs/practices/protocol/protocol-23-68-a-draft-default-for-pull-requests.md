# Protocol 23.68: a draft default for pull requests

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

the retired Landing concept, *What the landing rule carries*. **Additive only**: two optional fields and no operation.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_job` | `JobDetail.pr_mode_default?`: `ready` or `draft` | What a Job still at its approval gate opens as when its approval says nothing: the workflow's delivering step, the repository, this machine, then ready. Absent once `landing` is there, and from a Fleet before 23.68 |
| `get_preferences`, `save_preferences` | `Preferences.draft_pull_requests?`, and the name `draft_pull_requests` for a save | This machine's default, off until set. Absent is `false` |

**Bridge starts the draft choice on `pr_mode_default` where `landing` is absent.** A Job approved with no `landing.pr_mode` takes the same answer, so a Bridge that never learned the field still gets the default. **The default Fleet serves is for the workflow the Job was proposed on**: a person who picks another workflow in the proposal sees the first one's until the approval, and what is frozen is the picked workflow's. Bridge's half is in `packages/protocol/src/detail.ts (deleted)` and `preferences.ts`, written by hand like the rest.

# Merge pressed while the forge's checks run

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/fleet.md`, *A person presses to merge*. **Additive only**: one operation, one optional field on Bridge's `Outcome`, and a store column.

| Where | Carries | Notes |
| --- | --- | --- |
| `enable_job_auto_merge` (`POST /jobs/:job_id/auto_merge`) | no body | Answers the `PullRequestState` the forge shows afterwards. Records the ask on the Job; the Job stays at `awaiting_review`. 409 off the gate, once the checks have failed (`fleet.merge_checks_not_passed`), and with the forge's words where it refused. `Helm only` |
| the sweep | nothing on the wire | A Job at its gate whose pull request is found merged, and whose ask is recorded, is taken as `approve_review` takes it, actor `fleet`. Any other merge is recorded and left |

`Outcome.pullRequest` is Bridge's own and not on the wire: the `PullRequestState` main carries back from `enable_job_auto_merge`, which the gate draws as Auto-merge on.

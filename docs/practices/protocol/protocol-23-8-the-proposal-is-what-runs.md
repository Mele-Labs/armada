# Protocol 23.8: the proposal is what runs

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, slice 4 (#1765, carrying #1641, #1642, #1605, #1581 and #1699's route).

**A body, a route, a read and optional fields, all additive.** `approve_dispatch` takes
`ApproveDispatch`, the whole proposal as a person left it, and an empty body still approves it as
it stands, so a 23.7 Bridge approves exactly as before. `edit_job` saves `title?`, `facts?` and
`criteria?` without the press. `list_branches` reads the repository's branches through git.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/approve_dispatch` | `ApproveDispatch`, or nothing | `JobSummary`, `queued` | 409 `fleet.proposal_frozen` past `awaiting_approval`; 422 `fleet.no_such_branch`, 422 `fleet.unacceptable_proposal` |
| `POST /jobs/:job_id/edit`, `edit_job` | `EditJob` | `JobSummary`, still `awaiting_approval` | As above, and 422 on a body changing nothing |
| `GET /manifest/branches?manifest_id=`, `list_branches` | — | `Branches` | 422 on a repository not served |

**Read on the wire:** `Criterion.origin`, `{kind: issue, ref, url}`, `{kind: prompt}` or `{kind:
person}`, absent where nothing says; `Criterion.origin_moved_at`, on an issue's line once the
issue was edited after Fleet read it; and on `JobDetail`, `landing` (`target?`, `from_ref?`,
`pr_mode`), `drone_cap`, `policy_overrides` (`auto_merge?`, `review_gate?`, the words
`armada.yml` writes) and `approved_at`. `JobRequest.settings` carries the dispatch form's
`workflow_id?`, `tiers?`, `drone_cap?` and `lands?`. Each is absent where empty.

**The lock's bodies, and what slice 4 chose inside them.** `edit_job` takes #1641's criteria
rather than Bridge's lines, the spike's bodies table. An overridden step keeps deferring on the
record and the override is laid over the repository's word after the fold, so it wins however the
rule moves (answer 4) and #1683's record shows the word each gate read; `auto_merge` overridden
without a person reads `checks-pass`. Only `delivered` completes a Job and only one branch per Job
is cut, so `branching: group` and the other three `complete_when` values are refused rather than
kept unread; `land_together` does not cross. `from_ref` cuts the worktree and is what its work is
measured from; `target` is where the pull request opens. The Drone cap is kept, and slice 5
enforces it.

**Store V98** keeps a Job's landing, Drone cap, policy overrides and the issue its request linked.
The issue rotation is a second cursor on the pull-request rotation's interval (answer 5), over
every Job in flight whose request linked an issue.
